use crate::AppStateHandle;
use cloudreve_sync::DriveConfig;
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri::{
    utils::{config::WindowEffectsConfig, WindowEffect},
    webview::WebviewWindowBuilder,
    AppHandle, Manager, State, WebviewUrl,
};
use tauri_plugin_frame::WebviewWindowExt;

/// Result type for Tauri commands
type CommandResult<T> = Result<T, String>;

/// List all configured drives
#[tauri::command]
pub async fn list_drives(state: State<'_, AppStateHandle>) -> CommandResult<Vec<DriveConfig>> {
    let app_state = state
        .get()
        .ok_or_else(|| "App not yet initialized".to_string())?;
    Ok(app_state.drive_manager.list_drives().await)
}

/// Add a new drive configuration
#[tauri::command]
pub async fn add_drive(
    state: State<'_, AppStateHandle>,
    config: DriveConfig,
) -> CommandResult<String> {
    let app_state = state
        .get()
        .ok_or_else(|| "App not yet initialized".to_string())?;
    app_state
        .drive_manager
        .add_drive(config)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a drive by ID
#[tauri::command]
pub async fn remove_drive(
    state: State<'_, AppStateHandle>,
    drive_id: String,
) -> CommandResult<Option<DriveConfig>> {
    let app_state = state
        .get()
        .ok_or_else(|| "App not yet initialized".to_string())?;
    app_state
        .drive_manager
        .remove_drive(&drive_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get sync status for a drive
#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, AppStateHandle>,
    drive_id: String,
) -> CommandResult<serde_json::Value> {
    let app_state = state
        .get()
        .ok_or_else(|| "App not yet initialized".to_string())?;
    app_state
        .drive_manager
        .get_sync_status(&drive_id)
        .await
        .map_err(|e| e.to_string())
}

/// Show or create the main window
pub fn show_main_window(app: &AppHandle) {
    // Check if window already exists
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }

    // Create new main window
    match WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("Cloudreve")
        .inner_size(800.0, 600.0)
        .resizable(true)
        .visible(true)
        .build()
    {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(e) => {
            tracing::error!(target: "main", error = %e, "Failed to create main window");
        }
    }
}

/// Show or create the add-drive window
pub fn show_add_drive_window(app: &AppHandle) {
    // Check if window already exists
    if let Some(window) = app.get_webview_window("add-drive") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }

    // Create new add-drive window with mica effect
    let effects = WindowEffectsConfig {
        effects: vec![WindowEffect::Mica, WindowEffect::Acrylic],
        state: None,
        radius: None,
        color: None,
    };

    let builder = WebviewWindowBuilder::new(
        app,
        "add-drive",
        WebviewUrl::App("index.html/#/add-drive".into()),
    )
    .title("Add Drive")
    .inner_size(470.0, 630.0)
    .resizable(false)
    .visible(true)
    .transparent(true)
    .effects(effects)
    .decorations(false)
    .minimizable(false);

    // Platform-specific: title_bar_style and hidden_title are macOS-only
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true);

    match builder.build() {
        Ok(window) => {
            let _ = window.create_overlay_titlebar();
            let _ = window.set_focus();
        }
        Err(e) => {
            tracing::error!(target: "main", error = %e, "Failed to create add-drive window");
        }
    }
}
