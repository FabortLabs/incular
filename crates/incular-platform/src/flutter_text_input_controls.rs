//! The Flutter text-input attachment and custom-control forwarding protocol.

use std::rc::Rc;

use incular_config::TextDirection;
use incular_core::{Rect, Size};
use serde_json::{Value, json};

use crate::{
    FlutterJsonMethodChannel, FlutterPlatformChannelError, FlutterPlatformMessenger,
    FlutterTextInputConfiguration, FlutterTextInputStyle, InvalidFlutterTextInputStyleNumber,
    TextInputClientId, TextInputType,
    autofill::{AutofillEditingValue, AutofillMessageError},
    message_codecs::{JsonMethodCodec, JsonMethodCodecError},
};

type ControlCallback = dyn Fn(&FlutterTextInputControlUpdate);

/// Which keyboard UI is current. The platform transport remains attached
/// even when a custom visual control is used or hidden.
#[derive(Clone)]
pub enum FlutterTextInputControl {
    Platform,
    Hidden,
    Custom(Rc<ControlCallback>),
}

impl FlutterTextInputControl {
    pub fn custom(callback: impl Fn(&FlutterTextInputControlUpdate) + 'static) -> Self {
        Self::Custom(Rc::new(callback))
    }
}

impl PartialEq for FlutterTextInputControl {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Platform, Self::Platform) | (Self::Hidden, Self::Hidden) => true,
            (Self::Custom(left), Self::Custom(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}

#[derive(Clone)]
pub enum FlutterTextInputClientEvent {
    ControlChanged {
        old: FlutterTextInputControl,
        new: FlutterTextInputControl,
    },
    EditingValue(AutofillEditingValue),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlutterSelectionRect {
    pub position: usize,
    pub bounds: Rect,
    pub direction: TextDirection,
}

#[derive(Clone, Debug)]
pub enum FlutterTextInputControlUpdate {
    Attach {
        client: TextInputClientId,
        configuration: FlutterTextInputConfiguration,
    },
    Detach(TextInputClientId),
    Configuration(FlutterTextInputConfiguration),
    EditingState(AutofillEditingValue),
    Show,
    Hide,
    CaretRect(Rect),
    ComposingRect(Rect),
    EditableSizeAndTransform {
        size: Size,
        transform: [f64; 16],
    },
    SelectionRects(Vec<FlutterSelectionRect>),
    Style(FlutterTextInputStyle),
    RequestAutofill,
    FinishAutofillContext {
        should_save: bool,
    },
}

/// The corresponding outbound Flutter wire operation.
#[derive(Clone, Debug, PartialEq)]
pub struct FlutterTextInputWireCall {
    pub method: &'static str,
    pub arguments: Value,
}

impl FlutterTextInputWireCall {
    pub async fn send<M: FlutterPlatformMessenger>(
        &self,
        messenger: &mut M,
    ) -> Result<(), FlutterPlatformChannelError<M::Error>> {
        FlutterJsonMethodChannel::new("flutter/textinput")
            .invoke(messenger, self.method, self.arguments.clone())
            .await?;
        Ok(())
    }

    /// Reports an outbound failure once with the Flutter operation context.
    /// The original platform exception remains the error's typed source.
    pub async fn send_reporting<M: FlutterPlatformMessenger>(
        &self,
        messenger: &mut M,
        report: impl FnOnce(FlutterTextInputWireError<M::Error>),
    ) {
        if let Err(error) = self.send(messenger).await {
            report(FlutterTextInputWireError {
                context: self.error_context(),
                error,
            });
        }
    }

    fn error_context(&self) -> &'static str {
        match self.method {
            "TextInput.setClient" => "while attaching the text input client",
            "TextInput.clearClient" => "while detaching the text input client",
            "TextInput.updateConfig" => "while updating text input configuration",
            "TextInput.setEditingState" => "while setting text input editing state",
            "TextInput.show" => "while showing the text input client",
            "TextInput.hide" => "while hiding the text input client",
            "TextInput.setEditableSizeAndTransform" => {
                "while setting text input size and transform"
            }
            "TextInput.setMarkedTextRect" => "while setting text input composing rect",
            "TextInput.setCaretRect" => "while setting text input caret rect",
            "TextInput.setSelectionRects" => "while setting text input selection rects",
            "TextInput.setStyle" => "while updating text input style",
            "TextInput.requestAutofill" => "while requesting autofill",
            "TextInput.finishAutofillContext" => "while finishing autofill context",
            _ => "while sending a text input request",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{context}: {error}")]
pub struct FlutterTextInputWireError<E> {
    pub context: &'static str,
    #[source]
    pub error: FlutterPlatformChannelError<E>,
}

#[derive(Debug, thiserror::Error)]
pub enum FlutterTextInputControlsError {
    #[error(transparent)]
    Codec(#[from] JsonMethodCodecError),
    #[error(transparent)]
    EditingValue(#[from] AutofillMessageError),
    #[error(transparent)]
    Style(#[from] InvalidFlutterTextInputStyleNumber),
    #[error("expected a client ID and a text editing object")]
    InvalidEditingMessage,
    #[error("attach and detach must go through the attachment operations")]
    InvalidUpdate,
    #[error("text input geometry must be finite")]
    InvalidGeometry,
}

struct Attachment {
    id: TextInputClientId,
    client: Box<dyn Fn(FlutterTextInputClientEvent)>,
    geometry: GeometryCache,
    editing: Option<AutofillEditingValue>,
}

#[derive(Default)]
struct GeometryCache {
    caret: Option<Rect>,
    composing: Option<Rect>,
    size_transform: Option<(Size, [f64; 16])>,
    selection: Option<Vec<FlutterSelectionRect>>,
}

impl GeometryCache {
    fn needs_dispatch(&self, update: &FlutterTextInputControlUpdate) -> bool {
        use FlutterTextInputControlUpdate::*;
        match update {
            CaretRect(rect) => self.caret.as_ref() != Some(rect),
            ComposingRect(rect) => self.composing.as_ref() != Some(rect),
            EditableSizeAndTransform { size, transform } => {
                self.size_transform.as_ref() != Some(&(*size, *transform))
            }
            SelectionRects(rects) => self.selection.as_ref() != Some(rects),
            _ => true,
        }
    }

    fn record(&mut self, update: &FlutterTextInputControlUpdate) {
        use FlutterTextInputControlUpdate::*;
        match update {
            CaretRect(rect) => self.caret = Some(*rect),
            ComposingRect(rect) => self.composing = Some(*rect),
            EditableSizeAndTransform { size, transform } => {
                self.size_transform = Some((*size, *transform));
            }
            SelectionRects(rects) => self.selection = Some(rects.clone()),
            _ => {}
        }
    }
}

/// One host's optional Flutter-compatible text input connection manager.
pub struct FlutterTextInputControls {
    current: FlutterTextInputControl,
    attached: Option<Attachment>,
    next_client: u64,
    hide_pending: bool,
    web_protocol: bool,
    messages: Vec<FlutterTextInputWireCall>,
}

impl FlutterTextInputControls {
    /// Set web_protocol for the browser's custom-keyboard multiline field.
    #[must_use]
    pub fn new(web_protocol: bool) -> Self {
        Self {
            current: FlutterTextInputControl::Platform,
            attached: None,
            next_client: 1,
            hide_pending: false,
            web_protocol,
            messages: Vec::new(),
        }
    }

    pub fn set_input_control(&mut self, new: FlutterTextInputControl) {
        if self.current == new {
            return;
        }
        let old = std::mem::replace(&mut self.current, new.clone());
        if let Some(attachment) = &self.attached {
            (attachment.client)(FlutterTextInputClientEvent::ControlChanged { old, new });
        }
    }

    pub fn attach(
        &mut self,
        configuration: FlutterTextInputConfiguration,
        client: impl Fn(FlutterTextInputClientEvent) + 'static,
    ) -> TextInputClientId {
        let id = TextInputClientId::new(self.next_client);
        let bound = self.attach_existing(id, configuration, client);
        assert!(bound, "text input client IDs exhausted");
        id
    }

    /// Attaches the client ID already assigned by the mounted editor's
    /// runtime. An older or already issued ID cannot replace the connection.
    pub fn attach_existing(
        &mut self,
        id: TextInputClientId,
        configuration: FlutterTextInputConfiguration,
        client: impl Fn(FlutterTextInputClientEvent) + 'static,
    ) -> bool {
        if id.get() < self.next_client {
            return false;
        }
        let Some(next) = id.get().checked_add(1) else {
            return false;
        };
        self.next_client = next;
        self.attached = Some(Attachment {
            id,
            client: Box::new(client),
            geometry: GeometryCache::default(),
            editing: None,
        });
        let arguments = json!([id.get(), self.platform_configuration(&configuration)]);
        self.forward(
            FlutterTextInputControlUpdate::Attach {
                client: id,
                configuration,
            },
            ("TextInput.setClient", arguments),
        );
        true
    }

    pub fn close(&mut self, id: TextInputClientId) -> bool {
        if !self.is_attached(id) {
            return false;
        }
        self.forward(
            FlutterTextInputControlUpdate::Detach(id),
            ("TextInput.clearClient", Value::Null),
        );
        self.attached = None;
        self.hide_pending = true;
        true
    }

    /// Call at the next host microtask. A newly attached field prevents a
    /// delayed hide from dismissing the next editor's keyboard.
    pub fn flush_pending_hide(&mut self) -> bool {
        if !std::mem::take(&mut self.hide_pending) || self.attached.is_some() {
            return false;
        }
        self.forward(
            FlutterTextInputControlUpdate::Hide,
            ("TextInput.hide", Value::Null),
        );
        true
    }

    pub fn update(
        &mut self,
        id: TextInputClientId,
        update: FlutterTextInputControlUpdate,
    ) -> Result<bool, FlutterTextInputControlsError> {
        let Some(attachment) = self.attached.as_ref().filter(|attached| attached.id == id) else {
            return Ok(false);
        };
        if !attachment.geometry.needs_dispatch(&update) {
            return Ok(false);
        }
        let (method, arguments) = self.encode(&update)?;
        let attachment = self.attached.as_mut().expect("validated attachment");
        attachment.geometry.record(&update);
        if let FlutterTextInputControlUpdate::EditingState(value) = &update {
            attachment.editing = Some(value.clone());
        }
        self.forward(update, (method, arguments));
        Ok(true)
    }

    /// Publishes a mounted editor snapshot only when the value changed.
    /// Use [Self::update] to deliberately resend an editing state.
    pub fn publish_editing_state(
        &mut self,
        id: TextInputClientId,
        value: AutofillEditingValue,
    ) -> Result<bool, FlutterTextInputControlsError> {
        let Some(attachment) = self
            .attached
            .as_ref()
            .filter(|attachment| attachment.id == id)
        else {
            return Ok(false);
        };
        if attachment.editing.as_ref() == Some(&value) {
            return Ok(false);
        }
        self.update(id, FlutterTextInputControlUpdate::EditingState(value))
    }

    /// A host edit reaches the client and custom control; it is not echoed
    /// back to the platform that produced it.
    pub fn handle_platform_edit(
        &mut self,
        data: &[u8],
    ) -> Result<Option<bool>, FlutterTextInputControlsError> {
        let call = JsonMethodCodec::decode_method_call(data)?;
        if call.method != "TextInputClient.updateEditingState" {
            return Ok(None);
        }
        let Some(attachment) = &mut self.attached else {
            return Ok(Some(false));
        };
        use FlutterTextInputControlsError::InvalidEditingMessage;
        let args = call.arguments.as_array().ok_or(InvalidEditingMessage)?;
        let sender = args.first().ok_or(InvalidEditingMessage)?;
        let matches = sender
            .as_u64()
            .map(|id| id == attachment.id.get())
            .or_else(|| sender.as_i64().map(|id| cfg!(debug_assertions) && id == -1))
            .ok_or(InvalidEditingMessage)?;
        if !matches {
            return Ok(Some(false));
        }
        let value = AutofillEditingValue::from_json(args.get(1).ok_or(InvalidEditingMessage)?)?;
        attachment.editing = Some(value.clone());
        if let FlutterTextInputControl::Custom(control) = &self.current {
            control(&FlutterTextInputControlUpdate::EditingState(value.clone()));
        }
        (attachment.client)(FlutterTextInputClientEvent::EditingValue(value));
        Ok(Some(true))
    }

    /// Finishes autofill even when the last field is already detached.
    pub fn finish_autofill_context(&mut self, should_save: bool) {
        self.forward(
            FlutterTextInputControlUpdate::FinishAutofillContext { should_save },
            ("TextInput.finishAutofillContext", json!(should_save)),
        );
    }

    pub fn take_platform_calls(&mut self) -> Vec<FlutterTextInputWireCall> {
        std::mem::take(&mut self.messages)
    }

    #[must_use]
    pub fn is_attached(&self, id: TextInputClientId) -> bool {
        self.attached
            .as_ref()
            .is_some_and(|attachment| attachment.id == id)
    }

    fn forward(
        &mut self,
        update: FlutterTextInputControlUpdate,
        (method, arguments): (&'static str, Value),
    ) {
        self.messages
            .push(FlutterTextInputWireCall { method, arguments });
        if let FlutterTextInputControl::Custom(control) = &self.current {
            control(&update);
        }
    }

    fn platform_configuration(&self, config: &FlutterTextInputConfiguration) -> Value {
        let mut value = config.to_json();
        if self.current != FlutterTextInputControl::Platform {
            let mut input_type = TextInputType::None.to_flutter_json();
            if self.web_protocol {
                input_type["isMultiline"] = json!(config.input_type == TextInputType::Multiline);
            }
            value.insert("inputType".into(), input_type);
        }
        Value::Object(value)
    }

    fn encode(
        &self,
        update: &FlutterTextInputControlUpdate,
    ) -> Result<(&'static str, Value), FlutterTextInputControlsError> {
        use FlutterTextInputControlUpdate::*;
        Ok(match update {
            Attach { .. } | Detach(_) => return Err(FlutterTextInputControlsError::InvalidUpdate),
            Configuration(config) => (
                "TextInput.updateConfig",
                self.platform_configuration(config),
            ),
            EditingState(value) => ("TextInput.setEditingState", value.to_json()?),
            Show => ("TextInput.show", Value::Null),
            Hide => ("TextInput.hide", Value::Null),
            CaretRect(rect) => ("TextInput.setCaretRect", rect_json(*rect)?),
            ComposingRect(rect) => ("TextInput.setMarkedTextRect", rect_json(*rect)?),
            EditableSizeAndTransform { size, transform } => {
                if !size.width.is_finite()
                    || !size.height.is_finite()
                    || !transform.iter().all(|value| value.is_finite())
                {
                    return Err(FlutterTextInputControlsError::InvalidGeometry);
                }
                (
                    "TextInput.setEditableSizeAndTransform",
                    json!({"width": size.width, "height": size.height, "transform": transform.as_slice()}),
                )
            }
            SelectionRects(rects) => {
                let mut values = Vec::with_capacity(rects.len());
                for rect in rects {
                    let dims = rect_components(rect.bounds)?;
                    let direction = match rect.direction {
                        TextDirection::Rtl => 0,
                        TextDirection::Ltr => 1,
                    };
                    values.push(json!([
                        dims[0],
                        dims[1],
                        dims[2],
                        dims[3],
                        rect.position,
                        direction
                    ]));
                }
                ("TextInput.setSelectionRects", Value::Array(values))
            }
            Style(style) => ("TextInput.setStyle", style.to_json()?),
            RequestAutofill => ("TextInput.requestAutofill", Value::Null),
            FinishAutofillContext { should_save } => {
                ("TextInput.finishAutofillContext", json!(should_save))
            }
        })
    }
}

fn rect_components(rect: Rect) -> Result<[f32; 4], FlutterTextInputControlsError> {
    let values = [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    ];
    if !values.iter().all(|value| value.is_finite()) {
        return Err(FlutterTextInputControlsError::InvalidGeometry);
    }
    Ok(values)
}

fn rect_json(rect: Rect) -> Result<Value, FlutterTextInputControlsError> {
    let [x, y, width, height] = rect_components(rect)?;
    Ok(json!({"x": x, "y": y, "width": width, "height": height}))
}
