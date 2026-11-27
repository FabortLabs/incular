//! Asset-only deferred component requests for a Flutter-compatible host.

use crate::{
    FlutterPlatformMessenger,
    message_codecs::{MessageValue, MethodCall, MethodCodecError, StandardMethodCodec},
};

#[derive(Debug, thiserror::Error)]
pub enum DeferredComponentError<E> {
    #[error(transparent)]
    Codec(#[from] MethodCodecError),
    #[error("deferred component transport failed: {0}")]
    Transport(E),
}

/// Requests asset delivery from the installed application's embedder. This
/// does not load new code, unpack assets itself, or resolve file paths.
pub struct DeferredComponent<M> {
    messenger: M,
}

impl<M: FlutterPlatformMessenger> DeferredComponent<M> {
    pub const CHANNEL: &str = "flutter/deferredcomponent";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self { messenger }
    }

    /// Completes on the host's install reply. An unavailable optional host
    /// service is a no-op, as on platforms without deferred components.
    pub async fn install(
        &mut self,
        component_name: &str,
    ) -> Result<(), DeferredComponentError<M::Error>> {
        self.request("installDeferredComponent", component_name)
            .await
    }

    /// Completes when the host acknowledges the request, not necessarily
    /// when it finishes deleting the component's files.
    pub async fn uninstall(
        &mut self,
        component_name: &str,
    ) -> Result<(), DeferredComponentError<M::Error>> {
        self.request("uninstallDeferredComponent", component_name)
            .await
    }

    async fn request(
        &mut self,
        method: &'static str,
        component_name: &str,
    ) -> Result<(), DeferredComponentError<M::Error>> {
        let message = StandardMethodCodec::encode_method_call(&MethodCall {
            method: method.into(),
            arguments: MessageValue::Map(vec![
                (
                    MessageValue::String("loadingUnitId".into()),
                    MessageValue::Int(-1),
                ),
                (
                    MessageValue::String("componentName".into()),
                    MessageValue::String(component_name.into()),
                ),
            ]),
        })?;
        if let Some(envelope) = self
            .messenger
            .send_platform_message(Self::CHANNEL, Some(message))
            .await
            .map_err(DeferredComponentError::Transport)?
        {
            StandardMethodCodec::decode_envelope(&envelope)?;
        }
        Ok(())
    }
}
