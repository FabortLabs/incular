//! Shared transport and methods for Flutter-compatible JSON channels.

use std::{collections::BTreeMap, future::Future};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    FlutterInboundMessenger,
    flutter_inbound_messenger::encode_reply,
    message_codecs::{
        JsonMethodCodec, JsonMethodCodecError, MethodCall, MissingPluginException,
        PlatformException,
    },
};

/// Transport for Flutter-compatible channels. A missing handler returns
/// `Ok(None)`; registered handlers return bytes for the selected codec.
/// The request also distinguishes null from an empty byte buffer.
pub trait FlutterPlatformMessenger {
    type Error;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>>;
}

#[derive(Debug, thiserror::Error)]
pub enum FlutterPlatformChannelError<E> {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error(transparent)]
    MissingPlugin(#[from] MissingPluginException),
    #[error("while sending a Flutter-compatible platform command: {0}")]
    Transport(E),
}

/// A named Flutter-compatible JSON method channel.
///
/// A missing response is distinct from a success envelope with a null value.
/// Use [Self::optional] for a channel on which both result in null.
#[derive(Clone, Copy, Debug)]
pub struct FlutterJsonMethodChannel {
    name: &'static str,
    missing_ok: bool,
}

impl FlutterJsonMethodChannel {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            missing_ok: false,
        }
    }

    pub const fn optional(name: &'static str) -> Self {
        Self {
            name,
            missing_ok: true,
        }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub async fn invoke<M: FlutterPlatformMessenger>(
        self,
        messenger: &mut M,
        method: &str,
        arguments: Value,
    ) -> Result<Value, FlutterPlatformChannelError<M::Error>> {
        let payload = JsonMethodCodec::encode_method_call(&MethodCall {
            method: method.to_owned(),
            arguments,
        })?;
        let reply = messenger
            .send_platform_message(self.name, Some(payload))
            .await
            .map_err(FlutterPlatformChannelError::Transport)?;
        match reply {
            Some(bytes) => Ok(JsonMethodCodec::decode_envelope(&bytes)?),
            None if self.missing_ok => Ok(Value::Null),
            None => Err(MissingPluginException {
                message: Some(format!(
                    "No implementation found for method {method} on channel {}",
                    self.name,
                )),
            }
            .into()),
        }
    }

    /// Decodes the contents of a successful list result. A null result
    /// remains null, rather than becoming an empty vector.
    pub async fn invoke_list<T: DeserializeOwned, M: FlutterPlatformMessenger>(
        self,
        messenger: &mut M,
        method: &str,
        arguments: Value,
    ) -> Result<Option<Vec<T>>, FlutterPlatformChannelError<M::Error>> {
        let value = self.invoke(messenger, method, arguments).await?;
        Ok(serde_json::from_value(value).map_err(JsonMethodCodecError::Json)?)
    }

    /// Decodes a successful JSON object with string keys. A null result
    /// remains null, rather than becoming an empty map.
    pub async fn invoke_map<T: DeserializeOwned, M: FlutterPlatformMessenger>(
        self,
        messenger: &mut M,
        method: &str,
        arguments: Value,
    ) -> Result<Option<BTreeMap<String, T>>, FlutterPlatformChannelError<M::Error>> {
        let value = self.invoke(messenger, method, arguments).await?;
        Ok(serde_json::from_value(value).map_err(JsonMethodCodecError::Json)?)
    }

    /// Registers a receiver for decoded method calls. The handler may move the
    /// one-shot responder into a local task to complete asynchronously.
    pub fn set_method_call_handler(
        self,
        inbound: &FlutterInboundMessenger,
        handler: impl Fn(MethodCall<Value>, FlutterJsonMethodReply) + 'static,
    ) {
        inbound.set_message_handler(self.name, move |payload, reply| {
            let call = payload
                .as_deref()
                .ok_or(JsonMethodCodecError::InvalidMethodCall)
                .and_then(JsonMethodCodec::decode_method_call);
            match call {
                Ok(call) => handler(
                    call,
                    Box::new(encode_reply(reply, FlutterJsonMethodResult::encode)),
                ),
                Err(_) => reply(None),
            }
        });
    }

    pub fn clear_method_call_handler(self, inbound: &FlutterInboundMessenger) {
        inbound.clear_message_handler(self.name);
    }
}

/// Completes a single inbound JSON method call.
pub type FlutterJsonMethodReply =
    Box<dyn FnOnce(FlutterJsonMethodResult) -> Result<(), JsonMethodCodecError>>;

#[derive(Debug)]
pub enum FlutterJsonMethodResult {
    Success(Value),
    NotImplemented,
    PlatformError(PlatformException<Value>),
    /// An application error that is not already a platform exception.
    Error(Box<dyn std::error::Error>),
}

impl FlutterJsonMethodResult {
    fn encode(self) -> Result<Option<Vec<u8>>, JsonMethodCodecError> {
        let bytes = match self {
            Self::Success(value) => JsonMethodCodec::encode_success_envelope(&value)?,
            Self::NotImplemented => return Ok(None),
            Self::PlatformError(error) => JsonMethodCodec::encode_error_envelope(&error)?,
            Self::Error(error) => JsonMethodCodec::encode_error_envelope(&PlatformException {
                code: "error".into(),
                message: Some(error.to_string()),
                details: Value::Null,
                stacktrace: None,
            })?,
        };
        Ok(Some(bytes))
    }
}

pub(crate) async fn invoke_optional_json<M: FlutterPlatformMessenger>(
    messenger: &mut M,
    channel: &'static str,
    method: &'static str,
    arguments: Value,
) -> Result<(), FlutterPlatformChannelError<M::Error>> {
    invoke_optional_json_value(messenger, channel, method, arguments)
        .await
        .map(|_| ())
}

pub(crate) async fn invoke_optional_json_value<M: FlutterPlatformMessenger>(
    messenger: &mut M,
    channel: &'static str,
    method: &'static str,
    arguments: Value,
) -> Result<Value, FlutterPlatformChannelError<M::Error>> {
    FlutterJsonMethodChannel::optional(channel)
        .invoke(messenger, method, arguments)
        .await
}
