//! Rich payloads supplied by a platform input method.

use serde_json::Value;

/// A rich IME insertion received from the host. The URI is an opaque
/// identifier; decoding this value never reads from it.
#[derive(Clone, Debug)]
pub struct KeyboardInsertedContent {
    pub mime_type: String,
    pub uri: String,
    pub data: Option<Vec<u8>>,
}

impl KeyboardInsertedContent {
    /// Whether the input method supplied nonempty bytes.
    #[must_use]
    pub fn has_data(&self) -> bool {
        self.data.as_ref().is_some_and(|bytes| !bytes.is_empty())
    }

    pub fn from_flutter_json(value: &Value) -> Result<Self, KeyboardInsertedContentError> {
        use KeyboardInsertedContentError::*;
        let metadata = value.as_object().ok_or(InvalidMetadata)?;
        let mime_type = metadata
            .get("mimeType")
            .and_then(Value::as_str)
            .ok_or(InvalidMimeType)?;
        let uri = metadata
            .get("uri")
            .and_then(Value::as_str)
            .ok_or(InvalidUri)?;
        let data = match metadata.get("data") {
            None | Some(Value::Null) => None,
            Some(Value::Array(values)) => Some(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let byte = value
                            .as_i64()
                            .map(|value| value.rem_euclid(256) as u8)
                            .or_else(|| value.as_u64().map(|value| (value % 256) as u8));
                        byte.ok_or(InvalidByte { index })
                    })
                    .collect::<Result<_, _>>()?,
            ),
            Some(_) => return Err(InvalidData),
        };
        Ok(Self {
            mime_type: mime_type.into(),
            uri: uri.into(),
            data,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum KeyboardInsertedContentError {
    #[error("expected keyboard content metadata")]
    InvalidMetadata,
    #[error("expected a keyboard content MIME type string")]
    InvalidMimeType,
    #[error("expected a keyboard content URI string")]
    InvalidUri,
    #[error("expected keyboard content data to be a list or null")]
    InvalidData,
    #[error("keyboard content data at index {index} is not an integer")]
    InvalidByte { index: usize },
}
