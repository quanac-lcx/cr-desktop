use cloudreve_sync::events::Event;
use tauri::{AppHandle, Emitter, Manager};

/// Handle incoming events from the event broadcaster.
/// Returns true if the event was handled, false otherwise.
pub fn handle_event(app_handle: &AppHandle, event: &Event) {
    match event {
        Event::NoDrive { .. } => handle_no_drive(app_handle),
        Event::ConnectionStatusChanged { .. } => {
            // Currently just forwarded to frontend via emit
        }
    }
}

fn handle_no_drive(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("add-drive") {
        let _ = window.show();
        let _ = window.set_focus();
        tracing::info!(target: "events", "Opened add-drive window due to NoDrive event");
    } else {
        tracing::error!(target: "events", "Failed to find add-drive window");
    }
}

/// Emit an event to the frontend
pub fn emit_event(app_handle: &AppHandle, event: &Event) {
    if let Err(e) = app_handle.emit(event.name(), event) {
        tracing::error!(target: "events", error = %e, "Failed to emit event to frontend");
    } else {
        tracing::trace!(target: "events", event = ?event, "Event emitted to frontend");
    }
}
