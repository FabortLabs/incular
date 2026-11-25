//! Short UI-thread work, ordered independently from the frame callback queue.

use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    collections::BinaryHeap,
    future::{Future, poll_fn},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    task::{Poll, Waker},
};

use crate::scheduler::{Priority, transient_callback_count};

type Strategy = Rc<dyn Fn(Priority) -> bool>;

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

/// Schedule a short callback on this UI thread, returning its result when
/// the host services the queue. Higher priorities run first, and equal
/// priorities retain insertion order. Work below `Priority::ANIMATION`
/// waits while transient frame callbacks are pending by default.
///
/// The callback runs even if the result is not awaited. Use the runtime's
/// background task facilities for expensive or blocking operations.
pub fn schedule_task<T: 'static>(
    task: impl FnOnce() -> T + 'static,
    priority: Priority,
) -> impl Future<Output = Result<T, ScheduledTaskError>> {
    let completion = Rc::new(RefCell::new(Completion {
        value: None,
        waker: None,
    }));
    let run = {
        let completion = completion.clone();
        Box::new(move || {
            let result = catch_unwind(AssertUnwindSafe(task)).map_err(|panic| ScheduledTaskError {
                message: panic
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .unwrap_or("Scheduled UI task panicked")
                    .to_owned(),
            });
            let waker = {
                let mut completion = completion.borrow_mut();
                completion.value = Some(result);
                completion.waker.take()
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        })
    };
    let sequence = NEXT_SEQUENCE.with(|counter| {
        let next = counter.get();
        counter.set(next.checked_add(1).expect("UI task sequence exhausted"));
        next
    });
    TASKS.with(|tasks| {
        tasks.borrow_mut().push(Entry {
            priority,
            sequence,
            run,
        })
    });

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
    ready_head().is_some()
}

/// Run at most one eligible task. Returns whether any tasks remain,
/// including tasks whose priority is currently blocked. Runtime hosts
/// use this to let input and rendering yield between tasks.
#[must_use]
pub fn run_one_scheduled_task() -> bool {
    if let Some(sequence) = ready_head() {
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
    TASKS.with(|tasks| !tasks.borrow().is_empty())
}
