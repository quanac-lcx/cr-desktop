use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a file metadata entry in the inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: i64,
    pub drive_id: Uuid,
    pub is_folder: bool,
    pub local_path: String,
    pub remote_uri: String,
    pub created_at: i64, // Unix timestamp
    pub updated_at: i64, // Unix timestamp
    pub etag: String,
    pub metadata: HashMap<String, String>,
    pub props: Option<serde_json::Value>,
    pub permissions: String,
    pub shared: bool,
    pub size: i64,
}

/// Entry for inserting or updating file metadata
#[derive(Debug, Clone)]
pub struct MetadataEntry {
    pub drive_id: Uuid,
    pub is_folder: bool,
    pub created_at: i64, // Unix timestamp
    pub updated_at: i64, // Unix timestamp
    pub local_path: String,
    pub remote_uri: String,
    pub etag: String,
    pub permissions: String,
    pub shared: bool,
    pub size: i64,
    pub metadata: HashMap<String, String>,
    pub props: Option<serde_json::Value>,
}

impl MetadataEntry {
    pub fn new(
        drive_id: Uuid,
        local_path: impl Into<String>,
        remote_uri: impl Into<String>,
        is_folder: bool,
    ) -> Self {
        Self {
            drive_id,
            is_folder,
            local_path: local_path.into(),
            remote_uri: remote_uri.into(),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            etag: String::new(),
            metadata: HashMap::new(),
            props: None,
            permissions: String::new(),
            shared: false,
            size: 0,
        }
    }

    pub fn with_permissions(mut self, permissions: impl Into<String>) -> Self {
        self.permissions = permissions.into();
        self
    }

    pub fn with_shared(mut self, shared: bool) -> Self {
        self.shared = shared;
        self
    }

    pub fn with_size(mut self, size: i64) -> Self {
        self.size = size;
        self
    }

    pub fn with_created_at(mut self, created_at: i64) -> Self {
        self.created_at = created_at;
        self
    }

    pub fn with_updated_at(mut self, updated_at: i64) -> Self {
        self.updated_at = updated_at;
        self
    }

    pub fn with_etag(mut self, etag: impl Into<String>) -> Self {
        self.etag = etag.into();
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_props(mut self, props: serde_json::Value) -> Self {
        self.props = Some(props);
        self
    }
}

impl From<&FileMetadata> for MetadataEntry {
    fn from(file_metadata: &FileMetadata) -> Self {
        Self {
            drive_id: file_metadata.drive_id.clone(),
            is_folder: file_metadata.is_folder,
            created_at: file_metadata.created_at,
            updated_at: file_metadata.updated_at.clone(),
            local_path: file_metadata.local_path.clone(),
            remote_uri: file_metadata.remote_uri.clone(),
            etag: file_metadata.etag.clone(),
            permissions: file_metadata.permissions.clone(),
            shared: file_metadata.shared,
            metadata: file_metadata.metadata.clone(),
            props: file_metadata.props.clone(),
            size: file_metadata.size,
        }
    }
}
