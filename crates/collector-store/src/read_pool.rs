//! Bounded leases over separate, read-only SQLite connections.
use crate::{Connection, Error, Path, Result, StdMutex, storage};
use rusqlite::OpenFlags;
use std::{ops::Deref, sync::Condvar, time::Duration};

pub(crate) const CONNECTIONS: usize = 4;
const WAIT: Duration = Duration::from_millis(200);

pub(crate) struct ReadPool {
    idle: StdMutex<Vec<Connection>>,
    available: Condvar,
}

impl ReadPool {
    pub(crate) fn open(state: &Path) -> Result<Self> {
        let mut idle = Vec::with_capacity(CONNECTIONS);
        for _ in 0..CONNECTIONS {
            let db = Connection::open_with_flags(
                state.join("catalog.sqlite"),
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(storage)?;
            db.busy_timeout(WAIT).map_err(storage)?;
            // ATTACH inherits the read-only open flags of the main connection.
            db.execute(
                "ATTACH DATABASE ? AS private",
                [state.join("private.sqlite").to_string_lossy().as_ref()],
            )
            .map_err(storage)?;
            if !db.is_readonly("main").map_err(storage)?
                || !db.is_readonly("private").map_err(storage)?
            {
                return Err(Error::Storage("catalog read connection is writable".into()));
            }
            db.pragma_update(None, "query_only", true)
                .map_err(storage)?;
            idle.push(db);
        }
        Ok(Self {
            idle: StdMutex::new(idle),
            available: Condvar::new(),
        })
    }

    pub(crate) fn acquire(&self) -> Result<ReadLease<'_>> {
        let idle = self.idle.lock().map_err(storage)?;
        let (mut idle, _) = self
            .available
            .wait_timeout_while(idle, WAIT, |idle| idle.is_empty())
            .map_err(storage)?;
        Ok(ReadLease {
            pool: self,
            db: Some(idle.pop().ok_or(Error::CatalogBusy)?),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_each(&self, visit: impl Fn(&mut Connection)) {
        let mut idle = self.idle.lock().unwrap();
        assert_eq!(idle.len(), CONNECTIONS);
        idle.iter_mut().for_each(visit);
    }
}

pub(crate) struct ReadLease<'a> {
    pool: &'a ReadPool,
    db: Option<Connection>,
}

impl Deref for ReadLease<'_> {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        self.db.as_ref().expect("live read lease")
    }
}

impl Drop for ReadLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut idle) = self.pool.idle.lock() {
            idle.push(self.db.take().expect("live read lease"));
            self.pool.available.notify_one();
        }
    }
}
