//! Hand-off of the application from the app's `main` to the native shell.
//!
//! On a phone the shell starts first and then calls the app's entry symbol.
//! That entry runs the app's ordinary `main`, whose `incular::run(app)` call
//! lands here instead of starting an event loop; the shell then takes the
//! application and drives it.

use incular_runtime::Application;
use std::cell::RefCell;

thread_local! {
    // `Application` is single-threaded; the entry and the shell both run on
    // the platform main thread.
    static PENDING: RefCell<Option<Application>> = const { RefCell::new(None) };
}

/// Stores the application built by the app's `main` for the shell. A second
/// call replaces an application the shell has not taken yet.
pub fn install_application(application: Application) {
    PENDING.with(|slot| *slot.borrow_mut() = Some(application));
}

/// Takes the application installed by [`install_application`] on this thread.
#[must_use]
pub fn take_application() -> Option<Application> {
    PENDING.with(|slot| slot.borrow_mut().take())
}
