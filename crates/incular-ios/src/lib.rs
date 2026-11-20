//! iOS embedder for Incular.
//!
//! `run_application` registers Incular's UIKit classes and starts
//! `UIApplicationMain` from the app's own `main`. The generated `Info.plist`
//! names `IncularSceneDelegate` as the scene delegate; each scene gets a
//! window whose root view is a `CAMetalLayer`-backed `IncularView`. Touches,
//! safe-area insets, scene lifecycle and a `CADisplayLink` frame pump drive a
//! [`incular_mobile::MobileEngine`] on the main thread.

mod accessibility;
#[cfg(target_os = "ios")]
mod shell;

pub use accessibility::IosAccessibilityAdapter;

use std::fmt;

/// Runs the UIKit application on the calling main thread. On iOS this never
/// returns: UIKit owns the process until it exits.
///
/// # Errors
///
/// Returns [`RunError::UnsupportedTarget`] when called on another target.
pub fn run_application(application: incular_runtime::Application) -> Result<(), RunError> {
    #[cfg(target_os = "ios")]
    {
        shell::run(application)
    }
    #[cfg(not(target_os = "ios"))]
    {
        drop(application);
        Err(RunError::UnsupportedTarget)
    }
}

/// Errors starting an application on iOS.
#[derive(Debug)]
#[non_exhaustive]
pub enum RunError {
    /// The iOS embedder only runs on iOS targets.
    UnsupportedTarget,
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTarget => formatter.write_str("the iOS embedder only runs on iOS"),
        }
    }
}

impl std::error::Error for RunError {}
