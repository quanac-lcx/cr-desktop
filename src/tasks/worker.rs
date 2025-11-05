use super::models::{Task, TaskExecutionResult, TaskId, TaskStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing;

/// Message types for worker communication
pub enum WorkerMessage {
    Execute(Task),
    Cancel(TaskId, oneshot::Sender<bool>),
    Shutdown,
}

/// Result of task execution with custom result data
pub struct TaskResult {
    pub task_id: TaskId,
    pub status: TaskStatus,
    pub execution_result: TaskExecutionResult,
}

/// Worker pool that manages task execution
pub struct WorkerPool {
    max_workers: usize,
    active_workers: Arc<Mutex<HashMap<TaskId, JoinHandle<()>>>>,
    worker_tx: mpsc::UnboundedSender<WorkerMessage>,
    result_rx: Arc<Mutex<mpsc::UnboundedReceiver<TaskResult>>>,
}

impl WorkerPool {
    pub fn new(max_workers: usize) -> Self {
        let (worker_tx, worker_rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = mpsc::unbounded_channel();
        let active_workers = Arc::new(Mutex::new(HashMap::new()));

        // Spawn worker dispatcher
        let worker_rx = Arc::new(Mutex::new(worker_rx));
        let dispatcher_workers = active_workers.clone();

        tokio::spawn(async move {
            let mut rx = worker_rx.lock().await;

            while let Some(msg) = rx.recv().await {
                match msg {
                    WorkerMessage::Execute(task) => {
                        let task_id = task.id.clone();
                        let task_id_for_map = task_id.clone(); // Clone for inserting into map
                        let result_tx = result_tx.clone();
                        let workers = dispatcher_workers.clone();

                        let handle = tokio::spawn(async move {
                            tracing::info!(target: "tasks::worker", task_id = %task_id, "Starting task execution");

                            // Execute the task
                            let execution_result = (task.executor)(task.properties.clone()).await;

                            // Determine task status based on execution result
                            let status = if execution_result.success {
                                tracing::info!(target: "tasks::worker", task_id = %task_id, "Task completed successfully");
                                TaskStatus::Completed
                            } else {
                                tracing::error!(target: "tasks::worker", task_id = %task_id, error = ?execution_result.error, "Task failed");
                                TaskStatus::Failed
                            };

                            // Create task result
                            let task_result = TaskResult {
                                task_id: task_id.clone(),
                                status: status.clone(),
                                execution_result: execution_result.clone(),
                            };

                            // Execute callback if present
                            if let Some(callback) = task.callback {
                                callback(
                                    task_result.task_id.clone(),
                                    task_result.status.clone(),
                                    execution_result,
                                )
                                .await;
                            }

                            let _ = result_tx.send(task_result);

                            // Remove from active workers
                            workers.lock().await.remove(&task_id);
                        });

                        dispatcher_workers
                            .lock()
                            .await
                            .insert(task_id_for_map, handle);
                    }
                    WorkerMessage::Cancel(task_id, response_tx) => {
                        let mut workers = dispatcher_workers.lock().await;
                        if let Some(handle) = workers.remove(&task_id) {
                            handle.abort();
                            tracing::info!(target: "tasks::worker", task_id = %task_id, "Task cancelled");
                            let _ = response_tx.send(true);
                        } else {
                            let _ = response_tx.send(false);
                        }
                    }
                    WorkerMessage::Shutdown => {
                        tracing::info!(target: "tasks::worker", "Worker pool shutting down");
                        break;
                    }
                }
            }
        });

        Self {
            max_workers,
            active_workers,
            worker_tx,
            result_rx: Arc::new(Mutex::new(result_rx)),
        }
    }

    /// Submit a task for execution
    pub async fn execute(&self, task: Task) -> Result<(), String> {
        let active_count = self.active_workers.lock().await.len();

        if active_count >= self.max_workers {
            return Err(format!(
                "Worker pool at capacity ({}/{})",
                active_count, self.max_workers
            ));
        }

        self.worker_tx
            .send(WorkerMessage::Execute(task))
            .map_err(|e| format!("Failed to send task to worker: {}", e))?;

        Ok(())
    }

    /// Cancel a running task
    pub async fn cancel(&self, task_id: &TaskId) -> bool {
        let (tx, rx) = oneshot::channel();

        if self
            .worker_tx
            .send(WorkerMessage::Cancel(task_id.clone(), tx))
            .is_err()
        {
            return false;
        }

        rx.await.unwrap_or(false)
    }

    /// Get the next task result (non-blocking)
    pub async fn next_result(&self) -> Option<TaskResult> {
        self.result_rx.lock().await.try_recv().ok()
    }

    /// Check if worker pool has capacity
    pub async fn has_capacity(&self) -> bool {
        self.active_workers.lock().await.len() < self.max_workers
    }

    /// Get number of active workers
    pub async fn active_count(&self) -> usize {
        self.active_workers.lock().await.len()
    }

    /// Get IDs of all running tasks
    pub async fn running_task_ids(&self) -> Vec<TaskId> {
        self.active_workers.lock().await.keys().cloned().collect()
    }

    /// Shutdown the worker pool
    pub async fn shutdown(&self) {
        // Cancel all running tasks
        let task_ids = self.running_task_ids().await;
        for task_id in task_ids {
            self.cancel(&task_id).await;
        }

        // Send shutdown signal
        let _ = self.worker_tx.send(WorkerMessage::Shutdown);
    }

    /// Update max workers configuration
    pub fn set_max_workers(&mut self, max_workers: usize) {
        self.max_workers = max_workers;
        tracing::info!(target: "tasks::worker", max_workers, "Updated max workers");
    }

    pub fn max_workers(&self) -> usize {
        self.max_workers
    }
}
