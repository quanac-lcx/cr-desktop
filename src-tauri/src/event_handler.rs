use cloudreve_sync::events::Event;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::show_add_drive_window_impl;

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
    show_add_drive_window_impl(app_handle);
}

/// Emit an event to the frontend
pub fn emit_event(app_handle: &AppHandle, event: &Event) {
    if let Err(e) = app_handle.emit(event.name(), event) {
        tracing::error!(target: "events", error = %e, "Failed to emit event to frontend");
    } else {
        tracing::trace!(target: "events", event = ?event, "Event emitted to frontend");
    }
}
