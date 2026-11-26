//! Flutter-compatible message codecs for platform channels.

pub use incular_core::standard_message_codec::{
    MessageCodecError, MessageValue, MethodCall, MethodCodecError, MissingPluginException,
    PlatformException, StandardMessageCodec, StandardMethodCodec,
};

/// Unencoded byte messages. The caller's exact borrowed view is returned.
#[derive(Clone, Copy, Debug, Default)]
pub struct BinaryCodec;

impl BinaryCodec {
    #[must_use]
    pub const fn encode_message(bytes: Option<&[u8]>) -> Option<&[u8]> {
        bytes
    }

    #[must_use]
    pub const fn decode_message(bytes: Option<&[u8]>) -> Option<&[u8]> {
        bytes
    }
}

/// UTF-8 string messages. Absence is distinct from an empty string.
#[derive(Clone, Copy, Debug, Default)]
pub struct StringCodec;

impl StringCodec {
    #[must_use]
    pub fn encode_message(text: Option<&str>) -> Option<&[u8]> {
        text.map(str::as_bytes)
    }

    /// Decodes exactly the provided view, removing a leading UTF-8 BOM.
    pub fn decode_message(bytes: Option<&[u8]>) -> Result<Option<&str>, std::str::Utf8Error> {
        bytes
            .map(|bytes| std::str::from_utf8(without_utf8_bom(bytes)))
            .transpose()
    }
}

/// JSON messages carried as UTF-8. A top-level null has no message bytes.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonMessageCodec;

impl JsonMessageCodec {
    pub fn encode_message(message: &serde_json::Value) -> serde_json::Result<Option<Vec<u8>>> {
        if message.is_null() {
            Ok(None)
        } else {
            serde_json::to_vec(message).map(Some)
        }
    }

    pub fn decode_message(bytes: Option<&[u8]>) -> serde_json::Result<serde_json::Value> {
        bytes.map_or(Ok(serde_json::Value::Null), |bytes| {
            serde_json::from_slice(without_utf8_bom(bytes))
        })
    }
}

fn without_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes)
}

#[derive(Debug, thiserror::Error)]
pub enum JsonMethodCodecError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("expected a JSON method call with a string method name")]
    InvalidMethodCall,
    #[error("invalid JSON method reply envelope")]
    InvalidEnvelope,
    #[error(transparent)]
    Platform(Box<PlatformException<serde_json::Value>>),
}

/// JSON method calls and replies encoded as UTF-8.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonMethodCodec;

impl JsonMethodCodec {
    pub fn encode_method_call(
        call: &MethodCall<serde_json::Value>,
    ) -> Result<Vec<u8>, JsonMethodCodecError> {
        Ok(serde_json::to_vec(&serde_json::json!({
            "method": call.method,
            "args": call.arguments,
        }))?)
    }

    pub fn decode_method_call(
        data: &[u8],
    ) -> Result<MethodCall<serde_json::Value>, JsonMethodCodecError> {
        use serde_json::Value;
        let Value::Object(mut fields) = JsonMessageCodec::decode_message(Some(data))? else {
            return Err(JsonMethodCodecError::InvalidMethodCall);
        };
        let Some(Value::String(method)) = fields.remove("method") else {
            return Err(JsonMethodCodecError::InvalidMethodCall);
        };
        Ok(MethodCall {
            method,
            arguments: fields.remove("args").unwrap_or(Value::Null),
        })
    }

    pub fn encode_success_envelope(
        result: &serde_json::Value,
    ) -> Result<Vec<u8>, JsonMethodCodecError> {
        Ok(serde_json::to_vec(&[result])?)
    }

    pub fn encode_error_envelope(
        error: &PlatformException<serde_json::Value>,
    ) -> Result<Vec<u8>, JsonMethodCodecError> {
        let fields = (&error.code, &error.message, &error.details);
        if let Some(trace) = &error.stacktrace {
            Ok(serde_json::to_vec(&(fields.0, fields.1, fields.2, trace))?)
        } else {
            Ok(serde_json::to_vec(&fields)?)
        }
    }

    pub fn decode_envelope(data: &[u8]) -> Result<serde_json::Value, JsonMethodCodecError> {
        use serde_json::Value;
        let Value::Array(mut fields) = JsonMessageCodec::decode_message(Some(data))? else {
            return Err(JsonMethodCodecError::InvalidEnvelope);
        };
        let stacktrace = match fields.len() {
            1 => return Ok(fields.pop().expect("one success value")),
            3 => None,
            4 => nullable_string(fields.pop().expect("trace field"))?,
            _ => return Err(JsonMethodCodecError::InvalidEnvelope),
        };
        let details = fields.pop().expect("details field");
        let message = nullable_string(fields.pop().expect("message field"))?;
        let Some(Value::String(code)) = fields.pop() else {
            return Err(JsonMethodCodecError::InvalidEnvelope);
        };
        Err(JsonMethodCodecError::Platform(Box::new(
            PlatformException {
                code,
                message,
                details,
                stacktrace,
            },
        )))
    }
}

fn nullable_string(value: serde_json::Value) -> Result<Option<String>, JsonMethodCodecError> {
    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(text) => Ok(Some(text)),
        _ => Err(JsonMethodCodecError::InvalidEnvelope),
    }
}
