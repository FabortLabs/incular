//! Type-erased recognizer factories used by `RawGestureDetector`.

use std::{any::Any, any::TypeId, rc::Rc};

use crate::{GestureAction, GestureDecision, PointerEvent, RawPointerEvent};

/// A stateful recognizer owned by one mounted raw gesture detector.
///
/// The trait is object-safe so a retained element can keep recognizers in a
/// type-erased map while factories still validate their concrete type during
/// updates. Implementors may override [`Self::observe_raw`] when device or
/// button metadata affects recognition; the default forwards to the legacy
/// four-field event.
pub trait GestureRecognizer: Any {
    /// Observes an event without dispatching user callbacks.
    fn observe(&mut self, event: PointerEvent) -> GestureDecision;

    /// Observes a metadata-rich raw event. The default is appropriate for
    /// recognizers whose algorithm only needs position, phase, and time.
    fn observe_raw(&mut self, event: RawPointerEvent) -> GestureDecision {
        self.observe(event.legacy())
    }

    /// Dispatches an action after arena arbitration accepts this recognizer.
    fn dispatch(&mut self, action: GestureAction);

    /// Whether this recognizer should receive a Flutter-style default win
    /// when it remains the only contender after routing the current sample.
    #[doc(hidden)]
    fn can_win_uncontested(&self, _pointer: u64) -> bool {
        false
    }

    /// Notifies the recognizer that its stream was cancelled or lost the
    /// arena. The default has no additional state to clear.
    fn cancel(&mut self) {}

    /// Applies a competing member's arena win to the named contact.
    #[doc(hidden)]
    fn reject_pointer(&mut self, _pointer: u64) {
        self.cancel();
    }

    /// Interrupts the named contact, such as when its detector unmounts.
    #[doc(hidden)]
    fn cancel_pointer(&mut self, _pointer: u64) {
        self.cancel();
    }

    /// Releases recognizer-owned resources before it is removed from a
    /// retained element. This is deliberately separate from `Drop` so a
    /// factory can observe lifecycle cleanup deterministically.
    fn dispose(&mut self) {}

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Error returned when a factory is asked to initialize the wrong concrete
/// recognizer. A malformed erased map therefore rejects safely instead of
/// panicking or invoking a closure with an invalid type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("gesture recognizer factory type mismatch (expected {expected:?}, got {actual:?})")]
pub struct GestureRecognizerFactoryError {
    pub expected: TypeId,
    pub actual: TypeId,
}

/// Erased constructor/initializer pair for one recognizer type.
pub trait GestureRecognizerFactory {
    /// The concrete recognizer type used as the stable map key.
    fn type_id(&self) -> TypeId;

    /// Creates a fresh recognizer for a mounted element.
    fn create(&self) -> Box<dyn GestureRecognizer>;

    /// Applies the current widget configuration to an existing recognizer.
    fn initialize(
        &self,
        recognizer: &mut dyn GestureRecognizer,
    ) -> Result<(), GestureRecognizerFactoryError>;
}

/// Safe, Flutter-shaped factory implementation for one concrete recognizer.
pub struct GestureRecognizerFactoryWithHandlers<T: GestureRecognizer + 'static> {
    constructor: Rc<dyn Fn() -> T>,
    initializer: Rc<dyn Fn(&mut T)>,
}

impl<T: GestureRecognizer + 'static> Clone for GestureRecognizerFactoryWithHandlers<T> {
    fn clone(&self) -> Self {
        Self {
            constructor: self.constructor.clone(),
            initializer: self.initializer.clone(),
        }
    }
}

impl<T: GestureRecognizer + 'static> GestureRecognizerFactoryWithHandlers<T> {
    #[must_use]
    pub fn new(
        constructor: impl Fn() -> T + 'static,
        initializer: impl Fn(&mut T) + 'static,
    ) -> Self {
        Self {
            constructor: Rc::new(constructor),
            initializer: Rc::new(initializer),
        }
    }

    /// Convenience constructor for a factory whose recognizer implements
    /// `Default` and needs only an initializer.
    #[must_use]
    pub fn with_initializer(initializer: impl Fn(&mut T) + 'static) -> Self
    where
        T: Default,
    {
        Self::new(T::default, initializer)
    }
}

impl<T: GestureRecognizer + 'static> GestureRecognizerFactory
    for GestureRecognizerFactoryWithHandlers<T>
{
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn create(&self) -> Box<dyn GestureRecognizer> {
        Box::new((self.constructor)())
    }

    fn initialize(
        &self,
        recognizer: &mut dyn GestureRecognizer,
    ) -> Result<(), GestureRecognizerFactoryError> {
        let expected = TypeId::of::<T>();
        if !recognizer.as_any_mut().is::<T>() {
            return Err(GestureRecognizerFactoryError {
                expected,
                // `Any::type_id` is not available through a borrowed trait
                // object without imposing a `'static` borrow on the whole
                // erased recognizer. The public error still identifies the
                // requested type, while the safe `is` check prevents an
                // invalid initializer call.
                actual: TypeId::of::<dyn GestureRecognizer>(),
            });
        }
        let recognizer = recognizer
            .as_any_mut()
            .downcast_mut::<T>()
            .expect("recognizer type checked immediately above");
        (self.initializer)(recognizer);
        Ok(())
    }
}

/// A cloneable erased factory handle suitable for widget values.
#[derive(Clone)]
pub struct ErasedGestureRecognizerFactory(Rc<dyn GestureRecognizerFactory>);

impl ErasedGestureRecognizerFactory {
    #[must_use]
    pub fn new(factory: impl GestureRecognizerFactory + 'static) -> Self {
        Self(Rc::new(factory))
    }

    #[must_use]
    pub fn type_id(&self) -> TypeId {
        // Calling `type_id` on the Rc itself instead selects
        // Any::type_id for the wrapper, making all factories the same key.
        GestureRecognizerFactory::type_id(self.0.as_ref())
    }

    pub fn create(&self) -> Box<dyn GestureRecognizer> {
        self.0.create()
    }

    pub fn initialize(
        &self,
        recognizer: &mut dyn GestureRecognizer,
    ) -> Result<(), GestureRecognizerFactoryError> {
        self.0.initialize(recognizer)
    }

    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: GestureRecognizer + 'static> From<GestureRecognizerFactoryWithHandlers<T>>
    for ErasedGestureRecognizerFactory
{
    fn from(factory: GestureRecognizerFactoryWithHandlers<T>) -> Self {
        Self::new(factory)
    }
}

impl From<Rc<dyn GestureRecognizerFactory>> for ErasedGestureRecognizerFactory {
    fn from(factory: Rc<dyn GestureRecognizerFactory>) -> Self {
        Self(factory)
    }
}

impl From<Box<dyn GestureRecognizerFactory>> for ErasedGestureRecognizerFactory {
    fn from(factory: Box<dyn GestureRecognizerFactory>) -> Self {
        Self(Rc::from(factory))
    }
}
