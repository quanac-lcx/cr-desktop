# Task Queue System Documentation

## Overview

A comprehensive task queue system has been implemented with priority-based scheduling, worker pool management, and extensive task management capabilities.

## Features Implemented

### ✅ 1. Configurable Max Workers
- Set maximum number of concurrent workers via `TaskManagerConfig`
- Update max workers dynamically via API: `PUT /api/tasks/config`
- Workers are managed automatically by the worker pool

### ✅ 2. Stop Tasks
- Cancel individual tasks: `POST /api/tasks/{id}/cancel`
- Stop all tasks (pending + running): `POST /api/tasks/stop`
- Cancelled tasks are moved to completed buffer with status

### ✅ 3. Task Properties
Tasks have comprehensive properties:
- `task_type`: Upload, Download, Sync, Delete, Copy, Move, or Custom
- `target_path`: Target file path
- `source_path`: Source file path (for copy/move)
- `drive_id`: Associated drive ID
- `progress`: Current progress (0.0 - 1.0)
- `total_size` / `processed_size`: Size tracking
- `metadata`: Additional custom properties

### ✅ 4. Priority Queue
- Four priority levels: Low (0), Normal (1), High (2), Critical (3)
- Higher priority tasks are executed first
- FIFO ordering within same priority level
- New high-priority tasks can jump ahead in the queue

### ✅ 5. Task Search/Filter
- Search by task type: `GET /api/tasks?task_type=upload`
- Filter by target path: `GET /api/tasks?target_path=/path/to/file`
- Filter by drive ID: `GET /api/tasks?drive_id=drive-123`
- Filter by status: `GET /api/tasks?status=running`
- Combine multiple filters

### ✅ 6. Task Listing & Properties
- List all tasks: `GET /api/tasks`
- Get specific task: `GET /api/tasks/{id}`
- Get statistics: `GET /api/tasks/statistics`
- Each task includes: ID, type, priority, status, timestamps, progress, error (if any)

### ✅ 7. Completed Task Buffer
- Configurable buffer size (default: 100)
- FIFO eviction when buffer is full
- Update buffer size: `PUT /api/tasks/config` with `completed_buffer_size`
- Clear completed tasks: `POST /api/tasks/completed/clear`

### ✅ 8. Task Completion Callbacks/Signals
- Optional callback system for task completion
- Callbacks receive: task_id, status, error (if any)
- Integrated with event broadcasting system
- Custom events sent on task completion

## Architecture

### Core Components

#### 1. TaskManager (`src/tasks/manager.rs`)
Main task management interface with:
- Task submission and lifecycle management
- Background task processor
- Statistics tracking
- Configuration management

#### 2. TaskQueue (`src/tasks/queue.rs`)
Priority-based task queue using binary heap:
- O(log n) insertion
- O(log n) removal of highest priority
- Priority ordering with FIFO tie-breaking

#### 3. WorkerPool (`src/tasks/worker.rs`)
Manages concurrent task execution:
- Configurable worker limits
- Task cancellation support
- Result collection
- Graceful shutdown

#### 4. Task Models (`src/tasks/models.rs`)
Core data structures:
- `Task`: Complete task representation
- `TaskInfo`: Serializable task information
- `TaskProperties`: Task metadata
- `TaskFilter`: Search/filter criteria

## API Endpoints

### Task Management

```
GET    /api/tasks                  - List all tasks (with optional filters)
GET    /api/tasks/{id}             - Get specific task
POST   /api/tasks                  - Submit new task (stub)
POST   /api/tasks/{id}/cancel      - Cancel a task
POST   /api/tasks/stop             - Stop all tasks
GET    /api/tasks/statistics       - Get task statistics
PUT    /api/tasks/config           - Update configuration
POST   /api/tasks/completed/clear  - Clear completed tasks
```

### Example Requests

#### List Running Tasks
```bash
curl "http://localhost:3000/api/tasks?status=running"
```

#### Get Task Statistics
```bash
curl http://localhost:3000/api/tasks/statistics
```

