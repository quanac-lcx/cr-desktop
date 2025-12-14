# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cloudreve Sync Service is a Rust-based Windows desktop application that synchronizes files with a Cloudreve cloud drive server using the Windows Cloud Files API (cfapi). It provides:
- Real-time bidirectional file synchronization
- On-demand file hydration (files appear locally but are downloaded only when accessed)
- Windows Explorer shell integration (context menus, thumbnails, custom states)
- Multiple storage provider support for uploads (S3, OneDrive, Qiniu, Upyun, local)

## Build Commands

```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run the application (requires Windows)
cargo run

# Check for compilation errors without building
cargo check

# Run all tests
cargo test

# Run tests for a specific module
cargo test --package cloudreve-sync --lib inventory::db::tests

# Run tests for the cloudreve-api crate
cargo test --package cloudreve-api
```

## Architecture

### Crate Structure

- **cloudreve-sync** (main crate): The desktop sync service application
- **cloudreve-api** (workspace member at `cloudreve-api/`): Async Rust client library for Cloudreve REST API with automatic token refresh

### Module Organization

**Core Modules:**
- `drive/`: Drive management and sync operations
  - `manager.rs`: Central `DriveManager` coordinating all mounted drives
  - `mounts.rs`: Individual mount point (`Mount`) handling
  - `callback.rs`: Windows Cloud Filter callbacks (`SyncFilter` trait implementation)
  - `sync.rs`: Sync logic and placeholder/metadata conversion
  - `remote_events.rs`: Server-sent event handling for remote changes
- `cfapi/`: Windows Cloud Files API wrapper
  - `filter/`: Callback traits (`SyncFilter`, `Filter`) and request handling
  - `placeholder.rs`, `placeholder_file.rs`: Cloud file placeholder management
  - `root/`: Sync root registration and connection
- `inventory/`: SQLite database (via Diesel ORM) for local metadata persistence
  - Schema migrations in `migrations/inventory/`
  - Stores file metadata, task queue, upload sessions, drive properties
- `uploader/`: Chunked file upload with multiple storage provider backends
  - `providers/`: S3, OneDrive, Qiniu, Upyun, local implementations
  - Supports encryption and resumable uploads
- `tasks/`: Background task queue for uploads and sync operations
- `api/`: HTTP server (Axum) exposing REST endpoints and SSE for UI communication
- `shellext/`: Windows shell extensions (context menus, thumbnails, status UI)
- `events/`: Event broadcasting system for real-time UI updates

### Key Patterns

**Command Pattern**: Both `DriveManager` and individual `Mount` instances use async command channels (`mpsc::UnboundedSender<ManagerCommand>` / `MountCommand`) to handle operations from shell extensions and the API.

**Callback Threading**: Windows Cloud Filter callbacks (`SyncFilter` trait) run on OS threads. They use `blocking_recv()` on oneshot channels to wait for async operations.

**Database**: Single SQLite database at `~/.cloudreve/meta.db` with embedded Diesel migrations. Connection pool limited to 1 connection.

### HTTP API

Server runs on `0.0.0.0:3000` with endpoints:
- `GET /health`: Health check
- `GET/POST/PUT/DELETE /api/drives/*`: Drive management
- `GET /api/events`: SSE stream for real-time updates

## Windows-Specific Notes

This application targets Windows and uses:
- Windows Cloud Files API (`windows` crate with extensive feature flags)
- COM shell extensions for Explorer integration
- The `cfapi` module wraps low-level Windows APIs

## Database Migrations

Migrations are embedded and run automatically on startup. To add a new migration:
1. Create folder in `migrations/inventory/` (e.g., `0005_new_table/`)
2. Add `up.sql` and `down.sql` files
3. Use idempotent SQL (`IF NOT EXISTS`) for clean upgrades
