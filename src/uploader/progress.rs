//! Progress reporting for uploads with byte-level tracking, speed calculation,
//! and support for concurrent chunk uploads.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Progress update information sent to callbacks
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Total file size in bytes
    pub total_size: u64,
    /// Total bytes uploaded so far
    pub uploaded: u64,
    /// Progress percentage (0.0 - 1.0)
    pub progress: f64,
    /// Current upload speed in bytes per second
    pub speed_bytes_per_sec: u64,
    /// Estimated time remaining in seconds (None if speed is 0)
    pub eta_seconds: Option<u64>,
    /// Number of chunks being uploaded concurrently
    pub concurrent_chunks: usize,
    /// Total number of chunks
    pub total_chunks: usize,
    /// Completed chunk count
    pub completed_chunks: usize,
}

impl ProgressUpdate {
    /// Create a new progress update
    pub fn new(
        total_size: u64,
        uploaded: u64,
        speed_bytes_per_sec: u64,
        concurrent_chunks: usize,
        total_chunks: usize,
        completed_chunks: usize,
    ) -> Self {
        let progress = if total_size > 0 {
            (uploaded as f64 / total_size as f64).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let eta_seconds = if speed_bytes_per_sec > 0 && uploaded < total_size {
            Some((total_size - uploaded) / speed_bytes_per_sec)
        } else {
            None
        };

        Self {
            total_size,
            uploaded,
            progress,
            speed_bytes_per_sec,
            eta_seconds,
            concurrent_chunks,
            total_chunks,
            completed_chunks,
        }
    }
}

/// Trait for receiving progress updates
pub trait ProgressCallback: Send + Sync {
    /// Called when upload progress changes
    fn on_progress(&self, update: ProgressUpdate);
}

/// No-op progress callback implementation
#[allow(dead_code)]
pub struct NoOpProgress;

impl ProgressCallback for NoOpProgress {
    fn on_progress(&self, _update: ProgressUpdate) {}
}

/// Closure-based progress callback
#[allow(dead_code)]
pub struct FnProgress<F>(pub F);

impl<F> ProgressCallback for FnProgress<F>
where
    F: Fn(ProgressUpdate) + Send + Sync,
{
    fn on_progress(&self, update: ProgressUpdate) {
        (self.0)(update)
    }
}

/// Arc wrapper for progress callbacks
impl<T: ProgressCallback> ProgressCallback for Arc<T> {
    fn on_progress(&self, update: ProgressUpdate) {
        (**self).on_progress(update)
    }
}

/// Box wrapper for progress callbacks
impl ProgressCallback for Box<dyn ProgressCallback> {
    fn on_progress(&self, update: ProgressUpdate) {
        (**self).on_progress(update)
    }
}

/// Speed calculator using sliding window for accurate speed measurement
struct SpeedCalculator {
    /// Bytes recorded at different time points for sliding window
    samples: Vec<(Instant, u64)>,
    /// Window size for speed calculation (2 seconds)
    window_duration: Duration,
}

impl SpeedCalculator {
    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(32),
            window_duration: Duration::from_secs(2),
        }
    }

    /// Record a sample and calculate current speed
    fn record_and_calculate(&mut self, total_bytes: u64) -> u64 {
        let now = Instant::now();

        // Add new sample
        self.samples.push((now, total_bytes));

        // Remove samples outside the window
        let cutoff = now - self.window_duration;
        self.samples.retain(|(t, _)| *t >= cutoff);

        // Calculate speed from oldest sample in window
        if self.samples.len() >= 2 {
            let (oldest_time, oldest_bytes) = self.samples.first().unwrap();
            let elapsed = now.duration_since(*oldest_time);
            if elapsed.as_millis() > 0 {
                let bytes_diff = total_bytes.saturating_sub(*oldest_bytes);
                return (bytes_diff as f64 / elapsed.as_secs_f64()) as u64;
            }
        }

        0
    }
}

/// Thread-safe progress tracker for concurrent chunk uploads.
///
/// This tracker:
/// - Uses atomic counters for byte-level tracking across concurrent uploads
/// - Calculates upload speed using a sliding window
/// - Throttles progress reports to avoid performance drain
/// - Supports tracking multiple concurrent chunk uploads
pub struct ProgressTracker {
    /// Total file size
    total_size: u64,
    /// Total bytes uploaded (atomic for concurrent access)
    uploaded_bytes: AtomicU64,
    /// Completed bytes from finished chunks (for accurate tracking)
    completed_bytes: AtomicU64,
    /// Number of active concurrent chunk uploads
    active_chunks: AtomicU64,
    /// Total number of chunks
    total_chunks: usize,
    /// Number of completed chunks
    completed_chunks: AtomicU64,
    /// Speed calculator (protected by RwLock)
    speed_calc: RwLock<SpeedCalculator>,
    /// Cached speed value
    cached_speed: AtomicU64,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(total_size: u64, total_chunks: usize) -> Arc<Self> {
        Arc::new(Self {
            total_size,
            uploaded_bytes: AtomicU64::new(0),
            completed_bytes: AtomicU64::new(0),
            active_chunks: AtomicU64::new(0),
            total_chunks,
            completed_chunks: AtomicU64::new(0),
            speed_calc: RwLock::new(SpeedCalculator::new()),
            cached_speed: AtomicU64::new(0),
        })
    }

    /// Called when starting to upload a chunk
    pub fn start_chunk(&self) {
        self.active_chunks.fetch_add(1, Ordering::SeqCst);
    }

    /// Called when a chunk upload completes
    pub fn complete_chunk(&self, chunk_size: u64) {
        self.active_chunks.fetch_sub(1, Ordering::SeqCst);
        self.completed_chunks.fetch_add(1, Ordering::SeqCst);
        // Add the completed chunk size to completed_bytes
        self.completed_bytes.fetch_add(chunk_size, Ordering::SeqCst);
    }

    /// Add bytes uploaded within current chunk(s)
    /// This is the in-flight bytes for active chunks
    pub fn add_bytes(&self, bytes: u64) {
        self.uploaded_bytes.fetch_add(bytes, Ordering::SeqCst);
    }

    /// Reset in-flight bytes for a chunk (called before retry)
    pub fn reset_chunk_bytes(&self, bytes: u64) {
        self.uploaded_bytes.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// Get total uploaded bytes (completed + in-flight)
    pub fn total_uploaded(&self) -> u64 {
        self.completed_bytes.load(Ordering::SeqCst) + self.uploaded_bytes.load(Ordering::SeqCst)
    }

    /// Force create a progress update (for final report)
    pub async fn create_update(&self) -> ProgressUpdate {
        let total_uploaded = self.total_uploaded();

        // Calculate speed
        let speed = {
            let mut calc = self.speed_calc.write().await;
            let speed = calc.record_and_calculate(total_uploaded);
            self.cached_speed.store(speed, Ordering::SeqCst);
            speed
        };

        ProgressUpdate::new(
            self.total_size,
            total_uploaded,
            speed,
            self.active_chunks.load(Ordering::SeqCst) as usize,
            self.total_chunks,
            self.completed_chunks.load(Ordering::SeqCst) as usize,
        )
    }
}
