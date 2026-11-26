//! Completion of one driven scroll request, independent of later requests.

use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use crate::controller::ScrollAnimation;

#[derive(Debug, Default)]
struct State {
    done: bool,
    waker: Option<Waker>,
}

/// Resolves when one driven scroll animation stops, including when another
/// activity interrupts it. Like Flutter's scroll future, completion does
/// not imply the target was reached.
#[derive(Debug)]
#[must_use = "await the completion to observe when this scroll request ends"]
pub struct ScrollAnimationCompletion(Option<Rc<RefCell<State>>>);

impl ScrollAnimationCompletion {
    /// A request that completed immediately, with no driven animation.
    pub const fn ready() -> Self {
        Self(None)
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.0.as_ref().is_none_or(|state| state.borrow().done)
    }

    pub(crate) fn pending() -> (Self, ScrollAnimationSignal) {
        let state = Rc::new(RefCell::new(State::default()));
        (Self(Some(state.clone())), ScrollAnimationSignal(state))
    }
}

impl Future for ScrollAnimationCompletion {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        let Some(state) = &self.0 else {
            return Poll::Ready(());
        };
        let mut state = state.borrow_mut();
        if state.done {
            return Poll::Ready(());
        }
        if state
            .waker
            .as_ref()
            .is_none_or(|old| !old.will_wake(context.waker()))
        {
            state.waker = Some(context.waker().clone());
        }
        Poll::Pending
    }
}

/// Owned by a pending request and then by exactly its driver. Dropping
/// either completes the request on normal finish, takeover, or teardown.
#[doc(hidden)]
pub struct ScrollAnimationSignal(Rc<RefCell<State>>);

impl Drop for ScrollAnimationSignal {
    fn drop(&mut self) {
        let waker = {
            let mut state = self.0.borrow_mut();
            state.done = true;
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

pub(crate) struct PendingScrollAnimation {
    pub(crate) motion: ScrollAnimation,
    pub(crate) completion: Option<ScrollAnimationSignal>,
}
