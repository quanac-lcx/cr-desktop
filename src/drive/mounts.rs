use ::serde::{Deserialize, Serialize};
use anyhow::Result;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use uuid::serde;

use crate::tasks::{TaskManager, TaskManagerConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveConfig {
    pub id: Option<String>,
    pub name: String,
    pub instance_url: String,
    pub remote_path: String,
    pub credentials: Credentials,
    pub sync_path: PathBuf,
    pub icon_path: Option<String>,
    pub enabled: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub async fn start(&self) -> Result<()> {
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
        self.queue.shutdown().await;
    }
}
