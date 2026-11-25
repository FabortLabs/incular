//! Short UI-thread work, ordered independently from the frame callback queue.

use std::{
    any::Any,
    cell::{Cell, OnceCell, RefCell},
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, VecDeque},
    future::{Future, poll_fn},
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    },
    task::{Context, Poll, Wake, Waker},
};

use crate::scheduler::{Priority, transient_callback_count};

type Strategy = Rc<dyn Fn(Priority) -> bool>;
type HostWake = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
struct AsyncInbox {
    ready: Mutex<VecDeque<u64>>,
    has_ready: AtomicBool,
    host_wake: Mutex<Option<HostWake>>,
}

impl AsyncInbox {
    fn push(&self, signal: &AsyncSignal) {
        let notify = {
            let mut ready = self.ready.lock().expect("UI task inbox mutex");
            if signal.finished.load(AtomicOrdering::Acquire) {
                return;
            }
            let was_empty = ready.is_empty();
            ready.push_back(signal.id);
            self.has_ready.store(true, AtomicOrdering::Release);
            was_empty
        };
        if notify {
            self.notify_host();
        }
    }

    fn pop(&self) -> Option<u64> {
        if !self.has_ready.load(AtomicOrdering::Acquire) {
            return None;
        }
        let mut ready = self.ready.lock().expect("UI task inbox mutex");
        let id = ready.pop_front();
        self.has_ready
            .store(!ready.is_empty(), AtomicOrdering::Release);
        id
    }

    fn remove_finished(&self, id: u64) {
        let mut ready = self.ready.lock().expect("UI task inbox mutex");
        ready.retain(|queued| *queued != id);
        self.has_ready
            .store(!ready.is_empty(), AtomicOrdering::Release);
    }

    fn notify_host(&self) {
        let wake = self.host_wake.lock().expect("UI task wake mutex").clone();
        if let Some(wake) = wake {
            wake();
        }
    }
}

struct AsyncSignal {
    id: u64,
    inbox: Arc<AsyncInbox>,
    queued: AtomicBool,
    finished: AtomicBool,
}

impl Wake for AsyncSignal {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if !self.finished.load(AtomicOrdering::Acquire)
            && !self.queued.swap(true, AtomicOrdering::AcqRel)
        {
            self.inbox.push(self);
        }
    }
}

struct AsyncEntry {
    // Only the UI thread owns or polls the future. Worker-thread wakers carry
    // the task ID and inbox, never the possibly !Send future itself.
    future: Pin<Box<dyn Future<Output = ()>>>,
    signal: Arc<AsyncSignal>,
}

