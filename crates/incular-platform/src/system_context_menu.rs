//! State and platform-channel messages for system-owned text editing menus.

use std::{collections::HashMap, rc::Rc};

use incular_core::Rect;
use serde_json::{Value, json};

use crate::{
    FlutterPlatformChannelError, FlutterPlatformMessenger, TextInputClientId,
    flutter_platform_channel::invoke_optional_json,
    message_codecs::{JsonMethodCodec, JsonMethodCodecError},
};

const CHANNEL: &str = "flutter/platform";
const SHOW: &str = "ContextMenu.showSystemContextMenu";
const HIDE: &str = "ContextMenu.hideSystemContextMenu";
const DISMISS: &str = "ContextMenu.onDismissSystemContextMenu";
const ACTION: &str = "ContextMenu.onPerformCustomAction";

/// An iOS system text-menu item. The platform supplies the title and action
/// for the clipboard commands and Live Text.
#[derive(Clone)]
pub enum SystemContextMenuItem {
    Copy,
    Cut,
    Paste,
    SelectAll,
    LookUp(String),
    SearchWeb(String),
    Share(String),
    LiveText,
    Custom {
        title: String,
        on_pressed: Rc<dyn Fn()>,
    },
}

impl PartialEq for SystemContextMenuItem {
    fn eq(&self, other: &Self) -> bool {
        use SystemContextMenuItem::*;
        match (self, other) {
            (Copy, Copy)
            | (Cut, Cut)
            | (Paste, Paste)
            | (SelectAll, SelectAll)
            | (LiveText, LiveText) => true,
            (LookUp(a), LookUp(b)) | (SearchWeb(a), SearchWeb(b)) | (Share(a), Share(b)) => a == b,
            (
                Custom {
                    title: a,
                    on_pressed: action_a,
                },
                Custom {
                    title: b,
                    on_pressed: action_b,
                },
            ) => a == b && Rc::ptr_eq(action_a, action_b),
            _ => false,
        }
    }
}

impl SystemContextMenuItem {
    fn encode(&self) -> Value {
        let (kind, title) = match self {
            Self::Copy => ("copy", None),
            Self::Cut => ("cut", None),
            Self::Paste => ("paste", None),
            Self::SelectAll => ("selectAll", None),
            Self::LookUp(title) => ("lookUp", Some(title)),
            Self::SearchWeb(title) => ("searchWeb", Some(title)),
            Self::Share(title) => ("share", Some(title)),
            Self::LiveText => ("captureTextFromCamera", None),
            Self::Custom { .. } => unreachable!("custom items carry a menu-generation action ID"),
        };
        // iOS uses the type/title for built-ins; only custom items consume an
        // action ID. Keep the unused numeric legacy field on the wire.
        let mut value = json!({"callbackId": 0, "type": kind});
        if let Some(title) = title {
            value["title"] = json!(title);
        }
        value
    }
}

/// Identity of a text menu that can be shown and hidden independently.
#[derive(Clone, Default)]
pub struct SystemContextMenuController(Rc<Controller>);

#[derive(Default)]
struct Controller {
    on_system_hide: Option<Box<dyn Fn()>>,
}

impl SystemContextMenuController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_system_hide_handler(handler: impl Fn() + 'static) -> Self {
        Self(Rc::new(Controller {
            on_system_hide: Some(Box::new(handler)),
        }))
    }

    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

struct VisibleMenu {
    owner: SystemContextMenuController,
    target: Rect,
    items: Vec<SystemContextMenuItem>,
    callbacks: HashMap<String, Rc<dyn Fn()>>,
}

/// The one system menu shared by the controllers on a platform channel.
#[derive(Default)]
pub struct SystemContextMenus {
    visible: Option<VisibleMenu>,
    next_action_id: u64,
}

/// A show or hide operation produced by a synchronous menu state transition.
/// The embedding host sends the operation on its platform messenger.
#[must_use]
pub struct SystemContextMenuCommand {
    method: &'static str,
    arguments: Value,
}

