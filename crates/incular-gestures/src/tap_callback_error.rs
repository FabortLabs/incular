//! Error boundary for user callbacks invoked by a tap recognizer.

use std::{
    any::Any,
    cell::RefCell,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

/// Why a begun tap was cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapCancelReason {
    /// The pointer itself was cancelled before the tap won.
    Pointer,
    /// The winning tap was withdrawn, such as after movement or button changes.
    Spontaneous,
    /// The tap lost to another recognizer.
    Forced,
}

/// The tap callback that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapCallbackKind {
    Down,
    Move,
    Up,
    Tap,
    Cancel(TapCancelReason),
}

impl TapCallbackKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Down => "onTapDown",
            Self::Move => "onTapMove",
            Self::Up => "onTapUp",
            Self::Tap => "onTap",
            Self::Cancel(TapCancelReason::Pointer) => "onTapCancel",
            Self::Cancel(TapCancelReason::Spontaneous) => "spontaneous onTapCancel",
            Self::Cancel(TapCancelReason::Forced) => "forced onTapCancel",
        }
    }
}

/// A user callback panicked while processing a tap. The original payload
/// remains available for applications using a typed panic.
pub struct TapCallbackError {
    pub callback: TapCallbackKind,
    pub panic: Box<dyn Any + Send + 'static>,
}

impl TapCallbackError {
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| self.panic.downcast_ref::<&str>().copied())
    }
}

impl fmt::Debug for TapCallbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TapCallbackError")
            .field("callback", &self.callback)
            .field("message", &self.message())
            .finish_non_exhaustive()
    }
}

impl fmt::Display for TapCallbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "while invoking \"{}\": ", self.callback.name())?;
        f.write_str(self.message().unwrap_or("user callback panicked"))
    }
}

impl std::error::Error for TapCallbackError {}

pub type TapCallbackErrorHandler = Rc<dyn Fn(TapCallbackError)>;

thread_local! {
    static HANDLER: RefCell<Option<TapCallbackErrorHandler>> = const { RefCell::new(None) };
}

/// Replaces the current UI thread's tap error reporter and returns the
/// previous reporter. With unwinding panic strategies, an error is isolated
/// even when no reporter is installed; the normal panic hook still runs.
pub fn set_tap_callback_error_handler(
    handler: Option<TapCallbackErrorHandler>,
) -> Option<TapCallbackErrorHandler> {
    HANDLER.with(|current| current.replace(handler))
}

pub(crate) fn invoke(kind: TapCallbackKind, callback: impl FnOnce()) {
    let Err(panic) = catch_unwind(AssertUnwindSafe(callback)) else {
        return;
    };
    // Don't borrow the slot while invoking application code: the reporter
    // may replace itself, or encounter a failure in unrelated app code.
    if let Some(handler) = HANDLER.with(|current| current.borrow().clone()) {
        let _ = catch_unwind(AssertUnwindSafe(move || {
            handler(TapCallbackError {
                callback: kind,
                panic,
            });
        }));
    }
}

/// Uses the same error boundary when the retained arena dispatches a tap
/// from its deferred double-tap deadline or from a live callback slot.
#[doc(hidden)]
pub fn invoke_retained_tap(callback: impl FnOnce()) {
    invoke(TapCallbackKind::Tap, callback);
}
