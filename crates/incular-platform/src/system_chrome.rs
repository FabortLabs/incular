//! Flutter-compatible system chrome commands for application embedders.

use std::{future::Future, pin::Pin};

use incular_config::Brightness;
use incular_core::Color;
use serde_json::{Value, json};

use crate::SystemUiOverlayStyle;

use crate::flutter_platform_channel::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, invoke_optional_json,
};
use crate::message_codecs::{JsonMethodCodec, JsonMethodCodecError};

/// A rotation the platform may use for the application view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeviceOrientation {
    PortraitUp,
    LandscapeLeft,
    PortraitDown,
    LandscapeRight,
}

impl DeviceOrientation {
    const fn name(self) -> &'static str {
        match self {
            Self::PortraitUp => "DeviceOrientation.portraitUp",
            Self::LandscapeLeft => "DeviceOrientation.landscapeLeft",
            Self::PortraitDown => "DeviceOrientation.portraitDown",
            Self::LandscapeRight => "DeviceOrientation.landscapeRight",
        }
    }
}

/// Application label and primary ARGB color for the native task switcher.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplicationSwitcherDescription {
    pub label: Option<String>,
    pub primary_color: Option<u32>,
}

/// System UI visibility policy understood by a Flutter-compatible host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemUiMode {
    LeanBack,
    Immersive,
    ImmersiveSticky,
    EdgeToEdge,
    Manual,
}

/// One system bar, selected explicitly in manual UI mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SystemUiOverlay {
    Top,
    Bottom,
}

impl SystemUiOverlay {
    const fn name(self) -> &'static str {
        match self {
            Self::Top => "SystemUiOverlay.top",
            Self::Bottom => "SystemUiOverlay.bottom",
        }
    }
}

pub type SystemUiChangeCallback = Box<dyn FnMut(bool) -> Pin<Box<dyn Future<Output = ()>>>>;

#[derive(Debug, thiserror::Error)]
pub enum SystemUiChangeMessageError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("SystemChrome.systemUIChange arguments must start with a visibility boolean")]
    InvalidArguments,
}

/// The platform rejected a system bar style. Keeps the original transport
/// or platform reply and its usage context for the host's error reporter.
#[derive(Debug, thiserror::Error)]
#[error("while setting the system UI overlay style: {source}")]
pub struct SystemUiOverlayStyleError<E> {
    pub source: FlutterPlatformChannelError<E>,
}

impl<E> SystemUiOverlayStyleError<E> {
    #[must_use]
    pub const fn context(&self) -> &'static str {
        "while setting the system UI overlay style"
    }
}

pub type SystemUiOverlayStyleErrorCallback<E> = Box<dyn FnMut(&SystemUiOverlayStyleError<E>)>;

/// Sends system chrome policy to a compatible host and receives changes to
/// overlay visibility. The transport and physical effects are host-owned.
pub struct SystemChrome<M: FlutterPlatformMessenger> {
    messenger: M,
    visibility_listener: Option<SystemUiChangeCallback>,
    pending_style: Option<SystemUiOverlayStyle>,
    latest_style: Option<SystemUiOverlayStyle>,
    overlay_style_error: Option<SystemUiOverlayStyleErrorCallback<M::Error>>,
}