impl SystemContextMenuCommand {
    pub async fn send<M: FlutterPlatformMessenger>(
        self,
        messenger: &mut M,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        invoke_optional_json(messenger, CHANNEL, self.method, self.arguments).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SystemContextMenuError {
    #[error("the system menu requires an active text input connection")]
    NoTextInput,
    #[error("the system menu requires at least one item")]
    EmptyItems,
    #[error("the system menu target must be finite")]
    InvalidTarget,
    #[error("another controller owns the visible system menu")]
    AlreadyVisible,
    #[error("there is no visible system menu to receive a platform message")]
    NoVisibleMenu,
    #[error("the visible system menu has no action with ID {0}")]
    UnknownAction(String),
    #[error("malformed custom context menu action")]
    InvalidAction,
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
}

impl SystemContextMenus {
    pub fn is_visible(&self, controller: &SystemContextMenuController) -> bool {
        self.visible
            .as_ref()
            .is_some_and(|menu| menu.owner.same(controller))
    }

    pub fn show_with_items(
        &mut self,
        controller: &SystemContextMenuController,
        active_client: Option<TextInputClientId>,
        target: Rect,
        items: Vec<SystemContextMenuItem>,
    ) -> Result<Option<SystemContextMenuCommand>, SystemContextMenuError> {
        if items.is_empty() {
            return Err(SystemContextMenuError::EmptyItems);
        }
        if active_client.is_none() {
            return Err(SystemContextMenuError::NoTextInput);
        }
        let (x, y, width, height) = (
            target.origin.x,
            target.origin.y,
            target.size.width,
            target.size.height,
        );
        if ![x, y, width, height].into_iter().all(f32::is_finite) {
            return Err(SystemContextMenuError::InvalidTarget);
        }
        if let Some(visible) = &self.visible {
            if target == visible.target && items == visible.items {
                return Ok(None);
            }
            if !visible.owner.same(controller) {
                return Err(SystemContextMenuError::AlreadyVisible);
            }
        }

        let mut callbacks = HashMap::new();
        let mut encoded = Vec::with_capacity(items.len());
        for item in &items {
            if let SystemContextMenuItem::Custom { title, on_pressed } = item {
                let id = self.next_action_id.to_string();
                self.next_action_id += 1;
                encoded.push(json!({"id": id, "title": title, "type": "custom"}));
                callbacks.insert(id, on_pressed.clone());
            } else {
                encoded.push(item.encode());
            }
        }
        self.visible = Some(VisibleMenu {
            owner: controller.clone(),
            target,
            items,
            callbacks,
        });
        Ok(Some(SystemContextMenuCommand {
            method: SHOW,
            arguments: json!({
                "targetRect": {"x": x, "y": y, "width": width, "height": height},
                "items": encoded,
            }),
        }))
    }

    pub fn hide(
        &mut self,
        controller: &SystemContextMenuController,
    ) -> Option<SystemContextMenuCommand> {
        if !self.is_visible(controller) {
            return None;
        }
        self.visible = None;
        Some(SystemContextMenuCommand {
            method: HIDE,
            arguments: Value::Null,
        })
    }

    /// Handles a corresponding inbound method call. Other platform methods
    /// remain available for the embedding host to dispatch.
    pub fn handle_platform_message(
        &mut self,
        payload: &[u8],
    ) -> Result<bool, SystemContextMenuError> {
        let call = JsonMethodCodec::decode_method_call(payload)?;
        match call.method.as_str() {
            DISMISS => {
                let menu = self
                    .visible
                    .take()
                    .ok_or(SystemContextMenuError::NoVisibleMenu)?;
                if let Some(callback) = &menu.owner.0.on_system_hide {
                    callback();
                }
            }
            ACTION => {
                let menu = self
                    .visible
                    .as_ref()
                    .ok_or(SystemContextMenuError::NoVisibleMenu)?;
                let id = call
                    .arguments
                    .get(1)
                    .and_then(Value::as_str)
                    .ok_or(SystemContextMenuError::InvalidAction)?;
                let callback = menu
                    .callbacks
                    .get(id)
                    .ok_or_else(|| SystemContextMenuError::UnknownAction(id.to_owned()))?;
                callback();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
