//! User-selected text commands for native or Flutter-compatible hosts.

use serde_json::Value;

use crate::flutter_platform_channel::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, invoke_optional_json,
};

/// An adaptive action on an unobscured text selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextSelectionAction {
    LookUp,
    SearchWeb,
    Share,
}

impl TextSelectionAction {
    #[must_use]
    pub const fn method_name(self) -> &'static str {
        match self {
            Self::LookUp => "LookUp.invoke",
            Self::SearchWeb => "SearchWeb.invoke",
            Self::Share => "Share.invoke",
        }
    }
}

/// A selection command produced after a user presses an adaptive menu item.
/// The host owns the dictionary, search, or sharing interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSelectionActionRequest {
    pub action: TextSelectionAction,
    pub text: String,
}

impl TextSelectionActionRequest {
    /// Sends the exact optional Flutter platform call. This sends the string
    /// as its argument, without trimming it or changing the selection.
    pub async fn send_flutter<M: FlutterPlatformMessenger>(
        self,
        messenger: &mut M,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        invoke_optional_json(
            messenger,
            "flutter/platform",
            self.action.method_name(),
            Value::String(self.text),
        )
        .await
    }
}
