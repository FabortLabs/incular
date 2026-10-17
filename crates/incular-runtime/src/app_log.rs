//! Application log records bound for the DevTools console.
//!
//! [`log`] is callable from any thread. With the `devtools` feature it appends
//! to a bounded process-wide queue (drop-oldest, counted) that the desktop host
//! drains on the UI thread; without the feature every call is a no-op.

use std::fmt;

/// Maximum queued records before the oldest is dropped.
pub const LOG_QUEUE_CAPACITY: usize = 1024;
/// Maximum message length in bytes; longer messages are truncated.
pub const LOG_MESSAGE_LIMIT: usize = 4096;

/// Severity of an application log record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Lowercase wire name understood by the DevTools console.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One queued application log record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRecord {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
}

/// Records drained from the queue plus how many were dropped since the last drain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogBatch {
    pub records: Vec<LogRecord>,
    pub dropped: u64,
}

/// Bounded drop-oldest queue. Public so hosts and tests can own private instances.
#[derive(Debug)]
pub struct LogQueue {
    capacity: usize,
    records: std::collections::VecDeque<LogRecord>,
    dropped: u64,
}

impl LogQueue {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            records: std::collections::VecDeque::new(),
            dropped: 0,
        }
    }

    /// Appends a record, truncating its message and dropping the oldest entry
    /// when full.
    pub fn push(&mut self, level: LogLevel, target: &str, message: &str) {
        if self.records.len() >= self.capacity {
            self.records.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.records.push_back(LogRecord {
            level,
            target: truncate(target, 256).to_owned(),
            message: truncate(message, LOG_MESSAGE_LIMIT).to_owned(),
        });
    }

    /// Takes at most `max` oldest records; the dropped counter is reported once.
    pub fn drain(&mut self, max: usize) -> LogBatch {
        let take = max.min(self.records.len());
        LogBatch {
            records: self.records.drain(..take).collect(),
            dropped: std::mem::take(&mut self.dropped),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// Truncates to at most `limit` bytes on a character boundary.
fn truncate(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(feature = "devtools")]
mod global {
    use super::{LOG_QUEUE_CAPACITY, LogBatch, LogLevel, LogQueue};
    use std::sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    };

    type Wake = Arc<dyn Fn() + Send + Sync>;

    struct Global {
        queue: Mutex<LogQueue>,
        wake: Mutex<Option<Wake>>,
        wake_pending: AtomicBool,
    }

    fn global() -> &'static Global {
        static GLOBAL: OnceLock<Global> = OnceLock::new();
        GLOBAL.get_or_init(|| Global {
            queue: Mutex::new(LogQueue::new(LOG_QUEUE_CAPACITY)),
            wake: Mutex::new(None),
            wake_pending: AtomicBool::new(false),
        })
    }

    pub fn log(level: LogLevel, target: &str, message: &str) {
        let global = global();
        if let Ok(mut queue) = global.queue.lock() {
            queue.push(level, target, message);
        }
        // Coalesce: one wake per drain, so a burst posts a single event.
        if !global.wake_pending.swap(true, Ordering::AcqRel) {
            let wake = global.wake.lock().ok().and_then(|wake| wake.clone());
            if let Some(wake) = wake {
                wake();
            }
        }
    }

    pub fn set_wake(wake: Wake) {
        let global = global();
        if let Ok(mut slot) = global.wake.lock() {
            *slot = Some(wake);
        }
        // Wake once for records logged before the host registered.
        global.wake_pending.store(false, Ordering::Release);
        if !global.queue.lock().map(|q| q.is_empty()).unwrap_or(true) {
            global.wake_pending.store(true, Ordering::Release);
            if let Some(wake) = global.wake.lock().ok().and_then(|w| w.clone()) {
                wake();
            }
        }
    }

    pub fn drain(max: usize) -> LogBatch {
        let global = global();
        // Clear before taking so a record racing the drain re-arms the wake.
        global.wake_pending.store(false, Ordering::Release);
        let (batch, remaining) = global
            .queue
            .lock()
            .map(|mut queue| (queue.drain(max), !queue.is_empty()))
            .unwrap_or_default();
        if remaining
            && !global.wake_pending.swap(true, Ordering::AcqRel)
            && let Some(wake) = global.wake.lock().ok().and_then(|w| w.clone())
        {
            wake();
        }
        batch
    }

    pub fn pending() -> bool {
        global()
            .queue
            .lock()
            .map(|q| !q.is_empty())
            .unwrap_or(false)
    }
}

/// Queues an application log record for the DevTools console. Never blocks on
/// I/O, allocates a bounded amount, and is safe from any thread. No-op without
/// the `devtools` feature.
pub fn log(level: LogLevel, target: &str, message: impl AsRef<str>) {
    #[cfg(feature = "devtools")]
    global::log(level, target, message.as_ref());
    #[cfg(not(feature = "devtools"))]
    let _ = (level, target, message);
}

/// Registers the host callback that wakes the event loop when records are
/// queued. Backend API for desktop hosts.
#[cfg(feature = "devtools")]
pub fn set_log_wake(wake: std::sync::Arc<dyn Fn() + Send + Sync>) {
    global::set_wake(wake);
}

/// Drains up to `max` queued records from the process-wide queue. Backend API
/// for desktop hosts.
#[cfg(feature = "devtools")]
#[must_use]
pub fn drain_logs(max: usize) -> LogBatch {
    global::drain(max)
}

/// True while records remain queued. Backend API for desktop hosts.
#[cfg(feature = "devtools")]
#[must_use]
pub fn logs_pending() -> bool {
    global::pending()
}
