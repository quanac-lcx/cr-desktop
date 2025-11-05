use crate::cfapi::root::{SecurityId, SyncRootId, SyncRootIdBuilder};
use ::serde::{Deserialize, Serialize};
use anyhow::Result;
use sha2::{Sha256, Digest};
use url::Url;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

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

pub struct Mount {
    queue: Arc<TaskManager>,
    config: DriveConfig,
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
        }
    }

    pub fn get_config(&self) -> DriveConfig {
        self.config.clone()
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.config.sync_root_id.is_none() {
            self.config.sync_root_id = Some(self.generate_sync_root_id()?);
        }
        Ok(())
    }

    /// Generate a sync root ID for this mount based on the instance URL and account name
    fn generate_sync_root_id(&self) -> Result<SyncRootId> {
        // Parse the instance URL to get the hostname
        let url = Url::parse(&self.config.instance_url)?;
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
            .account_name(&self.config.user_id)
            .build();

        Ok(sync_root_id)
    }

    pub fn id(&self) -> String {
        self.config
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    pub async fn shutdown(&self) {
        tracing::info!(target: "drive::mounts", id=self.id(), "Shutting down Mount");
        self.queue.shutdown().await;
    }
}
