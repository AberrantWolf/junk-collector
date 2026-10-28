//! Bound durable cancellation polling during sector reads and CPU checkpoints.
use crate::*;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(1);

pub(super) struct ProcessingCancellation {
    probe: Box<dyn Fn() -> Result<bool> + Send + Sync>,
    last_clear: Mutex<Option<Instant>>,
}

impl ProcessingCancellation {
    pub(super) fn new(jobs: Arc<dyn JobPort>, id: JobId, token: String) -> Self {
        Self {
            probe: Box::new(move || jobs.cancellation_requested(id, &token)),
            last_clear: Mutex::new(None),
        }
    }

    pub(super) fn requested(&self) -> Result<bool> {
        self.requested_with_clock(Instant::now)
    }

    #[cfg(test)]
    fn requested_at(&self, now: Instant) -> Result<bool> {
        self.requested_with_clock(|| now)
    }

    fn requested_with_clock(&self, clock: impl FnOnce() -> Instant) -> Result<bool> {
        let mut last_clear = self.last_clear.lock().map_err(|_| {
            Error::Unavailable("processing cancellation lock poisoned; restart required".into())
        })?;
        // Sample after acquiring the shared poll: a suspended reader must not
        // reuse clearance using a timestamp from before it waited for the lock.
        let now = clock();
        if last_clear.is_some_and(|last| now.saturating_duration_since(last) < POLL_INTERVAL) {
            return Ok(false);
        }
        // Timestamp before the query: time spent waiting for storage must not
        // extend the interval. Never cache cancellation or a failed claim check.
        let cancelled = (self.probe)()?;
        if !cancelled {
            *last_clear = Some(now);
        }
        Ok(cancelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn sector_reads_share_bounded_polling_and_detect_cancellation() {
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let poll = ProcessingCancellation {
            probe: Box::new(move || Ok(count.fetch_add(1, Ordering::SeqCst) > 0)),
            last_clear: Mutex::new(None),
        };
        let start = Instant::now();
        // A fast reader can visit thousands of sectors inside one interval.
        // Those checkpoints must not monopolize the shared job database.
        for _ in 0..10_000 {
            assert!(!poll.requested_at(start).unwrap());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(
            !poll
                .requested_at(start + POLL_INTERVAL - Duration::from_nanos(1))
                .unwrap()
        );
        assert!(poll.requested_at(start + POLL_INTERVAL).unwrap());
        assert!(poll.requested_at(start + POLL_INTERVAL).unwrap());
    }

    #[test]
    fn expired_claim_or_storage_failure_is_never_cached_as_clear() {
        let calls = AtomicUsize::new(0);
        let poll = ProcessingCancellation {
            probe: Box::new(move || match calls.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(false),
                1 => Err(Error::CommandInProgress),
                _ => Err(Error::Storage("fixture read failed".into())),
            }),
            last_clear: Mutex::new(None),
        };
        let start = Instant::now();
        assert!(!poll.requested_at(start).unwrap());
        let expired = start + POLL_INTERVAL;
        assert!(matches!(
            poll.requested_at(expired),
            Err(Error::CommandInProgress)
        ));
        assert!(matches!(poll.requested_at(expired), Err(Error::Storage(_))));
    }

    #[test]
    fn reader_stops_before_consuming_bytes_when_the_next_poll_is_cancelled() {
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = cancelled.clone();
        let poll = Arc::new(ProcessingCancellation {
            probe: Box::new(move || Ok(flag.load(Ordering::SeqCst))),
            last_clear: Mutex::new(None),
        });
        let mut reader = crate::ProcessingReader {
            reader: Box::new(std::io::Cursor::new(vec![7; 2352 * 2])),
            cancellation: poll.clone(),
        };
        let mut sector = [0; 2352];
        assert_eq!(reader.read(&mut sector).unwrap(), sector.len());
        cancelled.store(true, Ordering::SeqCst);
        // Advance the polling window deterministically, without wall-clock sleeps.
        *poll.last_clear.lock().unwrap() = Some(Instant::now() - POLL_INTERVAL);
        assert!(reader.read(&mut sector).is_err());
        assert_eq!(reader.stream_position().unwrap(), 2352);
        assert!(
            poll.requested().unwrap(),
            "sibling checkpoints also see cancellation"
        );
    }
}
