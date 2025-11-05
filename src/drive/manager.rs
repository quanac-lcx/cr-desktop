use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::mounts::{DriveConfig, Mount};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveState {
    pub drives: HashMap<String, DriveConfig>,
}

impl Default for DriveState {
    fn default() -> Self {
        Self {
            drives: HashMap::new(),
        }
    }
}

pub struct DriveManager {
    drives: Arc<RwLock<HashMap<String, Mount>>>,
    config_dir: PathBuf,
}

impl DriveManager {
    /// Create a new DriveManager instance
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;

        // Ensure config directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create .cloudreve config directory")?;
        }

        Ok(Self {
            config_dir,
            drives: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get the .cloudreve config directory path
    fn get_config_dir() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Failed to get user home directory")?;
        Ok(home_dir.join(".cloudreve"))
    }

    /// Get the config file path
    fn get_config_file(&self) -> PathBuf {
        self.config_dir.join("drives.json")
    }

    /// Load drive configurations from disk
    pub async fn load(&self) -> Result<()> {
        let config_file = self.get_config_file();

        if !config_file.exists() {
            tracing::info!(target: "drive", "No existing drive config found, starting fresh");
            return Ok(());
        }

        tracing::debug!(target: "drive", path = %config_file.display(), "Loading drive configurations");

        let content =
            fs::read_to_string(&config_file).context("Failed to read drive config file")?;

        let state: DriveState =
            serde_json::from_str(&content).context("Failed to parse drive config")?;

        // Add drives to manager
        let mut count = 0;
        for (id, config) in state.drives.iter() {
            self.add_drive(config.clone())
                .await
                .context(format!("Failed to add drive: {}", id))?;
            count += 1;
        }

        tracing::info!(target: "drive", count = count, "Loaded drive(s) from config");

        Ok(())
    }

    /// Persist drive configurations to disk
    pub async fn persist(&self) -> Result<()> {
        let config_file = self.get_config_file();
        let mut write_guard = self.drives.write().await;

        tracing::debug!(target: "drive", path = %config_file.display(), count = write_guard.len(), "Persisting drive configurations");

        let mut newState = DriveState::default();

        // Update drive states from underlying mounts
        for (id, mount) in write_guard.iter() {
            let config = mount.get_config();
            newState.drives.insert(id.clone(), config);
        }

        let content =
            serde_json::to_string_pretty(&newState).context("Failed to serialize drive state")?;
        fs::write(&config_file, content).context("Failed to write drive config file")?;

        tracing::info!(target: "drive", count = newState.drives.len(), "Persisted drive(s) to config");

        Ok(())
    }

    /// Add a new drive
    pub async fn add_drive(&self, mut config: DriveConfig) -> Result<String> {
        // Fetch favicon if icon_path is not set or doesn't exist
        if config.icon_path.is_none()
            || !config
                .icon_path
                .as_ref()
                .map(|p| std::path::Path::new(p).exists())
                .unwrap_or(false)
        {
            match super::favicon::fetch_and_save_favicon(&config.instance_url).await {
                Ok(path) => {
                    tracing::info!(target: "drive", icon_path = %path, "Favicon fetched successfully");
                    config.icon_path = Some(path);
                }
                Err(e) => {
                    tracing::warn!(target: "drive", error = %e, "Failed to fetch favicon, continuing without icon");
                }
            }
        }

        let mut write_guard = self.drives.write().await;
        let mount = Mount::new(config.clone());
        if let Err(e) = mount.start().await {
            tracing::error!(target: "drive", error = %e, "Failed to start drive");
            return Err(e).context("Failed to start drive");
        }

        let id = mount.id();
        write_guard.insert(id.clone(), mount);
        Ok(id)
    }

    /// Remove a drive by ID
    pub async fn remove_drive(&self, id: &str) -> Result<Option<DriveConfig>> {
        let mut write_guard = self.drives.write().await;
        Ok(write_guard.remove(id).map(|mount| mount.get_config()))
    }

    /// Get a drive by ID
    pub async fn get_drive(&self, id: &str) -> Option<DriveConfig> {
        let read_guard = self.drives.read().await;
        read_guard.get(id).map(|mount| mount.get_config())
    }

    /// List all drives
    pub async fn list_drives(&self) -> Vec<DriveConfig> {
        let read_guard = self.drives.read().await;
        read_guard
            .values()
            .map(|mount| mount.get_config())
            .collect()
    }

    /// Update drive configuration
    pub async fn update_drive(&self, id: &str, config: DriveConfig) -> Result<()> {
        let mut write_guard = self.drives.write().await;
        if write_guard.contains_key(id) {
            write_guard.insert(id.to_string(), Mount::new(config.clone()));
            Ok(())
        } else {
            anyhow::bail!("Drive not found: {}", id)
        }
    }

    /// Enable/disable a drive
    pub async fn set_drive_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let mut write_guard = self.drives.write().await;
        if let Some(drive) = write_guard.get_mut(id) {
            drive.get_config().enabled = enabled;
            Ok(())
        } else {
            anyhow::bail!("Drive not found: {}", id)
        }
    }

    /// Placeholder: Start syncing a drive
    pub async fn start_sync(&self, id: &str) -> Result<()> {
        // TODO: Implement actual sync logic
        tracing::info!(target: "drive::sync", drive_id = %id, "Starting sync for drive");
        Ok(())
    }

    /// Placeholder: Stop syncing a drive
    pub async fn stop_sync(&self, id: &str) -> Result<()> {
        // TODO: Implement actual sync logic
        tracing::info!(target: "drive::sync", drive_id = %id, "Stopping sync for drive");
        Ok(())
    }

    /// Placeholder: Get sync status for a drive
    pub async fn get_sync_status(&self, id: &str) -> Result<serde_json::Value> {
        // TODO: Implement actual status retrieval
        tracing::debug!(target: "drive::sync", drive_id = %id, "Getting sync status");
        Ok(serde_json::json!({
            "drive_id": id,
            "status": "idle",
            "last_sync": null,
            "files_synced": 0,
        }))
    }

    pub async fn shutdown(&self) {
        let mut write_guard = self.drives.write().await;
        for (id, mount) in write_guard.iter() {
            mount.shutdown().await;
        }
        tracing::info!(target: "drive", "All drives shutdown");
    }
}
