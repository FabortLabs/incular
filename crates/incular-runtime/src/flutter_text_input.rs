//! An optional Flutter text-input channel adapter for embedders that speak
//! Flutter's JSON method protocol instead of Incular's native IME events.

use incular_platform::{
    TextInputClientId,
    message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall},
};
use incular_text::{FlutterTextDeltaError, FlutterTextEditingDelta};
use serde_json::{Map, Value, json};

/// The opt-in Flutter delta channel for one attached text-input client.
/// Replace it when another client attaches, and discard it on detach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlutterDeltaTextInputConnection {
    client: TextInputClientId,
}

#[derive(Debug, thiserror::Error)]
pub enum FlutterDeltaTextInputError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("expected a client ID and a deltas array")]
    InvalidArguments,
    #[error("delta {index}: {source}")]
    InvalidDelta {
        index: usize,
        #[source]
        source: FlutterTextDeltaError,
    },
}

impl FlutterDeltaTextInputConnection {
    pub const CHANNEL: &str = "flutter/textinput";
    const UPDATE: &str = "TextInputClient.updateEditingStateWithDeltas";

    #[must_use]
    pub const fn new(client: TextInputClientId) -> Self {
        Self { client }
    }

    /// Encodes a Flutter `TextInput.setClient` call, retaining the
    /// embedder's input configuration and enabling granular updates.
    pub fn encode_set_client(
        self,
        mut configuration: Map<String, Value>,
    ) -> Result<Vec<u8>, JsonMethodCodecError> {
        configuration.insert("enableDeltaModel".into(), Value::Bool(true));
        JsonMethodCodec::encode_method_call(&MethodCall {
            method: "TextInput.setClient".into(),
            arguments: json!([self.client.get(), configuration]),
        })
    }

    /// Delivers a fully decoded batch to the attached client. Other methods
    /// and updates from a previously attached client return `Ok(false)`
    /// without inspecting or applying their delta arrays.
    ///
    /// # Errors
    /// Malformed messages or any malformed delta in the batch are returned
    /// without invoking the client. Errors omit the user's editing text.
    pub fn handle_delta_call(
        self,
        data: &[u8],
        on_deltas: impl FnOnce(Vec<FlutterTextEditingDelta>),
    ) -> Result<bool, FlutterDeltaTextInputError> {
        use FlutterDeltaTextInputError::{InvalidArguments, InvalidDelta};
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != Self::UPDATE {
            return Ok(false);
        }
        let args = call.arguments.as_array().ok_or(InvalidArguments)?;
        let sender = args.first().ok_or(InvalidArguments)?;
        let sender = if let Some(id) = sender.as_u64() {
            Some(id)
        } else if sender.as_i64().is_some() {
            None
        } else {
            return Err(InvalidArguments);
        };
        // Flutter's -1 cross-client override exists only in debug tests.
        let debug_override =
            cfg!(debug_assertions) && args.first().and_then(Value::as_i64) == Some(-1);
        if sender != Some(self.client.get()) && !debug_override {
            return Ok(false);
        }
        let encoded = args
            .get(1)
            .and_then(|arg| arg.get("deltas"))
            .and_then(Value::as_array)
            .ok_or(InvalidArguments)?;
        let deltas = encoded
            .iter()
            .enumerate()
            .map(|(index, encoded)| {
                FlutterTextEditingDelta::from_json(encoded)
                    .map_err(|source| InvalidDelta { index, source })
            })
            .collect::<Result<Vec<_>, _>>()?;
        on_deltas(deltas);
        Ok(true)
    }
}

/// Malformed input lifecycle message on the optional Flutter channel.
#[derive(Debug, thiserror::Error)]
pub enum FlutterTextInputLifecycleError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("expected a text input connection ID")]
    InvalidClientId,
}

impl crate::Runtime {
    /// Handles Flutter's text-input close and focus-received messages.
    /// Returns the accepted/rejected result, or `None` for another method.
    /// IDs are scoped to this window and refer to a specific attachment.
    pub fn handle_flutter_text_input_lifecycle_call(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputLifecycleError> {
        let call = JsonMethodCodec::decode_method_call(data)?;
        let receive_focus = match call.method.as_str() {
            "TextInputClient.onConnectionClosed" => false,
            "TextInputClient.onFocusReceived" => true,
            _ => return Ok(None),
        };
        let id = call
            .arguments
            .as_array()
            .and_then(|args| args.first())
            .ok_or(FlutterTextInputLifecycleError::InvalidClientId)?;
        let Some(client) = id.as_u64().map(TextInputClientId::new) else {
            // Ignore negative test sentinels rather than overriding the
            // identity check of a production input attachment.
            return if id.as_i64().is_some_and(|id| id < 0) {
                Ok(Some(false))
            } else {
                Err(FlutterTextInputLifecycleError::InvalidClientId)
            };
        };
        Ok(Some(if receive_focus {
            self.text_input_focus_received(client)
        } else {
            self.text_input_connection_closed(client)
        }))
    }
}
