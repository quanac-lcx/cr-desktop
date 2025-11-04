use super::models::{Task, TaskId};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Wrapper for Task in priority queue
struct PriorityTask {
    task: Task,
    sequence: u64, // Used for FIFO ordering within same priority
}

impl Eq for PriorityTask {}

impl PartialEq for PriorityTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority && self.sequence == other.sequence
    }
}

impl Ord for PriorityTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority comes first
        match self.task.priority.cmp(&other.task.priority) {
            Ordering::Equal => {
                // Within same priority, lower sequence (earlier) comes first
                other.sequence.cmp(&self.sequence)
            }
            other_order => other_order,
        }
    }
}

impl PartialOrd for PriorityTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority-based task queue
pub struct TaskQueue {
    heap: BinaryHeap<PriorityTask>,
    sequence_counter: u64,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            sequence_counter: 0,
        }
    }

    /// Add a task to the queue
    pub fn push(&mut self, task: Task) {
        let priority_task = PriorityTask {
            task,
            sequence: self.sequence_counter,
        };
        self.sequence_counter += 1;
        self.heap.push(priority_task);
    }

    /// Get the highest priority task from the queue
    pub fn pop(&mut self) -> Option<Task> {
        self.heap.pop().map(|pt| pt.task)
    }

    /// Peek at the highest priority task without removing it
    pub fn peek(&self) -> Option<&Task> {
        self.heap.peek().map(|pt| &pt.task)
    }

    /// Get the number of tasks in the queue
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Remove a specific task by ID
    pub fn remove(&mut self, task_id: &TaskId) -> Option<Task> {
        // Convert to vec, remove the task, and rebuild heap
        let mut tasks: Vec<_> = self.heap.drain().collect();

        if let Some(pos) = tasks.iter().position(|pt| pt.task.id == *task_id) {
            let removed = tasks.remove(pos);

            // Rebuild heap with remaining tasks
            self.heap = tasks.into_iter().collect();

            Some(removed.task)
        } else {
            // Rebuild heap with all tasks
            self.heap = tasks.into_iter().collect();
            None
        }
    }

    /// Get all tasks as a vector (for listing purposes)
    pub fn get_all(&self) -> Vec<&Task> {
        self.heap.iter().map(|pt| &pt.task).collect()
    }

    /// Clear all tasks from the queue
    pub fn clear(&mut self) -> Vec<Task> {
        self.heap.drain().map(|pt| pt.task).collect()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}
