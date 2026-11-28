//! Error boundary for user callbacks invoked by tap and tap-drag recognizers.

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
    DragStart,
    DragUpdate,
    DragEnd,
    DragCancel,
    TapTrackStart,
    TapTrackReset,
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
            Self::DragStart => "onDragStart",
            Self::DragUpdate => "onDragUpdate",
            Self::DragEnd => "onDragEnd",
            Self::DragCancel => "onCancel",
            Self::TapTrackStart => "onTapTrackStart",
            Self::TapTrackReset => "onTapTrackReset",
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
pub type TapCallbackTraceHandler = Rc<dyn Fn(TapCallbackKind)>;

thread_local! {
    static HANDLER: RefCell<Option<TapCallbackErrorHandler>> = const { RefCell::new(None) };
    static TRACE: RefCell<Option<TapCallbackTraceHandler>> = const { RefCell::new(None) };
}

/// Replaces the current UI thread's tap and tap-drag reporter and returns the
/// previous reporter. With unwinding panic strategies, an error is isolated
/// even when no reporter is installed; the normal panic hook still runs.
pub fn set_tap_callback_error_handler(
    handler: Option<TapCallbackErrorHandler>,
) -> Option<TapCallbackErrorHandler> {
    HANDLER.with(|current| current.replace(handler))
}

/// Observes tap callbacks immediately before invocation on the current UI
/// thread. Tracing is off by default and independent of error reporting.
pub fn set_tap_callback_trace_handler(
    handler: Option<TapCallbackTraceHandler>,
) -> Option<TapCallbackTraceHandler> {
    TRACE.with(|current| current.replace(handler))
}

pub(crate) fn invoke(kind: TapCallbackKind, callback: impl FnOnce()) {
    if let Some(trace) = TRACE.with(|current| current.borrow().clone()) {
        let _ = catch_unwind(AssertUnwindSafe(|| trace(kind)));
    }
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
