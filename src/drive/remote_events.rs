use crate::{
    cfapi::placeholder::LocalFileInfo,
    drive::{commands::MountCommand, mounts::Mount, sync::SyncMode},
};
use anyhow::{Context, Result};
use cloudreve_api::{
    api::explorer::FileEventsApi,
    models::explorer::{FileEvent, FileEventData, FileEventType},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_SECS: u64 = 1;
const MAX_BACKOFF_SECS: u64 = 32;
const LONG_RETRY_DELAY_SECS: u64 = 3600; // 1 hour

struct BackoffState {
    retry_count: u32,
    current_delay: Duration,
}

impl BackoffState {
    fn new() -> Self {
        Self {
            retry_count: 0,
            current_delay: Duration::from_secs(INITIAL_BACKOFF_SECS),
        }
    }

    fn reset(&mut self) {
        self.retry_count = 0;
        self.current_delay = Duration::from_secs(INITIAL_BACKOFF_SECS);
    }

    fn next_delay(&mut self) -> Option<Duration> {
        if self.retry_count >= MAX_RETRIES {
            return None;
        }
        let delay = self.current_delay;
        self.retry_count += 1;
        self.current_delay =
            Duration::from_secs((self.current_delay.as_secs() * 2).min(MAX_BACKOFF_SECS));
        Some(delay)
    }
}

enum ListenResult {
    Error(anyhow::Error),
    ReconnectRequired,
    StreamEnded,
}

impl Mount {
    pub async fn process_remote_events(s: Arc<Self>) {
        tracing::info!(target: "drive::remote_events", "Listening to remote events");
        let mut backoff = BackoffState::new();

        let sync_path = {
            let config = s.config.read().await;
            config.sync_path.clone()
        };

        loop {
            let result = s.listen_remote_events().await;
            match result {
                ListenResult::ReconnectRequired => {
                    tracing::info!(target: "drive::remote_events", "Reconnect required, re-subscribing immediately");
                    backoff.reset();
                    continue;
                }
                ListenResult::StreamEnded => {
                    tracing::warn!(target: "drive::remote_events", "Event stream ended unexpectedly, reconnecting");
                    backoff.reset();
                    continue;
                }
                ListenResult::Error(e) => {
                    if let Some(delay) = backoff.next_delay() {
                        tracing::error!(
                            target: "drive::remote_events",
                            error = %e,
                            retry_count = backoff.retry_count,
                            delay_secs = delay.as_secs(),
                            "Failed to listen to remote events, retrying"
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        tracing::error!(
                            target: "drive::remote_events",
                            error = %e,
                            "Max retries reached, waiting 1 hour before retrying. Triggerring full sync..."
                        );
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        let _ = s.command_tx.send(MountCommand::Sync {
                            local_paths: vec![sync_path.clone()],
                            mode: SyncMode::FullHierarchy,
                        });
                        tokio::time::sleep(Duration::from_secs(LONG_RETRY_DELAY_SECS)).await;
                        backoff.reset();
                    }
                }
            }
        }
    }

    async fn listen_remote_events(&self) -> ListenResult {
        let (remote_base, sync_path) = {
            let config = self.config.read().await;
            (config.remote_path.clone(), config.sync_path.clone())
        };

        let mut subscription = match self.cr_client.subscribe_file_events(&remote_base).await {
            Ok(sub) => sub,
            Err(e) => return ListenResult::Error(e.into()),
        };

        loop {
            match subscription.next_event().await {
                Ok(Some(event)) => match event {
                    FileEvent::Event(events) => {
                        tracing::trace!(target: "drive::remote_events", events = ?events, "Handling file events batch");
                        if let Err(e) = self.handle_file_events(sync_path.clone(), events).await {
                            tracing::error!(target: "drive::remote_events", error = ?e, "Failed to handle file events");
                        }
                    }
                    FileEvent::Resumed => {
                        tracing::debug!(target: "drive::remote_events", "Subscription resumed");
                    }
                    FileEvent::Subscribed => {
                        tracing::info!(target: "drive::remote_events", "New subscribtion, triggger full sync...");
                        let _ = self.command_tx.send(MountCommand::Sync {
                            local_paths: vec![sync_path.clone()],
                            mode: SyncMode::FullHierarchy,
                        });
                    }
                    FileEvent::KeepAlive => {
                        tracing::trace!(target: "drive::remote_events", "Keep-alive");
                    }
                    FileEvent::ReconnectRequired => {
                        tracing::debug!(target: "drive::remote_events", "Reconnect required");
                        return ListenResult::ReconnectRequired;
                    }
                },
                Ok(None) => {
                    return ListenResult::StreamEnded;
                }
                Err(e) => {
                    return ListenResult::Error(e.into());
                }
            }
        }
    }

    async fn handle_file_events(
        &self,
        sync_root: PathBuf,
        events: Vec<FileEventData>,
    ) -> Result<()> {
        // Group events by type
        let mut create_events: Vec<FileEventData> = Vec::new();
        let mut modify_events: Vec<FileEventData> = Vec::new();
        let mut rename_events: Vec<FileEventData> = Vec::new();
        let mut delete_events: Vec<FileEventData> = Vec::new();

        for event in events {
            match event.event_type {
                FileEventType::Create => create_events.push(event),
                FileEventType::Modify => modify_events.push(event),
                FileEventType::Rename => rename_events.push(event),
                FileEventType::Delete => delete_events.push(event),
            }
        }

        // Handle Create events grouped by parent
        if !create_events.is_empty() {
            self.handle_create_events(sync_root.clone(), create_events)
                .await?;
        }

        // Handle other event types (currently no-op, but structured for future expansion)
        // Modify, Rename, Delete events can be handled here when needed

        Ok(())
    }

    async fn handle_create_events(
        &self,
        sync_root: PathBuf,
        events: Vec<FileEventData>,
    ) -> Result<()> {
        // Group create events by parent of `from` path
        let mut grouped_by_parent: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        for event in events {
            // Remote paths use Unix-style separators, convert to OS-native path
            let relative_path: PathBuf = event.from.trim_start_matches('/').split('/').collect();
            let local_from_path = sync_root.join(&relative_path);

            if let Some(parent) = local_from_path.parent() {
                grouped_by_parent
                    .entry(parent.to_path_buf())
                    .or_default()
                    .push(local_from_path);
            }
        }

        // Process each group
        for (parent, paths) in grouped_by_parent {
            if let Err(e) = self
                .sync_last_presented_parent(sync_root.clone(), parent, paths)
                .await
            {
                tracing::error!(
                    target: "drive::remote_events",
                    error = ?e,
                    "Failed to sync parent for create events"
                );
            }
        }

        Ok(())
    }

    async fn sync_last_presented_parent(
        &self,
        sync_root: PathBuf,
        initial_parent: PathBuf,
        local_paths: Vec<PathBuf>,
    ) -> Result<()> {
        // Walk up from initial_parent to find the first existing & populated parent
        let mut current_path = initial_parent.clone();
        let mut child_of_existing: Option<PathBuf> = None;

        loop {
            // Check if we've gone above the sync root
            if !current_path.starts_with(&sync_root)
                || current_path == sync_root.parent().unwrap_or(Path::new(""))
            {
                tracing::warn!(
                    target: "drive::remote_events",
                    sync_root = %sync_root.display(),
                    initial_parent = %initial_parent.display(),
                    "File event parent is not under sync root, skipping"
                );
                return Ok(());
            }

            let path_info = LocalFileInfo::from_path(&current_path)
                .context("failed to get path file info")?;

            if path_info.exists {
                if !path_info.is_placeholder() || path_info.is_folder_populated() {
                    // Found an existing & populated parent, sync from here
                    let (mode, sync_paths) = if let Some(child_path) = child_of_existing {
                        // We walked up, so sync the intermediate child folder
                        tracing::trace!(
                            target: "drive::remote_events",
                            existing_parent = %current_path.display(),
                            child_path = %child_path.display(),
                            "Syncing intermediate child path with PathOnly"
                        );
                        (SyncMode::PathOnly, vec![child_path])
                    } else if local_paths.len() > 1 {
                        // Multiple paths in same parent - sync parent with first layer
                        tracing::trace!(
                            target: "drive::remote_events",
                            parent_path = %current_path.display(),
                            path_count = local_paths.len(),
                            "Syncing parent path with PathAndFirstLayer for multiple new events"
                        );
                        (SyncMode::PathAndFirstLayer, vec![current_path.clone()])
                    } else {
                        // Single path - sync only that path
                        tracing::trace!(
                            target: "drive::remote_events",
                            parent_path = %current_path.display(),
                            "Syncing single path for new event"
                        );
                        (SyncMode::PathOnly, local_paths.clone())
                    };

                    self.command_tx
                        .send(MountCommand::Sync {
                            local_paths: sync_paths,
                            mode,
                        })
                        .context("failed to send sync command")?;
                    return Ok(());
                } else {
                    tracing::trace!(
                        target: "drive::remote_events",
                        parent_path = %current_path.display(),
                        "Parent path is a placeholder and not populated, skipping"
                    );
                    return Ok(());
                }
            }

            // Current path doesn't exist, walk up to its parent
            // Remember current_path as the child that will need syncing
            child_of_existing = Some(current_path.clone());

            match current_path.parent() {
                Some(parent) => {
                    current_path = parent.to_path_buf();
                }
                None => {
                    tracing::warn!(
                        target: "drive::remote_events",
                        sync_root = %sync_root.display(),
                        "Reached filesystem root without finding existing parent"
                    );
                    return Ok(());
                }
            }
        }
    }
}
