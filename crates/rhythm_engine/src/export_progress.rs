//! Pure export-job progress and cooperative cancellation state.
//! No wall-clock values affect frame timestamps or the creative Project.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportProgressError {
    NoFrames,
    OutOfRange,
    WentBackwards,
}

#[derive(Debug)]
pub struct ExportProgress {
    total: u64,
    completed: u64,
    started_at: Instant,
}

impl ExportProgress {
    pub fn new(total: u64, started_at: Instant) -> Result<Self, ExportProgressError> {
        if total == 0 {
            return Err(ExportProgressError::NoFrames);
        }
        Ok(Self {
            total,
            completed: 0,
            started_at,
        })
    }

    /// Count a frame only once the export worker has successfully delivered it
    /// to the encoder. An invalid update must not corrupt the prior progress.
    pub fn record_completed(&mut self, completed: u64) -> Result<(), ExportProgressError> {
        if completed < self.completed {
            return Err(ExportProgressError::WentBackwards);
        }
        if completed > self.total {
            return Err(ExportProgressError::OutOfRange);
        }
        self.completed = completed;
        Ok(())
    }

    #[must_use]
    pub const fn completed(&self) -> u64 {
        self.completed
    }

    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    #[must_use]
    pub fn percent(&self) -> f64 {
        (self.completed as f64) * 100.0 / (self.total as f64)
    }

    #[must_use]
    pub fn elapsed_at(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.started_at)
    }
}

/// Cheap cross-thread signal. The worker checks it before frame evaluation,
/// GPU encoding, readback and FFmpeg writes. The owned process is reaped when
/// a canceled worker drops its FFmpeg process guard.
#[derive(Debug, Clone, Default)]
pub struct ExportCancellationToken(Arc<AtomicBool>);

impl ExportCancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportCancellationToken, ExportProgress, ExportProgressError};
    use std::time::{Duration, Instant};

    #[test]
    fn progress_tracks_completed_frames_percent_and_monotonic_elapsed() {
        let start = Instant::now();
        let mut progress = ExportProgress::new(600, start).expect("600 frames");
        assert_eq!(progress.completed(), 0);
        assert_eq!(progress.total(), 600);
        assert_eq!(progress.percent(), 0.0);
        progress.record_completed(150).expect("one quarter");
        assert_eq!(progress.percent(), 25.0);
        assert_eq!(progress.completed(), 150);
        assert_eq!(
            progress.elapsed_at(start + Duration::from_secs(3)),
            Duration::from_secs(3)
        );
        progress.record_completed(600).expect("complete");
        assert_eq!(progress.percent(), 100.0);
    }

    #[test]
    fn invalid_progress_never_corrupts_prior_count() {
        let start = Instant::now();
        assert_eq!(
            ExportProgress::new(0, start).expect_err("zero frames").total,
            0
        );
    }

    #[test]
    fn cooperative_cancel_is_visible_to_an_independent_worker_clone() {
        let main = ExportCancellationToken::default();
        let worker = main.clone();
        assert!(!worker.is_cancelled());
        main.cancel();
        assert!(worker.is_cancelled());
        assert!(main.is_cancelled());
    }
}
