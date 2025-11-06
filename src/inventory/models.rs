use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a file metadata entry in the inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: i64,
    pub drive_id: Uuid,
    pub local_path: String,
    pub remote_uri: String,
    pub created_at: i64, // Unix timestamp
    pub updated_at: i64, // Unix timestamp
    pub etag: String,
    pub metadata: HashMap<String, String>,
    pub props: Option<serde_json::Value>,
}

/// Entry for inserting or updating file metadata
#[derive(Debug, Clone)]
pub struct MetadataEntry {
    pub drive_id: Uuid,
    pub local_path: String,
    pub remote_uri: String,
    pub etag: String,
    pub metadata: HashMap<String, String>,
    pub props: Option<serde_json::Value>,
}

impl MetadataEntry {
    pub fn new(
        drive_id: Uuid,
        local_path: impl Into<String>,
        remote_uri: impl Into<String>,
    ) -> Self {
        Self {
            drive_id,
            local_path: local_path.into(),
            remote_uri: remote_uri.into(),
            etag: String::new(),
            metadata: HashMap::new(),
            props: None,
        }
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

