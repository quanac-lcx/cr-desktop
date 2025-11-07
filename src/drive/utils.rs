use std::path::PathBuf;

use anyhow::{Context, Result};
use cloudreve_api::models::uri::CrUri;
use url::Url;

use crate::{drive::mounts::DriveConfig, inventory::FileMetadata};

pub fn local_path_to_cr_uri(path: PathBuf, root: PathBuf, remote_base: String) -> Result<CrUri> {
    let mut base = CrUri::new(&remote_base)?;

    // Strip the root from path to get the relative path
    let relative = path.strip_prefix(&root).context("Path is not under root")?;

    // Convert to string with forward slashes (for URI compatibility)
    let relative_str = relative
        .to_str()
        .context("Path contains invalid UTF-8")?
        .replace("\\", "/");

    // Join the relative path to the base URI if not empty
    if !relative_str.is_empty() {
        base.join_raw(&relative_str);
    }

    Ok(base)
}

pub fn remote_path_to_local_relative_path(
    remote_path: &CrUri,
    remote_base: &CrUri,
) -> Result<PathBuf> {
    let remote_path_str = remote_path.path().clone();
    let remote_base_str = remote_base.path().clone();

    // 1. add ending slash if not presented to remote_base_str
    let remote_base_str = if !remote_base_str.ends_with('/') {
        remote_base_str + "/"
    } else {
        remote_base_str
    };

    // 2. remove remote_base_str from remote_path_str
    let relative_path = remote_path_str
        .strip_prefix(&remote_base_str)
        .context("Path is not under remote base")?;

    Ok(PathBuf::from(relative_path))
}

pub fn view_folder_online_url(remote_path: &str, config: &DriveConfig) -> Result<String> {
    // parse
    let mut base = config.instance_url.parse::<Url>()?;
    base.set_path("/home");

    // set query `path` to remote_path,
    {
        let mut query = base.query_pairs_mut();
        query.append_pair("path", remote_path);
    }
    Ok(base.to_string())
}

pub fn view_file_online_url(file_meta: &FileMetadata, config: &DriveConfig) -> Result<String> {
    let mut base = config.instance_url.parse::<Url>()?;
    base.set_path("/home");
    // set query `path` to remote_path,
    {
        let mut query = base.query_pairs_mut();
        query
            .append_pair(
                "path",
                CrUri::new(&file_meta.remote_uri)?
                    .parent()?
                    .to_string()
                    .as_str(),
            )
            .append_pair("open", file_meta.remote_uri.as_str());
    }
    Ok(base.to_string())
}
