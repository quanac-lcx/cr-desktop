use crate::cfapi::{
    error::{CResult, CloudErrorKind},
    filter::{Filter, Request, SyncFilter, info, ticket},
    root::{
        Connection, HydrationType, PopulationType, SecurityId, Session, SyncRootId, SyncRootIdBuilder, SyncRootInfo
    },
};
use ::serde::{Deserialize, Serialize};
use anyhow::{Context, Result};
use tokio::runtime::{Handle, Runtime};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc, time::Duration,
};
use url::Url;
use windows::Storage::Provider::StorageProviderSyncRootManager;

use crate::tasks::{TaskManager, TaskManagerConfig};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DriveConfig {
    pub id: Option<String>,
    pub name: String,
    pub instance_url: String,
    pub remote_path: String,
    pub credentials: Credentials,
    pub sync_path: PathBuf,
    pub icon_path: Option<String>,
    pub enabled: bool,
    pub user_id: String,

    // Windows CFAPI
    pub sync_root_id: Option<SyncRootId>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Credentials {
    pub access_token: Option<String>,
    pub refresh_token: String,
    pub expires_at: i64,
}

#[derive(Clone)]
pub struct Mount {
    queue: Arc<TaskManager>,
    config: DriveConfig,
    connection: Option<Connection<CallbackHandler>>,
}

impl Mount {
    pub fn new(config: DriveConfig) -> Self {
        let task_config = TaskManagerConfig {
            max_workers: 4,
            completed_buffer_size: 100,
        };
        let task_manager = TaskManager::new(task_config);
        Self {
            config,
            queue: task_manager,
            connection: None,
        }
    }

    pub fn get_config(&self) -> DriveConfig {
        self.config.clone()
    }

    pub async fn start(&mut self) -> Result<()> {
        if !StorageProviderSyncRootManager::IsSupported()
            .context("Cloud Filter API is not supported")?
        {
            return Err(anyhow::anyhow!("Cloud Filter API is not supported"));
        }

        // if sync root id is not set, generate one
        if self.config.sync_root_id.is_none() {
            self.config.sync_root_id = Some(
                generate_sync_root_id(
                    &self.config.instance_url,
                    &self.config.name,
                    &self.config.user_id,
                )
                .context("failed to generate sync root id")?,
            );
        }
        let config = &self.config;
        let sync_root_id = config.sync_root_id.as_ref().unwrap();

        // Register sync root if not registered
        if !sync_root_id.is_registered()? {
            tracing::info!(target: "drive::mounts", id = %self.id(), "Registering sync root");
            let mut sync_root_info = SyncRootInfo::default();
            sync_root_info.set_display_name(config.name.clone());
            sync_root_info.set_hydration_type(HydrationType::Progressive);
            sync_root_info.set_population_type(PopulationType::Full);
            if let Some(icon_path) = config.icon_path.as_ref() {
                sync_root_info.set_icon(format!("{},0", icon_path));
            }
            sync_root_info.set_version("1.0.0");
            sync_root_info
                .set_recycle_bin_uri("http://cloudmirror.example.com/recyclebin")
                .context("failed to set recycle bin uri")?;
            sync_root_info
                .set_path(Path::new(&config.sync_path))
                .context("failed to set sync root path")?;
            sync_root_id
                .register(sync_root_info)
                .context("failed to register sync root")?;
        }

        tracing::info!(target: "drive::mounts", id = %self.id(), "Connecting to sync root");
        let connection = Session::new()
            .connect(&config.sync_path, CallbackHandler::new(config.clone()))
            .context("failed to connect to sync root")?;

        self.connection = Some(connection);

        Ok(())
    }

    pub fn id(&self) -> String {
        self.config
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    pub async fn shutdown(&self) {
        tracing::info!(target: "drive::mounts", id=self.id(), "Shutting down Mount");
        if let Some(ref connection) = self.connection {
            connection.disconnect();
        }
        if let Some(sync_root_id) = self.config.sync_root_id.as_ref() {
            if let Err(e) = sync_root_id.unregister() {
                tracing::warn!(target: "drive::mounts", id=self.id(), error=%e, "Failed to unregister sync root");
            }
        }
        self.queue.shutdown().await;
    }
}

fn generate_sync_root_id(
    instance_url: &str,
    account_name: &str,
    user_id: &str,
) -> Result<SyncRootId> {
    // Parse the instance URL to get the hostname
    let url = Url::parse(instance_url)?;
    let hostname = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host found"))?;

    // Generate a SHA-256 hash of the hostname
    let mut hasher = Sha256::new();
    hasher.update(hostname.as_bytes());
    let hash_result = hasher.finalize();

    // Convert hash to hex string and truncate to reasonable length
    // Use first 16 characters (64 bits) of the hash for the provider name
    let hash_hex = format!("{:x}", hash_result);
    let provider_name = format!("cloudreve{}", &hash_hex[..16]);

    // Build the sync root ID
    let sync_root_id = SyncRootIdBuilder::new(provider_name)
        .user_security_id(SecurityId::current_user()?)
        .account_name(user_id)
        .build();

    Ok(sync_root_id)
}

#[derive(Clone)]
pub struct CallbackHandler{
    config: DriveConfig,
}

impl CallbackHandler {
    pub fn new(config: DriveConfig) -> Self {
        Self { config }
    }

    pub fn id(&self) -> String {
        self.config.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    pub async fn sleep(&self) {
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

impl SyncFilter for CallbackHandler {
     fn fetch_data(
        &self,
        _request: crate::cfapi::filter::Request,
        _ticket: crate::cfapi::filter::ticket::FetchData,
        _info: crate::cfapi::filter::info::FetchData,
    ) -> crate::cfapi::error::CResult<()> {
        todo!()
    }

     fn deleted(&self, request: Request, _info: info::Deleted) {
        tracing::debug!(target: "drive::mounts", id = %self.id(), path = %request.path().display(), "Deleted");
    }

     fn delete(
        &self,
        request: Request,
        ticket: ticket::Delete,
        info: info::Delete,
    ) -> CResult<()> {
        tracing::debug!(target: "drive::mounts", id = %self.id(), path = %request.path().display(), "Delete");
        ticket.pass().unwrap();
        Ok(())
    }

     fn rename(
        &self,
        request: Request,
        ticket: ticket::Rename,
        info: info::Rename,
    ) -> CResult<()> {
        let src = request.path();
        let dest = info.target_path();
        tracing::debug!(target: "drive::mounts", id = %self.id(), source_path = %src.display(), target_path = %dest.display(), "Rename");
        Err(CloudErrorKind::NotSupported)
    }

     fn fetch_placeholders(
        &self,
        request: Request,
        ticket: ticket::FetchPlaceholders,
        info: info::FetchPlaceholders,
    ) -> CResult<()> {
        tracing::debug!(target: "drive::mounts", id = %self.id(), path = %request.path().display(), "FetchPlaceholders");
        tracing::debug!(target: "drive::mounts", id = %self.id(), path = %request.path().display(), "FetchPlaceholders after sleep");

        ticket.pass_with_placeholder(&mut []).unwrap();

        Ok(())
    }

     fn closed(&self, request: Request, info: info::Closed) {
        tracing::debug!(target: "drive::mounts", id = %self.id(), path = %request.path().display(), deleted = %info.deleted(), "Closed");
    }

     fn cancel_fetch_data(&self, _request: Request, _info: info::CancelFetchData) {
        tracing::debug!(target: "drive::mounts", id = %self.id(), "CancelFetchData");
    }

     fn validate_data(
        &self,
        _request: Request,
        _ticket: ticket::ValidateData,
        _info: info::ValidateData,
    ) -> CResult<()> {
        tracing::debug!(target: "drive::mounts", id = %self.id(), "ValidateData");
        Err(CloudErrorKind::NotSupported)
    }

     fn cancel_fetch_placeholders(
        &self,
        _request: Request,
        _info: info::CancelFetchPlaceholders,
    ) {
        tracing::debug!(target: "drive::mounts", id = %self.id(), "CancelFetchPlaceholders");
    }

     fn opened(&self, request: Request, _info: info::Opened) {
        tracing::debug!(target: "drive::mounts", id = %self.id(), path = %request.path().display(), "Opened");
    }

     fn dehydrate(
        &self,
        _request: Request,
        _ticket: ticket::Dehydrate,
        info: info::Dehydrate,
    ) -> CResult<()> {
        tracing::debug!(
            target: "drive::mounts",
            id = %self.id(),
            reason = ?info.reason(),
            "Dehydrate"
        );
        Err(CloudErrorKind::NotSupported)
    }

     fn dehydrated(&self, _request: Request, info: info::Dehydrated) {
        tracing::debug!(
            target: "drive::mounts",
            id = %self.id(),
            reason = ?info.reason(),
            "Dehydrated"
        );
    }

     fn renamed(&self, _request: Request, info: info::Renamed) {
        let dest = info.source_path();
        tracing::debug!(target: "drive::mounts", id = %self.id(), dest_path = %dest.display(), "Renamed");
    }
}
