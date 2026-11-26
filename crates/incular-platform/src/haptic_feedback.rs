//! Requests for the haptic patterns supplied by a platform host.

use std::future::Future;

use serde_json::Value;

use crate::message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall};

/// Transport for the optional Flutter-compatible platform channel.
/// A missing handler returns `Ok(None)`.
pub trait FlutterPlatformMessenger {
    type Error;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>>;
}

/// The haptic patterns selected by Flutter's platform protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HapticFeedbackType {
    LightImpact,
    MediumImpact,
    HeavyImpact,
    SelectionClick,
    SuccessNotification,
    WarningNotification,
    ErrorNotification,
}

impl HapticFeedbackType {
    fn argument(self) -> Value {
        Value::String(
            match self {
                Self::LightImpact => "HapticFeedbackType.lightImpact",
                Self::MediumImpact => "HapticFeedbackType.mediumImpact",
                Self::HeavyImpact => "HapticFeedbackType.heavyImpact",
                Self::SelectionClick => "HapticFeedbackType.selectionClick",
                Self::SuccessNotification => "HapticFeedbackType.successNotification",
                Self::WarningNotification => "HapticFeedbackType.warningNotification",
                Self::ErrorNotification => "HapticFeedbackType.errorNotification",
            }
            .to_owned(),
        )
    }
}

/// A malformed reply, remote platform error, or failed transport.
#[derive(Debug, thiserror::Error)]
pub enum HapticFeedbackError<E> {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("while requesting platform haptic feedback: {0}")]
    Transport(E),
}

/// Haptic feedback through a platform messenger supplied by the embedder.
/// Unsupported hosts may ignore the request; there is no synthetic fallback.
pub struct HapticFeedback<M> {
    messenger: M,
}

impl<M: FlutterPlatformMessenger> HapticFeedback<M> {
    pub const CHANNEL: &str = "flutter/platform";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self { messenger }
    }

    /// Requests a standard vibration with no platform argument.
    pub async fn vibrate(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(None).await
    }

    pub async fn light_impact(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(Some(HapticFeedbackType::LightImpact)).await
    }

    pub async fn medium_impact(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(Some(HapticFeedbackType::MediumImpact)).await
    }

    pub async fn heavy_impact(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(Some(HapticFeedbackType::HeavyImpact)).await
    }

    pub async fn selection_click(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(Some(HapticFeedbackType::SelectionClick)).await
    }

    pub async fn success_notification(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(Some(HapticFeedbackType::SuccessNotification))
            .await
    }

    pub async fn warning_notification(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(Some(HapticFeedbackType::WarningNotification))
            .await
    }

    pub async fn error_notification(&mut self) -> Result<(), HapticFeedbackError<M::Error>> {
        self.request(Some(HapticFeedbackType::ErrorNotification))
            .await
    }

    pub async fn request(
        &mut self,
        effect: Option<HapticFeedbackType>,
    ) -> Result<(), HapticFeedbackError<M::Error>> {
        let payload = JsonMethodCodec::encode_method_call(&MethodCall {
            method: "HapticFeedback.vibrate".to_owned(),
            arguments: effect.map_or(Value::Null, HapticFeedbackType::argument),
        })?;
        if let Some(reply) = self
            .messenger
            .send_platform_message(Self::CHANNEL, payload)
            .await
            .map_err(HapticFeedbackError::Transport)?
        {
            JsonMethodCodec::decode_envelope(&reply)?;
        }
        Ok(())
    }
}