struct Entry {
    priority: Priority,
    sequence: u64,
    run: Box<dyn FnOnce()>,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

thread_local! {
    static TASKS: RefCell<BinaryHeap<Entry>> = const { RefCell::new(BinaryHeap::new()) };
    static STRATEGY: RefCell<Option<Strategy>> = const { RefCell::new(None) };
    static NEXT_SEQUENCE: Cell<u64> = const { Cell::new(0) };
    static ASYNC_INBOX: OnceCell<Arc<AsyncInbox>> = const { OnceCell::new() };
    static ASYNC_TASKS: RefCell<HashMap<u64, AsyncEntry>> = RefCell::new(HashMap::new());
    static INSIDE_ASYNC_POLL: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A scheduled UI task panicked before returning a result.
pub struct ScheduledTaskError {
    message: String,
}

impl std::fmt::Display for ScheduledTaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ScheduledTaskError {}

struct Completion<T> {
    value: Option<Result<T, ScheduledTaskError>>,
    waker: Option<Waker>,
}

type TaskCompletion<T> = Rc<RefCell<Completion<T>>>;

fn new_completion<T>() -> TaskCompletion<T> {
    Rc::new(RefCell::new(Completion {
        value: None,
        waker: None,
    }))
}

fn complete<T>(completion: &TaskCompletion<T>, value: Result<T, ScheduledTaskError>) {
    let waker = {
        let mut completion = completion.borrow_mut();
        completion.value = Some(value);
        completion.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

fn result_future<T>(
    completion: TaskCompletion<T>,
) -> impl Future<Output = Result<T, ScheduledTaskError>> {
    poll_fn(move |context| {
        let mut completion = completion.borrow_mut();
        if let Some(result) = completion.value.take() {
            return Poll::Ready(result);
        }
        if completion
            .waker
            .as_ref()
            .is_none_or(|waker| !waker.will_wake(context.waker()))
        {
            completion.waker = Some(context.waker().clone());
        }
        Poll::Pending
    })
}

fn panicked(payload: Box<dyn Any + Send>) -> ScheduledTaskError {
    ScheduledTaskError {
        message: payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("Scheduled UI task panicked")
            .to_owned(),
    }
}

fn next_sequence() -> u64 {
    NEXT_SEQUENCE.with(|counter| {
        let next = counter.get();
        counter.set(next.checked_add(1).expect("UI task sequence exhausted"));
        next
    })
}

fn enqueue(priority: Priority, sequence: u64, run: impl FnOnce() + 'static) {
    TASKS.with(|tasks| {
        tasks.borrow_mut().push(Entry {
            priority,
            sequence,
            run: Box::new(run),
        });
    });
}

/// Schedule a short callback on this UI thread, returning its result when
/// the host services the queue. Higher priorities run first, and equal
/// priorities retain insertion order. Work below `Priority::ANIMATION`
/// waits while transient frame callbacks are pending by default.
///
/// The callback runs even if the result is not awaited. Use the runtime's
/// background task facilities for expensive or blocking operations. Use
/// [`schedule_async_task`] to wait for a callback's returned future.
pub fn schedule_task<T: 'static>(
    task: impl FnOnce() -> T + 'static,
    priority: Priority,
) -> impl Future<Output = Result<T, ScheduledTaskError>> {
    let completion = new_completion();
    enqueue(priority, next_sequence(), {
        let completion = completion.clone();
        move || {
            complete(
                &completion,
                catch_unwind(AssertUnwindSafe(task)).map_err(panicked),
            )
        }
    });
    result_future(completion)
}

/// Schedule a future-returning callback on the current UI thread. The result
/// resolves only when the inner future finishes, and dropping the result
/// handle does not cancel the task. Priority gates the initial callback;
/// future continuations resume on a wake without reentering that gate.
///
/// The future may borrow UI-thread-owned state through `Rc`, but all of its
/// polls must be inexpensive and non-blocking.
pub fn schedule_async_task<T: 'static, F: Future<Output = T> + 'static>(
    task: impl FnOnce() -> F + 'static,
    priority: Priority,
) -> impl Future<Output = Result<T, ScheduledTaskError>> {
    let inbox = ASYNC_INBOX.with(|inbox| inbox.get_or_init(Arc::default).clone());
    let id = next_sequence();
    let signal = Arc::new(AsyncSignal {
        id,
        inbox: inbox.clone(),
        queued: AtomicBool::new(true),
        finished: AtomicBool::new(false),
    });
    let completion = new_completion();
    let future = {
        let completion = completion.clone();
        Box::pin(async move {
            let mut work = std::pin::pin!(async move { task().await });
            let result = poll_fn(|context| {
                match catch_unwind(AssertUnwindSafe(|| work.as_mut().poll(context))) {
                    Ok(Poll::Pending) => Poll::Pending,
                    Ok(Poll::Ready(value)) => Poll::Ready(Ok(value)),
                    Err(payload) => Poll::Ready(Err(panicked(payload))),
                }
            })
            .await;
            complete(&completion, result);
        })
    };
    enqueue(priority, id, move || {
        ASYNC_TASKS.with(|tasks| {
            tasks.borrow_mut().insert(id, AsyncEntry { future, signal });
        });
        poll_async(id);
    });
    inbox.notify_host();
    result_future(completion)
}

/// Install the native host wake hook for this scheduler thread.
#[doc(hidden)]
pub fn set_scheduled_task_wake(wake: impl Fn() + Send + Sync + 'static) {
    ASYNC_INBOX.with(|inbox| {
        let inbox = inbox.get_or_init(Arc::default);
        *inbox.host_wake.lock().expect("UI task wake mutex") = Some(Arc::new(wake));
        if inbox.has_ready.load(AtomicOrdering::Acquire) || has_ready_scheduled_task() {
            inbox.notify_host();
        }
    });
}

struct PollGuard(bool);

impl Drop for PollGuard {
    fn drop(&mut self) {
        INSIDE_ASYNC_POLL.set(self.0);
    }
}

fn poll_async(id: u64) {
    let Some(mut entry) = ASYNC_TASKS.with(|tasks| tasks.borrow_mut().remove(&id)) else {
        return;
    };
    let _guard = PollGuard(INSIDE_ASYNC_POLL.replace(true));
    entry.signal.queued.store(false, AtomicOrdering::Release);
    let waker = Waker::from(entry.signal.clone());
    match entry.future.as_mut().poll(&mut Context::from_waker(&waker)) {
        Poll::Pending => {
            ASYNC_TASKS.with(|tasks| tasks.borrow_mut().insert(id, entry));
        }
        Poll::Ready(()) => {
            entry.signal.finished.store(true, AtomicOrdering::Release);
            if entry.signal.queued.load(AtomicOrdering::Acquire) {
                entry.signal.inbox.remove_finished(id);
            }
        }
    }
}

fn has_ready_async() -> bool {
    !INSIDE_ASYNC_POLL.get()
        && ASYNC_INBOX.with(|inbox| {
            inbox
                .get()
                .is_some_and(|queue| queue.has_ready.load(AtomicOrdering::Acquire))
        })
}

fn take_ready_async() -> Option<u64> {
    // A callback can service the synchronous queue recursively. Do not drain
    // wake signals until the in-flight task is back in the local registry.
    if INSIDE_ASYNC_POLL.get() {
        return None;
    }
    ASYNC_INBOX.with(|inbox| inbox.get().and_then(|queue| queue.pop()))
}

/// Choose whether the highest-priority waiting task may run on this thread.
/// The strategy should be cheap and must not reorder or enqueue work while
/// deciding. Lower priorities cannot pass a blocked head task.
pub fn set_scheduling_strategy(strategy: impl Fn(Priority) -> bool + 'static) {
    let previous = STRATEGY.with(|current| current.replace(Some(Rc::new(strategy))));
    drop(previous);
}

/// Restore Flutter's default animation-aware scheduling strategy.
pub fn reset_scheduling_strategy() {
    let previous = STRATEGY.with(|current| current.take());
    drop(previous);
}

fn ready_head() -> Option<u64> {
    let (sequence, priority) = TASKS.with(|tasks| {
        tasks
            .borrow()
            .peek()
            .map(|entry| (entry.sequence, entry.priority))
    })?;
    let strategy = STRATEGY.with(|current| current.borrow().clone());
    let allowed = match strategy {
        Some(strategy) => strategy(priority),
        None => transient_callback_count() == 0 || priority >= Priority::ANIMATION,
    };
    allowed.then_some(sequence)
}

/// Whether at least one scheduled UI task may run now.
#[must_use]
pub fn has_ready_scheduled_task() -> bool {
    has_ready_async() || ready_head().is_some()
}

/// Run at most one eligible task. Returns whether any tasks remain,
/// including tasks whose priority is currently blocked. An asynchronous task
/// that is waiting for a wake does not keep this queue active. Runtime hosts
/// use this to let input and rendering yield between tasks.
#[must_use]
pub fn run_one_scheduled_task() -> bool {
    if let Some(id) = take_ready_async() {
        poll_async(id);
    } else if let Some(sequence) = ready_head() {
        let entry = TASKS.with(|tasks| {
            let mut tasks = tasks.borrow_mut();
            if tasks.peek().is_some_and(|entry| entry.sequence == sequence) {
                tasks.pop()
            } else {
                None
            }
        });
        if let Some(entry) = entry {
            (entry.run)();
        }
    }
    has_ready_async() || TASKS.with(|tasks| !tasks.borrow().is_empty())
}
