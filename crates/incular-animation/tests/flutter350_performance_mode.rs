#![cfg(debug_assertions)]

use std::{cell::RefCell, rc::Rc};

use incular_animation::{
    PerformanceMode, debug_assert_no_pending_performance_mode_requests,
    debug_requested_performance_mode, request_performance_mode,
};
use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};

#[test]
fn a_single_request_sets_and_releases_the_mode() {
    assert_eq!(debug_requested_performance_mode(), None);
    let handle = request_performance_mode(PerformanceMode::Latency).unwrap();
    assert_eq!(
        debug_requested_performance_mode(),
        Some(PerformanceMode::Latency)
    );
    handle.dispose();
    assert_eq!(debug_requested_performance_mode(), None);
    assert!(debug_assert_no_pending_performance_mode_requests(
        "test should be clean"
    ));
}

#[test]
fn a_conflicting_request_cannot_replace_or_remove_the_accepted_mode() {
    let handle = request_performance_mode(PerformanceMode::Latency).unwrap();
    assert!(request_performance_mode(PerformanceMode::Throughput).is_none());
    assert_eq!(
        debug_requested_performance_mode(),
        Some(PerformanceMode::Latency)
    );
    handle.dispose();
    assert_eq!(debug_requested_performance_mode(), None);
}

#[test]
fn only_the_last_compatible_request_reverts() {
    let a = request_performance_mode(PerformanceMode::Latency).unwrap();
    let b = request_performance_mode(PerformanceMode::Latency).unwrap();
    assert_eq!(
        debug_requested_performance_mode(),
        Some(PerformanceMode::Latency)
    );
    a.dispose();
    assert_eq!(
        debug_requested_performance_mode(),
        Some(PerformanceMode::Latency)
    );
    b.dispose();
    assert_eq!(debug_requested_performance_mode(), None);
    let memory = request_performance_mode(PerformanceMode::Memory).unwrap();
    let error = std::panic::catch_unwind(|| {
        debug_assert_no_pending_performance_mode_requests("held a performance request");
    });
    assert!(error.is_err());
    drop(memory);
    assert!(debug_assert_no_pending_performance_mode_requests(
        "test should be clean"
    ));
}

#[test]
fn request_handle_dispatches_memory_events_without_retaining_the_request() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let _listener = MemoryAllocations::current().add_listener({
        let events = events.clone();
        move |event| events.borrow_mut().push(event)
    });
    let handle = request_performance_mode(PerformanceMode::Latency).unwrap();
    assert!(request_performance_mode(PerformanceMode::Memory).is_none());
    handle.dispose();
    let trace = events.borrow();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].kind, ObjectEventKind::Created);
    assert_eq!(trace[1].kind, ObjectEventKind::Disposed);
    assert_eq!(trace[0].id, trace[1].id);
    assert_eq!(trace[0].class_name, "PerformanceModeRequestHandle");
    assert_eq!(debug_requested_performance_mode(), None);
}
