use std::{
    cell::Cell,
    future::{Future, IntoFuture, Ready, ready},
    pin::pin,
    task::{Context, Poll, Waker},
    time::Duration,
};

use futures_core::Stream;
use incular_core::SynchronousFuture;

#[test]
fn a_synchronous_future_runs_continuations_inline_but_timeout_and_stream_wait_a_turn() {
    let future = SynchronousFuture::new(42);
    let result = Cell::new(None);
    let _ = future.then(|value| result.set(Some(value)));
    assert_eq!(result.get(), Some(42));
    result.set(None);

    let mut cx = Context::from_waker(Waker::noop());
    let mut timeout = pin!(async {
        result.set(Some(future.timeout(Duration::from_millis(1)).await));
    });
    assert!(timeout.as_mut().poll(&mut cx).is_pending());
    assert_eq!(result.get(), None);
    assert!(timeout.as_mut().poll(&mut cx).is_ready());
    assert_eq!(result.get(), Some(42));

    let mut stream = pin!(future.into_stream());
    assert!(stream.as_mut().poll_next(&mut cx).is_pending());
    assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(Some(42)));
    assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(None));
}

#[test]
fn completion_preserves_the_original_value_and_propagates_errors() {
    let ran = Cell::new(false);
    let future = SynchronousFuture::new(42);
    let mut complete = pin!(future.when_complete(|| {
        ran.set(true);
        Ok::<_, &str>(ready(Ok(31)))
    }));
    assert!(ran.get());
    let mut cx = Context::from_waker(Waker::noop());
    assert_eq!(complete.as_mut().poll(&mut cx), Poll::Ready(Ok(42)));

    let mut failure = pin!(
        future.when_complete(|| -> Result<Ready<Result<(), &str>>, _> { Err("invalid argument") })
    );
    assert_eq!(
        failure.as_mut().poll(&mut cx),
        Poll::Ready(Err("invalid argument"))
    );
    let mut later_failure = pin!(future.when_complete(|| Ok(ready(Err::<(), _>("async failure")))));
    assert_eq!(
        later_failure.as_mut().poll(&mut cx),
        Poll::Ready(Err("async failure"))
    );

    let mut done = pin!(future.into_future());
    assert_eq!(done.as_mut().poll(&mut cx), Poll::Ready(42));
}
