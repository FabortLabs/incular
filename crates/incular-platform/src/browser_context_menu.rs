//! Flutter's browser context-menu protocol.

use serde_json::Value;

use crate::flutter_platform_channel::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, invoke_optional_json,
};

/// Controls the browser's context menu for one mounted view. The enabled
/// value reflects completed calls, not unrelated listeners installed by
/// JavaScript outside the view.
pub struct BrowserContextMenu<M> {
    messenger: M,
    enabled: bool,
}

impl<M: FlutterPlatformMessenger> BrowserContextMenu<M> {
    pub const CHANNEL: &str = "flutter/contextmenu";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self {
            messenger,
            enabled: true,
        }
    }

    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// # Panics
    /// Asserts in debug builds on non-browser targets.
    pub async fn disable_context_menu(
        &mut self,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        self.set_enabled(false).await
    }

    /// # Panics
    /// Asserts in debug builds on non-browser targets.
    pub async fn enable_context_menu(
        &mut self,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        self.set_enabled(true).await
    }

    async fn set_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        debug_assert!(
            cfg!(target_arch = "wasm32"),
            "This has no effect on platforms other than web."
        );
        invoke_optional_json(
            &mut self.messenger,
            Self::CHANNEL,
            if enabled {
                "enableContextMenu"
            } else {
                "disableContextMenu"
            },
            Value::Null,
        )
        .await?;
        self.enabled = enabled;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod dom {
    use std::future::{Future, ready};

    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{Event, EventTarget};

    use super::{BrowserContextMenu, FlutterPlatformMessenger};
    use crate::message_codecs::{JsonMethodCodec, JsonMethodCodecError};

    #[derive(Debug, thiserror::Error)]
    pub enum DomContextMenuError {
        #[error(transparent)]
        Codec(#[from] JsonMethodCodecError),
        #[error("could not modify the browser context-menu listener: {0:?}")]
        Dom(JsValue),
    }

    /// DOM implementation of the Flutter context-menu channel. It affects
    /// just its associated view element and leaves sibling page content alone.
    pub struct DomContextMenuMessenger {
        element: EventTarget,
        blocker: Option<Closure<dyn FnMut(Event)>>,
    }

    impl DomContextMenuMessenger {
        #[must_use]
        pub fn new(element: EventTarget) -> Self {
            Self {
                element,
                blocker: None,
            }
        }

        fn set_enabled(&mut self, enabled: bool) -> Result<(), DomContextMenuError> {
            if enabled {
                if let Some(blocker) = self.blocker.as_ref() {
                    self.element
                        .remove_event_listener_with_callback(
                            "contextmenu",
                            blocker.as_ref().unchecked_ref(),
                        )
                        .map_err(DomContextMenuError::Dom)?;
                    self.blocker = None;
                }
            } else if self.blocker.is_none() {
                let blocker = Closure::wrap(
                    Box::new(|event: Event| event.prevent_default()) as Box<dyn FnMut(Event)>
                );
                self.element
                    .add_event_listener_with_callback(
                        "contextmenu",
                        blocker.as_ref().unchecked_ref(),
                    )
                    .map_err(DomContextMenuError::Dom)?;
                self.blocker = Some(blocker);
            }
            Ok(())
        }

        fn handle(
            &mut self,
            channel: &'static str,
            payload: &[u8],
        ) -> Result<Option<Vec<u8>>, DomContextMenuError> {
            if channel != BrowserContextMenu::<Self>::CHANNEL {
                return Ok(None);
            }
            let call = JsonMethodCodec::decode_method_call(payload)?;
            let enabled = match call.method.as_str() {
                "disableContextMenu" => false,
                "enableContextMenu" => true,
                _ => return Ok(None),
            };
            self.set_enabled(enabled)?;
            Ok(Some(JsonMethodCodec::encode_success_envelope(
                &serde_json::Value::Bool(true),
            )?))
        }
    }

    impl Drop for DomContextMenuMessenger {
        fn drop(&mut self) {
            let _ = self.set_enabled(true);
        }
    }

    impl FlutterPlatformMessenger for DomContextMenuMessenger {
        type Error = DomContextMenuError;

        fn send_platform_message(
            &mut self,
            channel: &'static str,
            payload: Vec<u8>,
        ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
            ready(self.handle(channel, &payload))
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use dom::{DomContextMenuError, DomContextMenuMessenger};
