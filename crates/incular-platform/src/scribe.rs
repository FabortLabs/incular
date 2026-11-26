//! Stylus handwriting messages for Flutter-compatible Android embedders.

use serde_json::Value;

use crate::flutter_platform_channel::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, invoke_optional_json,
    invoke_optional_json_value,
};

#[derive(Debug, thiserror::Error)]
pub enum ScribeError<E> {
    #[error(transparent)]
    Channel(#[from] FlutterPlatformChannelError<E>),
    #[error("MethodChannel.invokeMethod unexpectedly returned null.")]
    MissingCapability,
    #[error("the stylus handwriting capability must be a boolean")]
    InvalidCapability,
}

/// Sends stylus handwriting requests to the enclosing text-input host.
/// It does not substitute gestures on systems without handwriting input.
pub struct Scribe<M> {
    messenger: M,
}

impl<M: FlutterPlatformMessenger> Scribe<M> {
    pub const CHANNEL: &str = "flutter/scribe";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self { messenger }
    }

    pub async fn is_stylus_handwriting_available(&mut self) -> Result<bool, ScribeError<M::Error>> {
        self.capability("Scribe.isStylusHandwritingAvailable").await
    }

    pub async fn is_feature_available(&mut self) -> Result<bool, ScribeError<M::Error>> {
        self.capability("Scribe.isFeatureAvailable").await
    }

    pub async fn start_stylus_handwriting(&mut self) -> Result<(), ScribeError<M::Error>> {
        invoke_optional_json(
            &mut self.messenger,
            Self::CHANNEL,
            "Scribe.startStylusHandwriting",
            Value::Null,
        )
        .await?;
        Ok(())
    }

    async fn capability(&mut self, method: &'static str) -> Result<bool, ScribeError<M::Error>> {
        match invoke_optional_json_value(&mut self.messenger, Self::CHANNEL, method, Value::Null)
            .await?
        {
            Value::Bool(value) => Ok(value),
            Value::Null => Err(ScribeError::MissingCapability),
            _ => Err(ScribeError::InvalidCapability),
        }
    }
}
