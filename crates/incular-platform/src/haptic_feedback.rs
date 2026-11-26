//! Requests for the haptic patterns supplied by a platform host.

use serde_json::Value;

use crate::flutter_platform_channel::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, invoke_optional_json,
};

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
pub type HapticFeedbackError<E> = FlutterPlatformChannelError<E>;

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
        invoke_optional_json(
            &mut self.messenger,
            Self::CHANNEL,
            "HapticFeedback.vibrate",
            effect.map_or(Value::Null, HapticFeedbackType::argument),
        )
        .await
    }
}
