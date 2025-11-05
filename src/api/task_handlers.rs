use crate::tasks::{
    TaskCallback, TaskExecutionResult, TaskExecutor, TaskFilter, TaskId, TaskInfo, TaskPriority,
    TaskProperties, TaskStatistics, TaskType,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use super::{ApiResponse, AppError, AppState};

/// Request to submit a new task
#[derive(Debug, Deserialize)]
pub struct SubmitTaskRequest {
    pub task_type: TaskType,
    #[serde(default)]
    pub priority: TaskPriority,
    pub target_path: Option<String>,
    pub source_path: Option<String>,
    pub drive_id: Option<String>,
}

/// Query parameters for listing tasks
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub task_type: Option<String>,
    pub target_path: Option<PathBuf>,
    pub drive_id: Option<String>,
    pub status: Option<String>,
}

/// Request to update task manager config
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub max_workers: Option<usize>,
    pub completed_buffer_size: Option<usize>,
}

/// Response for task submission
#[derive(Debug, Serialize)]
pub struct SubmitTaskResponse {
    pub task_id: TaskId,
    pub status: String,
}

/// List all tasks with optional filtering
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<ApiResponse<Vec<TaskInfo>>>, AppError> {
    tracing::debug!(target: "api::tasks", "Listing tasks with filters");

    // Build filter from query
    let mut filter = TaskFilter::new();

    if let Some(task_type_str) = query.task_type {
        let task_type = parse_task_type(&task_type_str)?;
        filter = filter.with_type(task_type);
    }

    if let Some(path) = query.target_path {
        filter = filter.with_path(path);
    }

    if let Some(drive_id) = query.drive_id {
        filter = filter.with_drive_id(drive_id);
    }

    if let Some(status_str) = query.status {
        let status = parse_task_status(&status_str)?;
        filter = filter.with_status(status);
    }

    let tasks = state.task_manager.get_tasks(Some(filter)).await;

    tracing::info!(target: "api::tasks", count = tasks.len(), "Retrieved tasks");
    Ok(Json(ApiResponse::success(tasks)))
}

/// Get a specific task by ID
pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<TaskId>,
) -> Result<Json<ApiResponse<TaskInfo>>, AppError> {
    tracing::debug!(target: "api::tasks", task_id = %task_id, "Getting task details");

    match state.task_manager.get_task(&task_id).await {
        Some(task) => {
            tracing::info!(target: "api::tasks", task_id = %task_id, "Task found");
            Ok(Json(ApiResponse::success(task)))
        }
        None => {
            tracing::warn!(target: "api::tasks", task_id = %task_id, "Task not found");
            Err(AppError::NotFound(format!("Task not found: {}", task_id)))
        }
    }
}

/// Submit a new task
///
/// Note: This is a temporary stub implementation. The full implementation requires
/// resolving complex type interactions between TaskManager's async methods and axum's
/// Handler trait. Users can interact with the TaskManager directly in code or we can
/// expose a simpler synchronous API for HTTP endpoints.
#[debug_handler]
pub async fn submit_task(
    State(_state): State<AppState>,
    Json(req): Json<SubmitTaskRequest>,
) -> (StatusCode, Json<ApiResponse<SubmitTaskResponse>>) {
    let task_id = uuid::Uuid::new_v4().to_string();

    _state
        .task_manager
        .submit_simple_task(
            task_id.clone(),
            req.priority.clone(),
            TaskProperties::new(req.task_type.clone()),
        )
        .await
        .unwrap();

    tracing::info!(
        target: "api::tasks",
        task_id = %task_id,
        task_type = ?req.task_type,
        priority = ?req.priority,
        "Received task submission request (stub implementation)"
    );

    // TODO: Full implementation - see note above
    (
        StatusCode::ACCEPTED,
        Json(ApiResponse::success(SubmitTaskResponse {
            task_id,
            status: "accepted".to_string(),
        })),
    )
}

/// Cancel a task
pub async fn cancel_task(
    State(state): State<AppState>,
    Path(task_id): Path<TaskId>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::info!(target: "api::tasks", task_id = %task_id, "Cancelling task");

    state
        .task_manager
        .cancel_task(&task_id)
        .await
        .map_err(|e| AppError::NotFound(e))?;

    tracing::info!(target: "api::tasks", task_id = %task_id, "Task cancelled successfully");

    Ok(Json(ApiResponse::success(serde_json::json!({
        "task_id": task_id,
        "status": "cancelled"
    }))))
}

