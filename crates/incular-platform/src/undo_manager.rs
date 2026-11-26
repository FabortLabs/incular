//! Flutter's platform undo manager (`services/undo_manager.dart`): the
//! system's undo and redo commands for the focused text field, and the undo
//! availability the framework reports back to the system.

use std::future::Future;

use serde_json::{Value, json};

use crate::message_codecs::{JsonMethodCodec, JsonMethodCodecError, MethodCall};

/// Flutter's `UndoDirection`: which way a platform undo command moves
/// through the history.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UndoDirection {
    /// Reverts the last change.
    Undo,
    /// Reapplies the last reverted change.
    Redo,
}

impl UndoDirection {
    /// Parses the direction names of Flutter's `UndoManagerClient.handleUndo`
    /// message (`"undo"` and `"redo"`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "undo" => Some(Self::Undo),
            "redo" => Some(Self::Redo),
            _ => None,
        }
    }
}

/// Commands the runtime sends to the platform undo manager. A host applies
/// them to the system's undo state (iOS's `UIUndoManager`, for example) and
/// ignores them when the system has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoManagerCommand {
    /// Flutter's `UndoManager.setUndoState`: whether the focused field can
    /// undo and redo. As in Flutter, the runtime sends it only for the iOS
    /// target platform, when a text field gains focus and whenever its
    /// availability changes while it has focus.
    SetUndoState {
        /// Whether an undo is available.
        can_undo: bool,
        /// Whether a redo is available.
        can_redo: bool,
    },
}

/// Receives native undo or redo while registered with the undo service.
pub trait UndoManagerClient {
    fn undo(&mut self);
    fn redo(&mut self);
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;

    fn handle_platform_undo(&mut self, direction: UndoDirection) {
        match direction {
            UndoDirection::Undo => self.undo(),
            UndoDirection::Redo => self.redo(),
        }
    }
}

/// Opt-in bridge for embedders that use Flutter's JSON undo channel.
pub trait UndoManagerMessenger {
    type Error;

    fn send_undo_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

#[derive(Debug, thiserror::Error)]
pub enum UndoManagerInvocationError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("unimplemented undo manager method: {0}")]
    UnsupportedMethod(String),
    #[error("undo manager arguments must start with a direction string")]
    InvalidArguments,
    #[error("Unknown undo direction: {0}")]
    UnknownDirection(String),
    #[error("There must be a current UndoManagerClient.")]
    NoClient,
}

#[derive(Debug, thiserror::Error)]
pub enum UndoManagerSendError<E> {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error("while sending the UndoManager.setUndoState event: {0}")]
    Transport(E),
}

/// Context delivered to the embedder when an undo-state send fails.
pub struct UndoManagerErrorReport<'a, E> {
    pub error: &'a UndoManagerSendError<E>,
    pub library: &'static str,
    pub context: &'static str,
}

type ErrorReporter<E> = Box<dyn for<'a> FnMut(UndoManagerErrorReport<'a, E>)>;

/// Flutter-compatible coordinator for the current undo client and outbound
/// state. Incular's native editor can also use typed `PlatformEvent::Undo`
/// and `UndoManagerCommand` directly, without a serialized channel.
pub struct UndoManager<M: UndoManagerMessenger> {
    messenger: M,
    client: Option<Box<dyn UndoManagerClient>>,
    error_reporter: Option<ErrorReporter<M::Error>>,
}

impl<M: UndoManagerMessenger> UndoManager<M> {
    pub const CHANNEL: &str = "flutter/undomanager";

    #[must_use]
    pub const fn new(messenger: M) -> Self {
        Self {
            messenger,
            client: None,
            error_reporter: None,
        }
    }

    pub fn set_client(&mut self, client: Option<Box<dyn UndoManagerClient>>) {
        self.client = client;
    }

    #[must_use]
    pub fn client(&self) -> Option<&dyn UndoManagerClient> {
        self.client.as_deref()
    }

    /// Installs an error sink instead of requiring a process-global callback.
    pub fn set_error_reporter(
        &mut self,
        reporter: impl for<'a> FnMut(UndoManagerErrorReport<'a, M::Error>) + 'static,
    ) {
        self.error_reporter = Some(Box::new(reporter));
    }

    pub fn clear_error_reporter(&mut self) {
        self.error_reporter = None;
    }

    /// Routes a native JSON method call to the current client and returns
    /// the standard JSON method success envelope.
    pub fn handle_platform_message(
        &mut self,
        payload: &[u8],
    ) -> Result<Vec<u8>, UndoManagerInvocationError> {
        let MethodCall { method, arguments } = JsonMethodCodec::decode_method_call(payload)?;
        if method != "UndoManagerClient.handleUndo" {
            return Err(UndoManagerInvocationError::UnsupportedMethod(method));
        }
        let direction = arguments
            .as_array()
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .ok_or(UndoManagerInvocationError::InvalidArguments)?;
        let direction = UndoDirection::from_name(direction)
            .ok_or_else(|| UndoManagerInvocationError::UnknownDirection(direction.to_owned()))?;
        let client = self
            .client
            .as_mut()
            .ok_or(UndoManagerInvocationError::NoClient)?;
        client.handle_platform_undo(direction);
        Ok(JsonMethodCodec::encode_success_envelope(&Value::Null)?)
    }

    /// Sends both state flags and reports a transport failure exactly once.
    /// A native Rust caller still receives the original typed error.
    pub async fn set_undo_state(
        &mut self,
        can_undo: bool,
        can_redo: bool,
    ) -> Result<(), UndoManagerSendError<M::Error>> {
        let payload = JsonMethodCodec::encode_method_call(&MethodCall {
            method: "UndoManager.setUndoState".to_owned(),
            arguments: json!({ "canUndo": can_undo, "canRedo": can_redo }),
        })?;
        let result = self
            .messenger
            .send_undo_message(Self::CHANNEL, payload)
            .await
            .map_err(UndoManagerSendError::Transport);
        if let Err(error) = &result
            && let Some(reporter) = &mut self.error_reporter
        {
            reporter(UndoManagerErrorReport {
                error,
                library: "services library",
                context: "while sending the UndoManager.setUndoState event",
            });
        }
        result
    }
}
