//! An already-available value that also supports asynchronous callers.

use std::{
    future::{Future, IntoFuture, Ready, poll_fn, ready},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_core::Stream;

/// Flutter's synchronous-future boundary: a continuation can run inline
/// even when callers may also await the value. Rust consumes each handle;
/// clone it first to use a cloneable value in more than one continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SynchronousFuture<T>(T);

impl<T> SynchronousFuture<T> {
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Runs an immediate continuation on this call stack.
    pub fn then<R>(self, callback: impl FnOnce(T) -> R) -> SynchronousFuture<R> {
        SynchronousFuture(callback(self.0))
    }

    /// Starts the callback immediately and returns its future unchanged.
    pub fn then_async<F: Future>(self, callback: impl FnOnce(T) -> F) -> F {
        callback(self.0)
    }

    /// Unlike an inline continuation, a timeout wrapper publishes after an
    /// asynchronous turn. The already available value always beats the timer.
    pub async fn timeout(self, _time_limit: Duration) -> T {
        let mut deferred = false;
        poll_fn(|cx| {
            if std::mem::replace(&mut deferred, true) {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
        self.0
    }

    /// Delivers exactly one value asynchronously and then closes the stream.
    pub fn into_stream(self) -> impl Stream<Item = T> {
        SingleValueStream {
            value: Some(self.0),
            deferred: false,
        }
    }

    /// Calls the action immediately. Waits for it, preserves this future's
    /// own value on success, and propagates the action's error.
    pub fn when_complete<F, U, E>(
        self,
        action: impl FnOnce() -> Result<F, E>,
    ) -> impl Future<Output = Result<T, E>>
    where
        F: Future<Output = Result<U, E>>,
    {
        let action = action();
        async move {
            action?.await?;
            Ok(self.0)
        }
    }
}

impl<T> IntoFuture for SynchronousFuture<T> {
    type Output = T;
    type IntoFuture = Ready<T>;

    fn into_future(self) -> Self::IntoFuture {
        ready(self.0)
    }
}

struct SingleValueStream<T> {
    value: Option<T>,
    deferred: bool,
}

// The stream never projects a pinned reference to its value.
impl<T> Unpin for SingleValueStream<T> {}

impl<T> Stream for SingleValueStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        if !self.deferred {
            self.deferred = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(self.value.take())
    }
}