/// Stop all tasks
pub async fn stop_all_tasks(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::info!(target: "api::tasks", "Stopping all tasks");

    let count = state.task_manager.stop_all_tasks().await;

    tracing::info!(target: "api::tasks", cancelled_count = count, "All tasks stopped");

    Ok(Json(ApiResponse::success(serde_json::json!({
        "cancelled_count": count,
        "status": "stopped"
    }))))
}

/// Get task statistics
pub async fn get_statistics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<TaskStatistics>>, AppError> {
    tracing::debug!(target: "api::tasks", "Getting task statistics");

    let stats = state.task_manager.get_statistics().await;

    Ok(Json(ApiResponse::success(stats)))
}

/// Update task manager configuration
pub async fn update_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::info!(target: "api::tasks", "Updating task manager configuration");

    if let Some(max_workers) = req.max_workers {
        state.task_manager.set_max_workers(max_workers).await;
    }

    if let Some(buffer_size) = req.completed_buffer_size {
        state
            .task_manager
            .set_completed_buffer_size(buffer_size)
            .await;
    }

    tracing::info!(target: "api::tasks", "Configuration updated");

    Ok(Json(ApiResponse::success(serde_json::json!({
        "status": "updated"
    }))))
}

/// Clear completed tasks
pub async fn clear_completed(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::info!(target: "api::tasks", "Clearing completed tasks");

    let count = state.task_manager.clear_completed_tasks().await;

    tracing::info!(target: "api::tasks", cleared_count = count, "Completed tasks cleared");

    Ok(Json(ApiResponse::success(serde_json::json!({
        "cleared_count": count,
        "status": "cleared"
    }))))
}

// Helper functions

fn parse_task_type(s: &str) -> Result<TaskType, AppError> {
    match s.to_lowercase().as_str() {
        "upload" => Ok(TaskType::Upload),
        "download" => Ok(TaskType::Download),
        "sync" => Ok(TaskType::Sync),
        "delete" => Ok(TaskType::Delete),
        "copy" => Ok(TaskType::Copy),
        "move" => Ok(TaskType::Move),
        other => Ok(TaskType::Custom(other.to_string())),
    }
}

fn parse_task_status(s: &str) -> Result<crate::tasks::TaskStatus, AppError> {
    match s.to_lowercase().as_str() {
        "pending" => Ok(crate::tasks::TaskStatus::Pending),
        "running" => Ok(crate::tasks::TaskStatus::Running),
        "completed" => Ok(crate::tasks::TaskStatus::Completed),
        "failed" => Ok(crate::tasks::TaskStatus::Failed),
        "cancelled" => Ok(crate::tasks::TaskStatus::Cancelled),
        _ => Err(AppError::BadRequest(format!("Invalid status: {}", s))),
    }
}

fn create_task_callback(
    event_broadcaster: crate::events::EventBroadcaster,
    task_type: TaskType,
) -> TaskCallback {
    Arc::new(
        move |task_id: TaskId, status, execution_result: TaskExecutionResult| {
            let broadcaster = event_broadcaster.clone();
            let task_type = task_type.clone();
            Box::pin(async move {
                let payload = serde_json::json!({
                    "task_id": task_id,
                    "task_type": task_type,
                    "status": status,
                    "error": execution_result.error,
                    "result_data": execution_result.result_data,
                });
                broadcaster.custom_event("task_completed".to_string(), payload);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        },
    )
}

fn create_executor_for_task(task_type: &TaskType, state: &AppState) -> TaskExecutor {
    // Create task-type specific executor
    // This is a simplified example - in production, you'd have different executors for each task type
    let task_type = task_type.clone();
    let event_broadcaster = state.event_broadcaster.clone();

    Arc::new(move |props| {
        let task_type = task_type.clone();
        let broadcaster = event_broadcaster.clone();

        Box::pin(async move {
            tracing::info!(target: "tasks::executor", task_type = ?task_type, "Executing task");

            // Simulate work with progress updates
            for i in 0..10 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Update progress
                let mut p = props.write().await;
                p.progress = (i + 1) as f32 / 10.0;

                // Broadcast progress event
                if let Some(drive_id) = &p.drive_id {
                    if let Some(target_path) = &p.target_path {
                        broadcaster.sync_progress(
                            drive_id.clone(),
                            p.progress,
                            target_path.display().to_string(),
                        );
                    }
                }
                drop(p);
            }

            tracing::info!(target: "tasks::executor", task_type = ?task_type, "Task execution completed");

            // Return success with optional custom result data
            TaskExecutionResult::ok()
        })
    })
}