Response:
```json
{
  "success": true,
  "data": {
    "pending_count": 5,
    "running_count": 2,
    "completed_count": 10,
    "failed_count": 1,
    "cancelled_count": 2,
    "max_workers": 4,
    "completed_buffer_size": 100
  }
}
```

#### Cancel a Task
```bash
curl -X POST http://localhost:3000/api/tasks/{task-id}/cancel
```

#### Update Configuration
```bash
curl -X PUT http://localhost:3000/api/tasks/config \
  -H "Content-Type: application/json" \
  -d '{
    "max_workers": 8,
    "completed_buffer_size": 200
  }'
```

## Direct API Usage (Programmatic)

For full control, use the TaskManager directly in your Rust code:

```rust
use tasks::{TaskManager, TaskManagerConfig, TaskProperties, TaskType, TaskPriority};
use std::sync::Arc;

// Initialize
let config = TaskManagerConfig {
    max_workers: 4,
    completed_buffer_size: 100,
};
let task_manager = TaskManager::new(config);

// Submit a simple task
let task_id = uuid::Uuid::new_v4().to_string();
let properties = TaskProperties::new(TaskType::Upload)
    .with_target_path("/path/to/file".into())
    .with_drive_id("drive-123".to_string());

task_manager
    .submit_simple_task(task_id.clone(), TaskPriority::High, properties)
    .await?;

// Query tasks
let filter = TaskFilter::new()
    .with_type(TaskType::Upload)
    .with_status(TaskStatus::Running);
    
let tasks = task_manager.get_tasks(Some(filter)).await;

// Get statistics
let stats = task_manager.get_statistics().await;

// Cancel a task
task_manager.cancel_task(&task_id).await?;

// Stop all tasks
task_manager.stop_all_tasks().await;
```

## Custom Task Executors

For advanced use cases, create custom task executors:

```rust
use tasks::{TaskExecutor, TaskCallback};

// Custom executor
let executor: TaskExecutor = Arc::new(|props| {
    Box::pin(async move {
        // Your custom task logic here
        let mut p = props.write().await;
        
        // Update progress
        for i in 0..100 {
            p.progress = i as f32 / 100.0;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        Ok(())
    })
});

// Custom callback
let callback: TaskCallback = Arc::new(|task_id, status, error| {
    Box::pin(async move {
        println!("Task {} completed with status: {:?}", task_id, status);
    }) as Pin<Box<dyn Future<Output = ()> + Send>>
});

// Submit with custom executor and callback
task_manager
    .submit_task(task_id, priority, properties, executor, Some(callback))
    .await?;
```

## Configuration

Default configuration in `main.rs`:

```rust
let task_config = TaskManagerConfig {
    max_workers: 4,                // Concurrent workers
    completed_buffer_size: 100,    // Completed task history
};
```

## Graceful Shutdown

The task manager integrates with the application's graceful shutdown:

1. Signals received (SIGTERM, SIGINT)
2. All pending tasks cancelled
3. All running tasks cancelled
4. Worker pool shutdown
5. State persisted

## Known Limitations

### Task Submission via HTTP

The `POST /api/tasks` endpoint is currently a stub due to complex type interactions between Rust's async traits and Axum's Handler trait derivation. The TaskManager works perfectly when used directly in Rust code.

**Workaround**: Use the TaskManager API directly in your Rust code, or integrate task submission through other application-specific endpoints (e.g., triggering a sync task when a drive is added).

## Testing

Basic queue tests are included:

```bash
cargo test --lib tasks
```

## Future Enhancements

- [ ] Persistent task queue (survive restarts)
- [ ] Task dependencies and workflows
- [ ] Scheduled/recurring tasks
- [ ] Task result storage
- [ ] Metrics and monitoring endpoints
- [ ] Resolve HTTP task submission endpoint

## Event Integration

Tasks integrate with the existing event broadcasting system:
- Task completion events sent via SSE
- Subscribe to events: `GET /api/events`
- Progress updates broadcast for long-running tasks

## Examples

See `src/api/task_handlers.rs` for API integration examples and `src/tasks/manager.rs` for programmatic usage.

