//! An optional Flutter text-input channel adapter for embedders that speak
//! Flutter's JSON method protocol instead of Incular's native IME events.

use incular_platform::{
    FlutterTextInputConfiguration, TextInputClientId, TextInputState,
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

#[derive(Debug, thiserror::Error)]
pub enum FlutterTextInputSelectorError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("expected a client ID and an array of selector names")]
    InvalidArguments,
    #[error("selector {index} is not a string")]
    InvalidSelector { index: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum FlutterKeyboardContentError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("expected a client ID and keyboard content metadata")]
    InvalidArguments,
    #[error(transparent)]
    Content(#[from] incular_platform::KeyboardInsertedContentError),
}

#[derive(Debug, thiserror::Error)]
pub enum FlutterTextInputFloatingCursorError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("expected a client ID, floating cursor state and position")]
    InvalidArguments,
    #[error(transparent)]
    Position(#[from] incular_platform::FlutterFloatingCursorPointError),
}

fn sender_matches(sender: &Value, client: TextInputClientId) -> Option<bool> {
    if let Some(id) = sender.as_u64() {
        Some(id == client.get())
    } else {
        // Flutter's -1 cross-client override exists only in debug tests.
        sender.as_i64().map(|id| cfg!(debug_assertions) && id == -1)
    }
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

    /// Sends the typed Flutter configuration, opting this client into deltas.
    pub fn encode_set_client_with_config(
        self,
        configuration: &FlutterTextInputConfiguration,
    ) -> Result<Vec<u8>, JsonMethodCodecError> {
        self.encode_set_client(configuration.to_json())
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
        if !sender_matches(sender, self.client).ok_or(InvalidArguments)? {
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
    #[error("expected a valid UTF-16 autocorrection prompt range")]
    InvalidPromptRange,
}

/// A full editing-state message could not be routed or decoded.
/// Diagnostics include offsets and lengths, never the editing text.
#[derive(Debug, thiserror::Error)]
pub enum FlutterTextInputEditingError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("expected a client ID and an editing-state object")]
    InvalidArguments,
    #[error("invalid Flutter editing-state field {0}")]
    InvalidField(&'static str),
    #[error(
        "Invalid {range} range: Range {edge} {offset} is out of text of length {length} (range.{edge} >= 0 && range.{edge} <= text.length)"
    )]
    OutOfRange {
        range: &'static str,
        edge: &'static str,
        offset: i64,
        length: usize,
    },
    #[error("invalid UTF-16 {range} range: offset {offset} splits a character")]
    SplitScalar { range: &'static str, offset: i64 },
    #[error("invalid UTF-16 composing range: start {start} is after end {end}")]
    ReversedComposition { start: i64, end: i64 },
}

/// Malformed application-private command on the optional Flutter channel.
#[derive(Debug, thiserror::Error)]
pub enum FlutterTextInputPrivateCommandError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("expected a client ID and a private-command object")]
    InvalidArguments,
    #[error("expected a string private-command action")]
    InvalidAction,
    #[error("expected private-command data to be an object or null")]
    InvalidData,
}

fn editing_endpoint(
    text: &str,
    offset: i64,
    range: &'static str,
    edge: &'static str,
) -> Result<usize, FlutterTextInputEditingError> {
    use FlutterTextInputEditingError::{OutOfRange, SplitScalar};
    let requested = usize::try_from(offset).ok();
    let mut units = 0;
    for (bytes, scalar) in text.char_indices() {
        if requested == Some(units) {
            return Ok(bytes);
        }
        if requested.is_some_and(|requested| requested < units) {
            return Err(SplitScalar { range, offset });
        }
        units += scalar.len_utf16();
    }
    match requested {
        Some(requested) if requested == units => Ok(text.len()),
        Some(requested) if requested < units => Err(SplitScalar { range, offset }),
        _ => Err(OutOfRange {
            range,
            edge,
            offset,
            length: units,
        }),
    }
}

