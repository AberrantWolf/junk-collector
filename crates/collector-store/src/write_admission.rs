//! Admission to the sole local SQLite writer. Urgent Job control can pass
//! ordinary writes that are still queued. An ordinary waiter gets a turn
//! after at most three consecutive urgent admissions.
use crate::{Result, StdMutex, storage};
use std::sync::Condvar;

#[derive(Default)]
pub(crate) struct WriteAdmission {
    state: StdMutex<State>,
    available: Condvar,
}

#[derive(Default)]
struct State {
    busy: bool,
    ordinary: usize,
    urgent: usize,
    urgent_streak: usize,
}

impl WriteAdmission {
    pub(crate) fn enter(&self, urgent: bool) -> Result<WritePermit<'_>> {
        let mut state = self.state.lock().map_err(storage)?;
        if urgent {
            state.urgent += 1;
        } else {
            state.ordinary += 1;
        }
        let mut state = self
            .available
            .wait_while(state, |state| {
                let urgent_first =
                    state.urgent > 0 && (state.ordinary == 0 || state.urgent_streak < 3);
                state.busy || urgent_first != urgent
            })
            .map_err(storage)?;
        if urgent {
            state.urgent -= 1;
            state.urgent_streak = state.urgent_streak.saturating_add(1);
        } else {
            state.ordinary -= 1;
            state.urgent_streak = 0;
        }
        state.busy = true;
        Ok(WritePermit(self))
    }
}

pub(crate) struct WritePermit<'a>(&'a WriteAdmission);

impl Drop for WritePermit<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.busy = false;
            self.0.available.notify_all();
        }
    }
}
