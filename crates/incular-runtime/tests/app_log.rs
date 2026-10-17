//! Bounded application log queue and the process-wide DevTools log path.

use incular_runtime::{LOG_MESSAGE_LIMIT, LogLevel, LogQueue};

#[test]
fn queue_drops_oldest_and_counts() {
    let mut queue = LogQueue::new(3);
    for index in 0..5 {
        queue.push(LogLevel::Info, "t", &format!("m{index}"));
    }
    assert_eq!(queue.len(), 3);
    let batch = queue.drain(10);
    assert_eq!(batch.dropped, 2);
    let messages: Vec<_> = batch.records.iter().map(|r| r.message.as_str()).collect();
    assert_eq!(messages, ["m2", "m3", "m4"]);
    assert_eq!(queue.drain(10).dropped, 0);
    assert!(queue.is_empty());
}

#[test]
fn drain_respects_max() {
    let mut queue = LogQueue::new(10);
    for _ in 0..5 {
        queue.push(LogLevel::Warn, "t", "x");
    }
    assert_eq!(queue.drain(2).records.len(), 2);
    assert_eq!(queue.len(), 3);
}

#[test]
fn long_messages_truncate_on_char_boundary() {
    let mut queue = LogQueue::new(2);
    let message = "é".repeat(LOG_MESSAGE_LIMIT);
    queue.push(LogLevel::Error, "t", &message);
    let record = queue.drain(1).records.remove(0);
    assert!(record.message.len() <= LOG_MESSAGE_LIMIT);
    assert!(record.message.chars().all(|c| c == 'é'));
    assert_eq!(record.level.as_str(), "error");
}

#[cfg(feature = "devtools")]
mod global {
    use incular_runtime::{LogLevel, drain_logs, log, logs_pending, set_log_wake};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    // The global queue is process-wide, so one test covers it sequentially.
    #[test]
    fn cross_thread_logs_wake_once_and_drain() {
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = wakes.clone();
        set_log_wake(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        let _ = drain_logs(usize::MAX);
        let handles: Vec<_> = (0..4)
            .map(|thread| {
                std::thread::spawn(move || {
                    for index in 0..10 {
                        log(LogLevel::Info, "thread", format!("{thread}-{index}"));
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert!(logs_pending());
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            1,
            "burst coalesces to one wake"
        );
        let batch = drain_logs(1000);
        assert_eq!(batch.records.len(), 40);
        assert_eq!(batch.dropped, 0);
        assert!(!logs_pending());
        log(LogLevel::Debug, "t", "again");
        assert_eq!(wakes.load(Ordering::SeqCst), 2, "drain re-arms the wake");
        let _ = drain_logs(10);
    }
}

#[cfg(not(feature = "devtools"))]
#[test]
fn log_is_a_no_op_without_devtools() {
    incular_runtime::log(LogLevel::Info, "t", "ignored");
}