fn decode_full_editing_state(data: &Value) -> Result<TextInputState, FlutterTextInputEditingError> {
    use FlutterTextInputEditingError::{InvalidArguments, InvalidField, ReversedComposition};
    let fields = data.as_object().ok_or(InvalidArguments)?;
    let text = fields
        .get("text")
        .and_then(Value::as_str)
        .ok_or(InvalidField("text"))?;
    let read_offset = |field| match fields.get(field) {
        None | Some(Value::Null) => Ok(-1),
        Some(value) => value.as_i64().ok_or(InvalidField(field)),
    };
    // The retained selection stores base and extent but not affinity or
    // directionality metadata. Still enforce the incoming wire types.
    for (field, string) in [
        ("selectionAffinity", true),
        ("selectionIsDirectional", false),
    ] {
        if fields.get(field).is_some_and(|value| {
            !value.is_null()
                && if string {
                    !value.is_string()
                } else {
                    !value.is_boolean()
                }
        }) {
            return Err(InvalidField(field));
        }
    }
    let base = read_offset("selectionBase")?;
    let extent = read_offset("selectionExtent")?;
    let (selection_start, selection_end) = if (base, extent) == (-1, -1) {
        // A mounted native editor represents absent selection as an
        // unselected caret at the end, never as an invalid byte index.
        (text.len(), text.len())
    } else {
        let start = editing_endpoint(text, base.min(extent), "selection", "start")?;
        let end = editing_endpoint(text, base.max(extent), "selection", "end")?;
        if base <= extent {
            (start, end)
        } else {
            (end, start)
        }
    };
    let start = read_offset("composingBase")?;
    let end = read_offset("composingExtent")?;
    let composing = if (start, end) == (-1, -1) {
        None
    } else {
        if start > end {
            return Err(ReversedComposition { start, end });
        }
        Some((
            editing_endpoint(text, start, "composing", "start")?,
            editing_endpoint(text, end, "composing", "end")?,
        ))
    };
    Ok(TextInputState {
        text: text.to_owned(),
        selection_start,
        selection_end,
        composing,
    })
}

impl crate::Runtime {
    /// Applies a validated Flutter `TextInputClient.updateEditingState`
    /// message to the current editable attachment only. Wrong client IDs
    /// return `Some(false)`; unrelated methods return `None`.
    ///
    /// # Errors
    /// A malformed active message or an unrepresentable range leaves the
    /// controller, composition, and outgoing state unchanged.
    pub fn handle_flutter_text_input_editing_call(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputEditingError> {
        use FlutterTextInputEditingError::InvalidArguments;
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.updateEditingState" {
            return Ok(None);
        }
        let Some(active) = self.active_flutter_editable_client() else {
            return Ok(Some(false));
        };
        let args = call.arguments.as_array().ok_or(InvalidArguments)?;
        let sender = args.first().ok_or(InvalidArguments)?;
        if !sender_matches(sender, active).ok_or(InvalidArguments)? {
            return Ok(Some(false));
        }
        let state = decode_full_editing_state(args.get(1).ok_or(InvalidArguments)?)?;
        self.handle_text_editing_state(state);
        Ok(Some(true))
    }

    /// Delivers `TextInputClient.performPrivateCommand` to the current editor.
    /// Returns `Some(true)` for a valid active message (even without an app
    /// listener), `Some(false)` for a retired client, and `None` for another method.
    ///
    /// # Errors
    /// Invalid messages for the active attachment leave the editor untouched.
    pub fn handle_flutter_text_input_private_command_call(
        &self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputPrivateCommandError> {
        use FlutterTextInputPrivateCommandError::{InvalidAction, InvalidArguments, InvalidData};
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.performPrivateCommand" {
            return Ok(None);
        }
        let Some(active) = self.active_flutter_editable_client() else {
            return Ok(Some(false));
        };
        let args = call.arguments.as_array().ok_or(InvalidArguments)?;
        let sender = args.first().ok_or(InvalidArguments)?;
        if !sender_matches(sender, active).ok_or(InvalidArguments)? {
            return Ok(Some(false));
        }
        let command = args
            .get(1)
            .and_then(Value::as_object)
            .ok_or(InvalidArguments)?;
        let action = command
            .get("action")
            .and_then(Value::as_str)
            .ok_or(InvalidAction)?;
        let empty = Map::new();
        let data = match command.get("data") {
            None | Some(Value::Null) => &empty,
            Some(Value::Object(data)) => data,
            _ => return Err(InvalidData),
        };
        self.dispatch_flutter_app_private_command(action, data);
        Ok(Some(true))
    }

    /// Shows the active editor's autocorrection prompt range without
    /// changing its selection or editing value.
    pub fn handle_flutter_autocorrection_prompt_call(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputLifecycleError> {
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.showAutocorrectionPromptRect" {
            return Ok(None);
        }
        self.adopt_external_focus_change();
        let Some(active) = self.active_flutter_editable_client() else {
            return Ok(Some(false));
        };
        let args = call.arguments.as_array().ok_or(FlutterTextInputLifecycleError::InvalidClientId)?;
        let sender = args.first().ok_or(FlutterTextInputLifecycleError::InvalidClientId)?;
        if !sender_matches(sender, active).ok_or(FlutterTextInputLifecycleError::InvalidClientId)? {
            return Ok(Some(false));
        }
        let endpoint = |index| args.get(index).and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(FlutterTextInputLifecycleError::InvalidPromptRange);
        let (start, end) = (endpoint(1)?, endpoint(2)?);
        let field = self.focused_element().expect("validated active attachment");
        if !self.tree.set_autocorrection_prompt(field, start, end) {
            return Err(FlutterTextInputLifecycleError::InvalidPromptRange);
        }
        self.request_frame();
        Ok(Some(true))
    }

