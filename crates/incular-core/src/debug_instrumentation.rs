//! Timing for user-selected asynchronous actions.

use std::future::Future;

#[cfg(debug_assertions)]
use {crate::time::Instant, std::cell::Cell};

#[cfg(debug_assertions)]
thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
}

/// Whether this thread is collecting debug action timings.
#[must_use]
pub fn debug_instrumentation_enabled() -> bool {
    #[cfg(debug_assertions)]
    return ENABLED.with(Cell::get);
    #[cfg(not(debug_assertions))]
    false
}

/// Enables debug action timing on the current thread and returns the prior
/// setting. Release builds always leave instrumentation disabled.
pub fn set_debug_instrumentation_enabled(enabled: bool) -> bool {
    #[cfg(debug_assertions)]
    return ENABLED.with(|state| state.replace(enabled));
    #[cfg(not(debug_assertions))]
    {
        let _ = enabled;
        false
    }
}

/// Runs an asynchronous action, logging its duration when debug timing is
/// enabled. The caller chooses the debug sink, including the same sink as
/// its other debug output. Results and errors are left untouched.
///
/// Time starts when the future is first polled and includes time waiting
/// for wakes. Dropping an unfinished future normally does not report a completion.
/// An unwinding panic is logged and still propagates.
pub async fn debug_instrument_action<T, F>(
    description: &str,
    action: impl FnOnce() -> F,
    sink: impl Fn(&str),
) -> T
where
    F: Future<Output = T>,
{
    #[cfg(debug_assertions)]
    if debug_instrumentation_enabled() {
        let mut timer = ActionTimer {
            description,
            sink: &sink,
            start: Instant::now(),
            completed: false,
        };
        let result = action().await;
        timer.completed = true;
        return result;
    }

    let _ = (description, sink);
    action().await
}

#[cfg(debug_assertions)]
struct ActionTimer<'a, Sink: Fn(&str)> {
    description: &'a str,
    sink: &'a Sink,
    start: Instant,
    completed: bool,
}

#[cfg(debug_assertions)]
impl<Sink: Fn(&str)> Drop for ActionTimer<'_, Sink> {
    fn drop(&mut self) {
        let panicking = std::thread::panicking();
        if !self.completed && !panicking {
            return;
        }
        let micros = self.start.elapsed().as_micros();
        let seconds = micros / 1_000_000;
        let message = format!(
            "Action \"{}\" took {}:{:02}:{:02}.{:06}",
            self.description,
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60,
            micros % 1_000_000,
        );
        if panicking {
            // A failing diagnostic sink must not replace the action's
            // original panic or turn an unwind into a double-panic abort.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (self.sink)(&message);
            }));
        } else {
            (self.sink)(&message);
        }
    }
}
