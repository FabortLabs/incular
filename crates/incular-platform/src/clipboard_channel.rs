//! JSON methods for Flutter-compatible clipboard host adapters.

use serde_json::{Value, json};

use crate::message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall};

#[derive(Debug, thiserror::Error)]
pub enum ClipboardChannelError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("clipboard reply must be an object or null")]
    InvalidReply,
    #[error("clipboard reply did not contain text")]
    MissingText,
    #[error("clipboard reply contained null text")]
    NullText,
    #[error("clipboard text must be a string")]
    InvalidText,
    #[error("clipboard presence reply must contain a boolean value")]
    InvalidPresence,
}

/// Flutter's plain-text clipboard methods. These codecs do not read or
/// change the native clipboard by themselves; embedders supply transport.
pub struct FlutterClipboardCodec;

impl FlutterClipboardCodec {
    pub const CHANNEL: &str = "flutter/platform";
    pub const TEXT_PLAIN: &str = "text/plain";

    pub fn get_data_request(format: &str) -> Result<Vec<u8>, ClipboardChannelError> {
        request("Clipboard.getData", Value::String(format.to_owned()))
    }

    pub fn set_data_request(text: &str) -> Result<Vec<u8>, ClipboardChannelError> {
        request("Clipboard.setData", json!({ "text": text }))
    }

    pub fn has_strings_request() -> Result<Vec<u8>, ClipboardChannelError> {
        request(
            "Clipboard.hasStrings",
            Value::String(Self::TEXT_PLAIN.to_owned()),
        )
    }

    /// A missing plugin reply or null payload is an empty clipboard.
    /// A returned object must actually contain a non-null string.
    pub fn decode_get_data_reply(
        reply: Option<&[u8]>,
    ) -> Result<Option<String>, ClipboardChannelError> {
        match reply.map(JsonMethodCodec::decode_envelope).transpose()? {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Object(mut result)) => match result.remove("text") {
                None => Err(ClipboardChannelError::MissingText),
                Some(Value::Null) => Err(ClipboardChannelError::NullText),
                Some(Value::String(text)) => Ok(Some(text)),
                Some(_) => Err(ClipboardChannelError::InvalidText),
            },
            Some(_) => Err(ClipboardChannelError::InvalidReply),
        }
    }

    pub fn decode_has_strings_reply(reply: Option<&[u8]>) -> Result<bool, ClipboardChannelError> {
        match reply.map(JsonMethodCodec::decode_envelope).transpose()? {
            None | Some(Value::Null) => Ok(false),
            Some(Value::Object(result)) => result
                .get("value")
                .and_then(Value::as_bool)
                .ok_or(ClipboardChannelError::InvalidPresence),
            Some(_) => Err(ClipboardChannelError::InvalidReply),
        }
    }
}

fn request(method: &str, arguments: Value) -> Result<Vec<u8>, ClipboardChannelError> {
    Ok(JsonMethodCodec::encode_method_call(&MethodCall {
        method: method.to_owned(),
        arguments,
    })?)
}
