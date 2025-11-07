use crate::drive::{interop::GetPlacehodlerResult, mounts::Mount, utils::local_path_to_cr_uri};
use anyhow::{Context, Result};
use cloudreve_api::{
    api::explorer::ExplorerApiExt,
    models::{explorer::FileResponse, user::Token},
};
use std::path::PathBuf;
use tokio::sync::oneshot::Sender;
const PAGE_SIZE: i32 = 1000;

/// Messages sent from OS threads (SyncFilter callbacks) to the async processing task
///
/// # Safety    
/// This is safe because Windows CFAPI callbacks are designed to be invoked from arbitrary threads
/// and the data contained in Request, ticket, and info types are meant to be passed between threads
/// during the callback's lifetime.
#[derive(Debug)]
pub enum MountCommand {
    FetchPlaceholders {
        path: PathBuf,
        response: Sender<Result<GetPlacehodlerResult>>,
    },
    RefreshCredentials {
        credentials: Token,
    },
}

// SAFETY: Windows CFAPI is designed to allow callbacks from arbitrary threads.
// The Request, ticket, and info types contain data that is valid for the duration
// of the callback and can be safely transferred between threads.
unsafe impl Send for MountCommand {}

impl Mount {
    pub async fn fetch_placeholders(&self, path: PathBuf) -> Result<GetPlacehodlerResult> {
        let config = self.config.read().await;
        let remote_base = config.remote_path.clone();
        let sync_path = config.sync_path.clone();
        drop(config);

        let uri = local_path_to_cr_uri(path.clone(), sync_path, remote_base)
            .context("failed to convert local path to cloudreve uri")?;
        let mut placehodlers: Vec<FileResponse> = Vec::new();

        let mut previous_response = None;
        loop {
            let response = self
                .cr_client
                .list_files_all(previous_response.as_ref(), &uri.to_string(), PAGE_SIZE)
                .await?;

            for file in &response.res.files {
                tracing::debug!(target: "drive::mounts", file = %file.name, "Server file");
            }

            placehodlers.extend(response.res.files.clone());
            let has_more: bool = response.more;
            previous_response = Some(response);

            if !has_more {
                break;
            }
        }

        tracing::debug!(target: "drive::mounts", uri = %uri.to_string(), "Fetch file list from cloudreve");

        Ok(GetPlacehodlerResult {
            files: placehodlers,
            local_path: path.clone(),
            remote_path: uri.clone(),
        })
    }
}
