//! Reference-preserving pointer signals for custom dispatchers.

use std::{fmt, panic::AssertUnwindSafe, rc::Rc};

use incular_core::Transform;

use crate::RawPointerEvent;

/// A wheel or similar pointer signal with a shared event identity. Widget
/// routing uses the cheaper raw value type; a custom dispatcher can use
/// this handle when it must correlate individually transformed views.
#[derive(Clone)]
pub struct PointerSignalEvent(Rc<Signal>);
struct Signal {
    raw: RawPointerEvent,
    transform: Option<Transform>,
    original: Option<PointerSignalEvent>,
    respond: Option<Rc<dyn Fn(bool)>>,
}

impl fmt::Debug for PointerSignalEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PointerSignalEvent")
            .field("raw", &self.0.raw)
            .field("transform", &self.0.transform)
            .finish_non_exhaustive()
    }
}

impl PointerSignalEvent {
    #[must_use]
    pub fn new(raw: RawPointerEvent) -> Self {
        Self::with_optional_response(raw, None)
    }

    /// Installs the native response callback, with an argument indicating
    /// whether the platform can perform its default action.
    #[must_use]
    pub fn with_response(raw: RawPointerEvent, respond: impl Fn(bool) + 'static) -> Self {
        Self::with_optional_response(raw, Some(Rc::new(respond)))
    }

    fn with_optional_response(raw: RawPointerEvent, respond: Option<Rc<dyn Fn(bool)>>) -> Self {
        Self(Rc::new(Signal {
            raw,
            transform: None,
            original: None,
            respond,
        }))
    }

    #[must_use]
    pub fn raw(&self) -> &RawPointerEvent {
        &self.0.raw
    }

    /// A transformed event links to the original signal, not midway through
    /// a chain of transforms. The original itself has no backlink.
    #[must_use]
    pub fn original(&self) -> Option<Self> {
        self.0.original.clone()
    }

    #[must_use]
    pub fn same_instance(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    #[must_use]
    pub fn transformed(&self, transform: Option<Transform>) -> Self {
        if transform.is_none() || self.0.transform == transform {
            return self.clone();
        }
        let original = self.original().unwrap_or_else(|| self.clone());
        Self(Rc::new(Signal {
            raw: original.0.raw.transformed(transform),
            transform,
            original: Some(original),
            respond: None,
        }))
    }

    fn root(&self) -> &Self {
        self.0.original.as_ref().unwrap_or(self)
    }
}

/// Why a standalone signal could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerSignalError {
    /// A different original signal already had a registration.
    DifferentSignal,
    /// A callback panicked. The resolver is ready for the next event.
    CallbackPanicked,
}

impl fmt::Display for PointerSignalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DifferentSignal => {
                f.write_str("resolve the current pointer signal before starting another")
            }
            Self::CallbackPanicked => f.write_str("the pointer signal callback panicked"),
        }
    }
}

impl std::error::Error for PointerSignalError {}

type SignalCallback = Box<dyn FnOnce(PointerSignalEvent)>;

/// One dispatch's first registered signal callback and its exact event view.
#[derive(Default)]
pub struct PointerSignalResolver {
    winner: Option<(PointerSignalEvent, SignalCallback)>,
}

impl PointerSignalResolver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The first view wins; views of the same original can also register.
    /// Returns whether this was the first registration.
    pub fn register(
        &mut self,
        view: &PointerSignalEvent,
        callback: impl FnOnce(PointerSignalEvent) + 'static,
    ) -> Result<bool, PointerSignalError> {
        if let Some((first, _)) = &self.winner {
            return if first.root().same_instance(view.root()) {
                Ok(false)
            } else {
                Err(PointerSignalError::DifferentSignal)
            };
        }
        self.winner = Some((view.clone(), Box::new(callback)));
        Ok(true)
    }

    /// Invokes the first callback with its own view. Without a claim, allows
    /// the original's default platform action and returns false.
    pub fn resolve(&mut self, event: &PointerSignalEvent) -> Result<bool, PointerSignalError> {
        if let Some((first, _)) = &self.winner
            && !first.root().same_instance(event.root())
        {
            return Err(PointerSignalError::DifferentSignal);
        }
        if let Some((first, callback)) = self.winner.take() {
            return std::panic::catch_unwind(AssertUnwindSafe(|| callback(first)))
                .map(|()| true)
                .map_err(|_| PointerSignalError::CallbackPanicked);
        }
        if let Some(respond) = &event.root().0.respond {
            respond(true);
        }
        Ok(false)
    }
}