impl<M: FlutterPlatformMessenger> SystemChrome<M> {
    pub const CHANNEL: &str = "flutter/platform";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self {
            messenger,
            visibility_listener: None,
            pending_style: None,
            latest_style: None,
            overlay_style_error: None,
        }
    }

    /// Queues the style for the host's next microtask or UI turn. Returns
    /// true only when a new flush needs scheduling. Repeated writes before
    /// flushing replace the pending style instead of sending more calls.
    pub fn queue_system_ui_overlay_style(&mut self, style: SystemUiOverlayStyle) -> bool {
        if let Some(pending) = &mut self.pending_style {
            *pending = style;
            false
        } else if self.latest_style == Some(style) {
            false
        } else {
            self.pending_style = Some(style);
            true
        }
    }

    pub fn set_overlay_style_error_callback(
        &mut self,
        callback: Option<SystemUiOverlayStyleErrorCallback<M::Error>>,
    ) {
        self.overlay_style_error = callback;
    }

    /// Sends one queued style and reports any asynchronous platform failure.
    /// A turn that returns to the last style does not send another call.
    pub async fn flush_system_ui_overlay_style(
        &mut self,
    ) -> Result<bool, SystemUiOverlayStyleError<M::Error>> {
        let Some(style) = self.pending_style.take() else {
            return Ok(false);
        };
        if self.latest_style == Some(style) {
            return Ok(false);
        }
        // Flutter remembers the attempted style before its Future resolves.
        self.latest_style = Some(style);
        if let Err(source) = self
            .send(
                "SystemChrome.setSystemUIOverlayStyle",
                overlay_style_arguments(style),
            )
            .await
        {
            let report = SystemUiOverlayStyleError { source };
            if let Some(callback) = &mut self.overlay_style_error {
                callback(&report);
            }
            return Err(report);
        }
        Ok(true)
    }

    pub async fn set_preferred_orientations(
        &mut self,
        orientations: &[DeviceOrientation],
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        let arguments = Value::Array(
            orientations
                .iter()
                .map(|orientation| Value::from(orientation.name()))
                .collect(),
        );
        self.send("SystemChrome.setPreferredOrientations", arguments)
            .await
    }

    pub async fn set_application_switcher_description(
        &mut self,
        description: ApplicationSwitcherDescription,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        self.send(
            "SystemChrome.setApplicationSwitcherDescription",
            json!({ "label": description.label, "primaryColor": description.primary_color }),
        )
        .await
    }

    /// Sets a preset mode, or chooses the exact overlays in manual mode.
    ///
    /// # Panics
    /// Manual mode requires an explicit overlay list, which may be empty.
    pub async fn set_enabled_system_ui_mode(
        &mut self,
        mode: SystemUiMode,
        overlays: Option<&[SystemUiOverlay]>,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        let name = match mode {
            SystemUiMode::LeanBack => "SystemUiMode.leanBack",
            SystemUiMode::Immersive => "SystemUiMode.immersive",
            SystemUiMode::ImmersiveSticky => "SystemUiMode.immersiveSticky",
            SystemUiMode::EdgeToEdge => "SystemUiMode.edgeToEdge",
            SystemUiMode::Manual => {
                let overlays = overlays.expect("mode == SystemUiMode.manual && overlays != null");
                let arguments = Value::Array(
                    overlays
                        .iter()
                        .map(|overlay| Value::from(overlay.name()))
                        .collect(),
                );
                return self
                    .send("SystemChrome.setEnabledSystemUIOverlays", arguments)
                    .await;
            }
        };
        self.send("SystemChrome.setEnabledSystemUIMode", Value::from(name))
            .await
    }

    /// Clears the framework callback without making an outbound request
    /// when called with `None`.
    pub async fn set_system_ui_change_callback(
        &mut self,
        callback: Option<SystemUiChangeCallback>,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        self.visibility_listener = callback;
        if self.visibility_listener.is_some() {
            self.send("SystemChrome.setSystemUIChangeListener", Value::Null)
                .await?;
        }
        Ok(())
    }

    /// Routes one inbound visibility update and provides a JSON reply.
    /// Other methods are left to the embedder's other platform services.
    pub async fn handle_platform_message(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<Vec<u8>>, SystemUiChangeMessageError> {
        let call = JsonMethodCodec::decode_method_call(bytes)?;
        if call.method != "SystemChrome.systemUIChange" {
            return Ok(None);
        }
        let visible = call
            .arguments
            .as_array()
            .and_then(|args| args.first())
            .and_then(Value::as_bool)
            .ok_or(SystemUiChangeMessageError::InvalidArguments)?;
        if let Some(callback) = &mut self.visibility_listener {
            callback(visible).await;
        }
        Ok(Some(JsonMethodCodec::encode_success_envelope(
            &Value::Null,
        )?))
    }

    pub async fn restore_system_ui_overlays(
        &mut self,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        self.send("SystemChrome.restoreSystemUIOverlays", Value::Null)
            .await
    }

    async fn send(
        &mut self,
        method: &'static str,
        arguments: Value,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        invoke_optional_json(&mut self.messenger, Self::CHANNEL, method, arguments).await
    }
}

fn overlay_style_arguments(style: SystemUiOverlayStyle) -> Value {
    fn argb(color: Color) -> u32 {
        u32::from_be_bytes([color.alpha, color.red, color.green, color.blue])
    }
    fn brightness(value: Brightness) -> &'static str {
        match value {
            Brightness::Dark => "Brightness.dark",
            Brightness::Light => "Brightness.light",
        }
    }
    json!({
        "systemNavigationBarColor": style.system_navigation_bar_color.map(argb),
        "systemNavigationBarDividerColor": style.system_navigation_bar_divider_color.map(argb),
        "systemStatusBarContrastEnforced": style.system_status_bar_contrast_enforced,
        "statusBarColor": style.status_bar_color.map(argb),
        "statusBarBrightness": style.status_bar_brightness.map(brightness),
        "statusBarIconBrightness": style.status_bar_icon_brightness.map(brightness),
        "systemNavigationBarIconBrightness": style.system_navigation_bar_icon_brightness.map(brightness),
        "systemNavigationBarContrastEnforced": style.system_navigation_bar_contrast_enforced,
    })
}
