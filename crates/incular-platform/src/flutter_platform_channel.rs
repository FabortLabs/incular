//! Shared transport for Flutter's optional JSON platform channels.

use std::future::Future;

use serde_json::Value;

use crate::message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall};

/// Transport for optional Flutter-compatible channels. A missing handler
/// returns `Ok(None)`; successful and failed handlers return JSON envelopes.
pub trait FlutterPlatformMessenger {
    type Error;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>>;
}

/// A failed optional-channel command. A missing handler is not an error.
#[derive(Debug, thiserror::Error)]
pub enum FlutterPlatformChannelError<E> {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("while sending a Flutter-compatible platform command: {0}")]
    Transport(E),
}

pub(crate) async fn invoke_optional_json<M: FlutterPlatformMessenger>(
    messenger: &mut M,
    channel: &'static str,
    method: &'static str,
    arguments: Value,
) -> Result<(), FlutterPlatformChannelError<M::Error>> {
    let payload = JsonMethodCodec::encode_method_call(&MethodCall {
        method: method.to_owned(),
        arguments,
    })?;
    if let Some(reply) = messenger
        .send_platform_message(channel, payload)
        .await
        .map_err(FlutterPlatformChannelError::Transport)?
    {
        JsonMethodCodec::decode_envelope(&reply)?;
    }
    Ok(())
}
