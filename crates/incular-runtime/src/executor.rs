//! Target executor behind [`TaskScheduler`](crate::tasks::TaskScheduler).
//!
//! Native targets run application work on a lazily started multi-thread Tokio
//! runtime. Browser targets have one thread and an event loop owned by the
//! page, so futures run as `spawn_local` microtasks and "blocking" work runs
//! on that same thread after the current UI turn. Both backends report every
//! outcome through the same completion callback; neither owns Incular task
//! lifetime metadata.

use std::{future::Future, time::Duration};

use crate::tasks::TaskFailure;

pub(crate) type Completion<T> = Box<dyn FnOnce(Result<T, TaskFailure>) + Send>;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{AbortHandle, Executor};
#[cfg(target_arch = "wasm32")]
pub(crate) use web::{AbortHandle, Executor};

/// Completes after `duration` on every target: a Tokio timer natively and a
/// `setTimeout` in browsers.
///
/// Await it inside work started through the application's spawner (for
/// example [`RuntimeSpawner::spawn_into`](crate::RuntimeSpawner::spawn_into));
/// natively it needs the Tokio runtime those tasks run on.
pub async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    web::sleep(duration).await;
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{Completion, Duration, Future, TaskFailure};
    use tokio::runtime::{Builder, Handle, Runtime};

    pub(crate) use tokio::task::AbortHandle;

    const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(100);

    pub(crate) struct Executor {
        /// Started by the first task: most windows never spawn async work,
        /// and a multi-thread runtime parks one worker thread per core.
        runtime: std::cell::OnceCell<Runtime>,
        /// Handle of the shut-down runtime; later spawns resolve as stopped
        /// instead of starting a new runtime.
        stopped: Option<Handle>,
    }

    impl Executor {
        pub(crate) fn new() -> Self {
            Self {
                runtime: std::cell::OnceCell::new(),
                stopped: None,
            }
        }

        pub(crate) fn handle(&self) -> Handle {
            if let Some(stopped) = &self.stopped {
                return stopped.clone();
            }
            self.runtime
                .get_or_init(|| {
                    Builder::new_multi_thread()
                        .enable_time()
                        .enable_io()
                        .build()
                        .expect("Tokio runtime construction")
                })
                .handle()
                .clone()
        }

        pub(crate) fn spawn<F, T>(&self, future: F, complete: Completion<T>) -> AbortHandle
        where
            F: Future<Output = T> + Send + 'static,
            T: Send + 'static,
        {
            let handle = self.handle();
            let join = handle.spawn(future);
            let abort = join.abort_handle();
            handle.spawn(async move { complete(join_outcome(join.await)) });
            abort
        }

        pub(crate) fn spawn_blocking<T>(
            &self,
            work: impl FnOnce() -> T + Send + 'static,
            complete: Completion<T>,
        ) -> AbortHandle
        where
            T: Send + 'static,
        {
            let handle = self.handle();
            let join = handle.spawn_blocking(work);
            let abort = join.abort_handle();
            handle.spawn(async move { complete(join_outcome(join.await)) });
            abort
        }

        pub(crate) fn shutdown(&mut self) {
            // A runtime that never started is replaced by a thread-free one so
            // spawns after shutdown keep resolving as stopped.
            let runtime = self.runtime.take().unwrap_or_else(|| {
                Builder::new_current_thread()
                    .build()
                    .expect("Tokio runtime construction")
            });
            self.stopped = Some(runtime.handle().clone());
            // Tokio does not force-stop an already-running blocking closure.
            // The bounded wait keeps UI shutdown from hanging indefinitely.
            runtime.shutdown_timeout(SHUTDOWN_TIMEOUT);
        }
    }

    fn join_outcome<T>(result: Result<T, tokio::task::JoinError>) -> Result<T, TaskFailure> {
        match result {
            Ok(value) => Ok(value),
            Err(error) if error.is_cancelled() => Err(TaskFailure::Cancelled),
            Err(error) if error.is_panic() => Err(TaskFailure::Panicked),
            Err(_) => Err(TaskFailure::RuntimeStopped),
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::{
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        task::{Context, Poll, Waker},
    };

    use super::{Completion, Duration, Future, TaskFailure};
    use wasm_bindgen::{JsCast, prelude::*};

    /// Cancels a browser task before its next poll. Unlike Tokio, the browser
    /// executor cannot preempt a running microtask, so cancellation of
    /// "blocking" work that already started discards only its result.
    #[derive(Clone)]
    pub(crate) struct AbortHandle {
        aborted: Arc<AtomicBool>,
        waker: Arc<std::sync::Mutex<Option<Waker>>>,
    }

    impl AbortHandle {
        pub(crate) fn abort(&self) {
            self.aborted.store(true, Ordering::Release);
            if let Some(waker) = self.waker.lock().expect("abort waker mutex").take() {
                waker.wake();
            }
        }
    }

    struct Abortable<F> {
        future: Pin<Box<F>>,
        handle: AbortHandle,
    }

    impl<F: Future> Future for Abortable<F> {
        type Output = Result<F::Output, TaskFailure>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.handle.aborted.load(Ordering::Acquire) {
                return Poll::Ready(Err(TaskFailure::Cancelled));
            }
            *self.handle.waker.lock().expect("abort waker mutex") = Some(cx.waker().clone());
            self.future.as_mut().poll(cx).map(Ok)
        }
    }

    pub(crate) struct Executor {
        stopped: bool,
    }

    impl Executor {
        pub(crate) const fn new() -> Self {
            Self { stopped: false }
        }

        pub(crate) fn spawn<F, T>(&self, future: F, complete: Completion<T>) -> AbortHandle
        where
            F: Future<Output = T> + Send + 'static,
            T: Send + 'static,
        {
            let handle = AbortHandle {
                aborted: Arc::new(AtomicBool::new(self.stopped)),
                waker: Arc::new(std::sync::Mutex::new(None)),
            };
            let abortable = Abortable {
                future: Box::pin(future),
                handle: handle.clone(),
            };
            wasm_bindgen_futures::spawn_local(async move { complete(abortable.await) });
            handle
        }

        pub(crate) fn spawn_blocking<T>(
            &self,
            work: impl FnOnce() -> T + Send + 'static,
            complete: Completion<T>,
        ) -> AbortHandle
        where
            T: Send + 'static,
        {
            // There is no blocking pool in a browser. Deferring to a microtask
            // keeps the call non-reentrant: the work runs after this UI turn.
            self.spawn(async move { work() }, complete)
        }

        pub(crate) fn shutdown(&mut self) {
            self.stopped = true;
        }
    }

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = setTimeout)]
        fn set_timeout(handler: &js_sys::Function, timeout: i32) -> JsValue;
    }

    #[derive(Default)]
    struct TimerState {
        fired: bool,
        waker: Option<Waker>,
    }

    /// A `setTimeout` future. It holds no JavaScript values, so it stays
    /// `Send` like the Tokio timer it replaces, and it uses the global scope
    /// so it also works in workers, where there is no `window`.
    struct Sleep {
        millis: i32,
        armed: bool,
        state: Arc<std::sync::Mutex<TimerState>>,
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            let mut state = self.state.lock().expect("timer state mutex");
            if state.fired {
                return Poll::Ready(());
            }
            state.waker = Some(cx.waker().clone());
            drop(state);
            if !self.armed {
                self.armed = true;
                let shared = self.state.clone();
                let fire = Closure::once_into_js(move || {
                    let mut state = shared.lock().expect("timer state mutex");
                    state.fired = true;
                    if let Some(waker) = state.waker.take() {
                        waker.wake();
                    }
                });
                let _ = set_timeout(fire.unchecked_ref(), self.millis);
            }
            Poll::Pending
        }
    }

    pub(super) async fn sleep(duration: Duration) {
        Sleep {
            millis: i32::try_from(duration.as_millis()).unwrap_or(i32::MAX),
            armed: false,
            state: Arc::default(),
        }
        .await;
    }
}
