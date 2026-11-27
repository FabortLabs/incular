use std::{
    future::{Future, ready},
    pin::{Pin, pin},
    task::{Context, Poll, Waker},
};

use incular_devtools::preview_execution::{PreviewExecution, PreviewExecutionKind};
use tokio::sync::oneshot;

use PreviewExecutionKind::{HotReload, HotRestart};

fn poll<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn startup_edits_are_gated_bursts_are_coalesced_and_restart_wins() {
    let mut execution = PreviewExecution::new();
    let requests = execution.requests();
    let trace = std::cell::RefCell::new(Vec::new());
    let immediate = |kind| {
        trace.borrow_mut().push(kind);
        ready(kind)
    };

    assert!(requests.request(HotReload));
    {
        let mut startup = pin!(execution.perform_next(immediate));
        assert!(poll(startup.as_mut()).is_pending());
        assert!(trace.borrow().is_empty());
        assert!(requests.connected());
        assert_eq!(poll(startup.as_mut()), Poll::Ready(Some(HotReload)));
        assert!(!requests.connected());
    }

    assert!(requests.request(HotReload));
    {
        let (complete, ack) = oneshot::channel();
        let mut in_flight = pin!(execution.perform_next(|kind| {
            trace.borrow_mut().push(kind);
            async { ack.await.unwrap() }
        }));
        assert!(poll(in_flight.as_mut()).is_pending());
        assert_eq!(trace.borrow().as_slice(), [HotReload, HotReload]);
        for _ in 0..3 {
            assert!(requests.request(HotReload));
        }
        assert!(poll(in_flight.as_mut()).is_pending());
        assert_eq!(trace.borrow().len(), 2);
        complete.send(()).unwrap();
        assert_eq!(poll(in_flight.as_mut()), Poll::Ready(Some(())));
    }
    {
        let mut next = pin!(execution.perform_next(immediate));
        assert_eq!(poll(next.as_mut()), Poll::Ready(Some(HotReload)));
    }
    {
        let mut no_duplicate = pin!(execution.perform_next(immediate));
        assert!(poll(no_duplicate.as_mut()).is_pending());
        assert_eq!(trace.borrow().len(), 3);
    }

    assert!(requests.request(HotReload));
    {
        let (complete, ack) = oneshot::channel();
        let mut in_flight = pin!(execution.perform_next(|kind| {
            trace.borrow_mut().push(kind);
            async { ack.await.unwrap() }
        }));
        assert!(poll(in_flight.as_mut()).is_pending());
        assert_eq!(trace.borrow().len(), 4);
        assert!(requests.request(HotReload));
        assert!(requests.request(HotRestart));
        assert!(requests.request(HotReload));
        assert!(poll(in_flight.as_mut()).is_pending());
        assert_eq!(trace.borrow().len(), 4);
        complete.send(()).unwrap();
        assert_eq!(poll(in_flight.as_mut()), Poll::Ready(Some(())));
    }
    {
        let mut restart = pin!(execution.perform_next(immediate));
        assert_eq!(poll(restart.as_mut()), Poll::Ready(Some(HotRestart)));
    }
    requests.stop();
    assert!(!requests.request(HotReload));
    let mut stopped = pin!(execution.perform_next(immediate));
    assert_eq!(poll(stopped.as_mut()), Poll::Ready(None));
    assert_eq!(
        trace.take(),
        [HotReload, HotReload, HotReload, HotReload, HotRestart]
    );
}

#[test]
fn closing_before_connection_drops_pending_work_without_touching_the_host() {
    let mut execution = PreviewExecution::new();
    let requests = execution.requests();
    assert!(requests.request(HotRestart));
    let mut pending = pin!(execution.perform_next(|_| -> std::future::Ready<()> {
        panic!("a disconnected or stopped host must not be called");
    }));
    assert!(poll(pending.as_mut()).is_pending());
    requests.stop();
    assert!(!requests.connected());
    assert_eq!(poll(pending.as_mut()), Poll::Ready(None));
}

#[test]
fn a_gate_dropped_by_its_host_invalidates_existing_producers() {
    let execution = PreviewExecution::new();
    let requests = execution.requests();
    assert!(requests.connected());
    drop(execution);
    assert!(!requests.request(HotReload));
}