    /// Requests the active native editor toolbar. Returns false for a
    /// stale sender or a toolbar that was already visible.
    pub fn handle_flutter_text_toolbar_call(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputLifecycleError> {
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.showToolbar" {
            return Ok(None);
        }
        self.adopt_external_focus_change();
        let Some(active) = self.active_flutter_editable_client() else {
            return Ok(Some(false));
        };
        let sender = call.arguments.as_array().and_then(|args| args.first())
            .ok_or(FlutterTextInputLifecycleError::InvalidClientId)?;
        if !sender_matches(sender, active).ok_or(FlutterTextInputLifecycleError::InvalidClientId)? {
            return Ok(Some(false));
        }
        Ok(Some(self.show_flutter_editor_toolbar()))
    }

    /// Receives Flutter's keyboard floating-cursor gesture for the current
    /// attachment. Updates without a start are valid wire messages and do
    /// not implicitly start a gesture.
    pub fn handle_flutter_floating_cursor_call(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputFloatingCursorError> {
        use FlutterTextInputFloatingCursorError::InvalidArguments;
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.updateFloatingCursor" {
            return Ok(None);
        }
        self.adopt_external_focus_change();
        let Some(active) = self.active_flutter_editable_client() else {
            return Ok(Some(false));
        };
        let args = call.arguments.as_array().ok_or(InvalidArguments)?;
        let sender = args.first().ok_or(InvalidArguments)?;
        if !sender_matches(sender, active).ok_or(InvalidArguments)? {
            return Ok(Some(false));
        }
        let state = args.get(1).and_then(Value::as_str).ok_or(InvalidArguments)?;
        let position = args.get(2).ok_or(InvalidArguments)?;
        let point = incular_platform::FlutterFloatingCursorPoint::from_flutter_json(state, position)?;
        self.dispatch_flutter_floating_cursor(active, point);
        Ok(Some(true))
    }

    /// Delivers a rich keyboard insertion to the active editor. The data
    /// and URI are passed to the application rather than inserted as text
    /// or read from the filesystem by the runtime.
    pub fn handle_flutter_keyboard_content_call(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterKeyboardContentError> {
        use FlutterKeyboardContentError::InvalidArguments;
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.performAction" {
            return Ok(None);
        }
        let args = call.arguments.as_array().ok_or(InvalidArguments)?;
        if args.get(1).and_then(Value::as_str) != Some("TextInputAction.commitContent") {
            return Ok(None);
        }
        self.adopt_external_focus_change();
        let Some(active) = self.active_flutter_editable_client() else {
            return Ok(Some(false));
        };
        let sender = args.first().ok_or(InvalidArguments)?;
        if !sender_matches(sender, active).ok_or(InvalidArguments)? {
            return Ok(Some(false));
        }
        let data = args.get(2).ok_or(InvalidArguments)?;
        let content = incular_platform::KeyboardInsertedContent::from_flutter_json(data)?;
        Ok(Some(self.dispatch_flutter_keyboard_content(content)))
    }

    /// Handles a Flutter TextInputClient.performSelectors message.
    /// Validate the complete array before editing so a malformed suffix
    /// cannot partially apply earlier selectors.
    pub fn handle_flutter_text_input_selectors_call(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputSelectorError> {
        use FlutterTextInputSelectorError::{InvalidArguments, InvalidSelector};
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.performSelectors" {
            return Ok(None);
        }
        self.adopt_external_focus_change();
        let Some(active) = self.active_flutter_editable_client() else {
            return Ok(Some(false));
        };
        let args = call.arguments.as_array().ok_or(InvalidArguments)?;
        let sender = args.first().ok_or(InvalidArguments)?;
        if !sender_matches(sender, active).ok_or(InvalidArguments)? {
            return Ok(Some(false));
        }
        let selectors = args
            .get(1)
            .and_then(Value::as_array)
            .ok_or(InvalidArguments)?;
        for (index, value) in selectors.iter().enumerate() {
            if !value.is_string() {
                return Err(InvalidSelector { index });
            }
        }
        for selector in selectors.iter().filter_map(Value::as_str) {
            if self.active_flutter_editable_client() != Some(active) {
                break;
            }
            self.dispatch_flutter_text_selector(selector);
        }
        Ok(Some(true))
    }

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
