//! Browser variant of single-instance ownership.
//!
//! Every page is its own process with no shared lock file or local socket,
//! so an application that requests a single-instance policy receives a typed
//! [`SingleInstanceError::Unsupported`] at launch instead of a lock that
//! nothing enforces.

use incular_platform::{ApplicationActivation, SingleInstancePolicy};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug, thiserror::Error)]
pub enum SingleInstanceError {
    #[error("single-instance ownership is unavailable in browsers")]
    Unsupported,
}

#[doc(hidden)]
pub enum SingleInstanceRole {
    Primary(SingleInstancePrimary),
    SecondaryForwarded,
}

/// Uninhabited: a browser page never becomes a single-instance primary.
#[doc(hidden)]
#[derive(Debug)]
pub struct SingleInstancePrimary {
    never: Infallible,
}

impl SingleInstancePrimary {
    #[doc(hidden)]
    pub fn set_wake(&self, _wake: Arc<dyn Fn() + Send + Sync>) {
        match self.never {}
    }

    #[doc(hidden)]
    pub fn take_activations(&self) -> Vec<ApplicationActivation> {
        match self.never {}
    }
}

#[doc(hidden)]
pub fn acquire_single_instance(
    _policy: &SingleInstancePolicy,
    _activation: ApplicationActivation,
) -> Result<SingleInstanceRole, SingleInstanceError> {
    Err(SingleInstanceError::Unsupported)
}
