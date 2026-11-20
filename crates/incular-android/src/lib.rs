//! Android embedder for Incular.
//!
//! The Kotlin `dev.incular.IncularActivity` (sources under `android/`) owns
//! the process, the activity and a `SurfaceView`. It loads the app's shared
//! library and forwards surface, touch, inset and lifecycle callbacks through
//! JNI to a [`incular_mobile::MobileEngine`] running on the main thread. Frames
//! are paced by the NDK Choreographer; async runtime work wakes the main
//! looper through a socket pair.
//!
//! The app's library must export `incular_android_main`, which runs the app's
//! `main` (see the `incular::mobile_entry!` macro). Its `incular::run(app)`
//! call reaches [`run_application`], which hands the application to the
//! embedder instead of starting an event loop.

mod accessibility;
#[cfg(target_os = "android")]
mod shell;

pub use accessibility::AndroidAccessibilityAdapter;

use std::fmt;

/// Hands `application` to the Android embedder. Called by `incular::run`
/// from inside the app's `main`, which the embedder runs when the activity
/// starts; the activity then drives the application.
///
/// # Errors
///
/// Never fails today; the type matches the other platform entries.
pub fn run_application(application: incular_runtime::Application) -> Result<(), RunError> {
    incular_mobile::install_application(application);
    Ok(())
}

/// Errors starting an application on Android. No failure is possible yet: the
/// activity reports renderer and surface failures through logcat.
#[derive(Debug)]
pub enum RunError {}

impl fmt::Display for RunError {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl std::error::Error for RunError {}
