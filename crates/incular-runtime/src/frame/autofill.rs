//! Connects Flutter's opt-in tagged autofill channel to mounted editors.

use std::collections::HashMap;

use incular_platform::{
    FlutterTextInputConfiguration, FlutterTextInputControlUpdate, FlutterTextInputControls,
    FlutterTextInputControlsError, TextInputClientId, TextInputState,
    autofill::{
        AutofillConfiguration, AutofillEditingValue, AutofillFieldConfiguration,
        AutofillMessageError, AutofillSelectionAffinity, FlutterAutofillScope,
    },
};
use incular_text::{
    TextAffinity, TextEditingController, TextEditingValue, TextRange, TextSelection,
    byte_offset_at_utf16_boundary,
};
use incular_widgets::{TextFieldInputSnapshot, internal::ElementId};
use serde_json::{Map, Value};

use super::{Runtime, resolve_text_input_action};

impl incular_platform::FlutterMountedScribbleTargets for Runtime {
    fn has_scribble_target(&self, id: &str) -> bool {
        !cfg!(target_arch = "wasm32") && self.tree.flutter_handwriting_target(id).is_some()
    }

    fn focus_scribble_target(&mut self, id: &str, position: incular_core::Offset) {
        let Some(field) = self.tree.flutter_handwriting_target(id) else {
            return;
        };
        let Some((offset, affinity)) = self.tree.text_field_position_at(field, position) else {
            return;
        };
        self.set_focus_with_external_selection(Some(field), false);
        if self.focused != Some(field) {
            return;
        }
        let selection = TextSelection::collapsed(offset);
        if self.tree.text_field_select(field, selection, affinity, self.input_now()) {
            self.text_input_show_requested = true;
            self.report_text_selection_changed(
                field,
                selection,
                incular_text::SelectionChangedCause::StylusHandwriting,
            );
            self.frame_requested = true;
        }
    }

    fn scribble_targets_in_rect(
        &self,
        rect: incular_core::Rect,
    ) -> Vec<(String, incular_core::Rect)> {
        if cfg!(target_arch = "wasm32") {
            return Vec::new();
        }
        self.tree.flutter_handwriting_targets(rect)
    }
}

impl Runtime {
    fn mounted_autofill_scope(&self) -> Option<(ElementId, ElementId)> {
        let focused = self.focused?;
        let active = self.text_input_client?;
        if focused != active.target {
            return None;
        }
        Some((focused, self.tree.autofill_scope(focused)?))
    }

    /// Handles a native keyboard-show command for an attached Flutter host.
    /// Call after layout. The first show publishes geometry, style and the
    /// current editing state before the keyboard request. A show on a stale
    /// client is ignored. Hosts still own client attachment and closing.
    pub fn show_flutter_text_input(
        &self,
        client: TextInputClientId,
        controls: &mut FlutterTextInputControls,
    ) -> Result<bool, FlutterTextInputControlsError> {
        use FlutterTextInputControlUpdate as Update;

        let Some(connection) = self.text_input_client.filter(|connection| {
            connection.client == client
                && self.focused == Some(connection.target)
                && controls.is_attached(client)
        }) else {
            return Ok(false);
        };
        if controls.has_shown(client) {
            return controls.update(client, Update::Show);
        }
        let field = connection.target;
        let (Some((size, transform)), Some(controller), Some(style)) = (
            self.tree.flutter_editable_size_and_transform(field),
            self.tree.text_controller(field),
            self.tree.flutter_text_field_style(field),
        ) else {
            return Ok(false);
        };
        let (editing, composing) = controller.with_value(|value| {
            (editor_wire(&controller, &value.text, value.composing), value.composing)
        });
        let (marked, caret) = self.flutter_input_rects(field, &controller, composing);
        let [a, b, c, d, x, y] = transform.coefficients();
        controls.update(client, Update::EditableSizeAndTransform {
            size,
            transform: [a, b, 0., 0., c, d, 0., 0., 0., 0., 1., 0., x, y, 0., 1.],
        })?;
        self.publish_flutter_handwriting_rects(field, client, controls)?;
        if let Some(marked) = marked {
            controls.update(client, Update::ComposingRect(marked))?;
        }
        controls.publish_style(client, style)?;
        controls.publish_editing_state(client, editing)?;
        let shown = controls.update(client, Update::Show)?;
        if shown && self.text_input_configuration.as_ref().is_some_and(|config| {
            config.client == client && config.traits.autofill_hints.is_some()
        }) {
            controls.update(client, Update::RequestAutofill)?;
        }
        if let Some(caret) = caret {
            controls.update(client, Update::CaretRect(caret))?;
        }
        Ok(shown)
    }

    /// Publishes changed local character, composing and caret bounds after layout, for
    /// the active Flutter host attachment. Unchanged rectangles are not
    /// sent. The host may call this after each subsequent frame.
    pub fn publish_flutter_text_input_rects(
        &self,
        controls: &mut FlutterTextInputControls,
    ) -> Result<bool, FlutterTextInputControlsError> {
        use FlutterTextInputControlUpdate as Update;

        let Some(connection) = self.text_input_client.filter(|connection| {
            self.focused == Some(connection.target) && controls.has_shown(connection.client)
        }) else {
            return Ok(false);
        };
        let field = connection.target;
        let Some(controller) = self.tree.text_controller(field) else {
            return Ok(false);
        };
        let composing = controller.with_value(|value| value.composing);
        let (marked, caret) = self.flutter_input_rects(field, &controller, composing);
        let mut sent = self.publish_flutter_handwriting_rects(field, connection.client, controls)?;
        if let Some(marked) = marked {
            sent |= controls.update(connection.client, Update::ComposingRect(marked))?;
        }
        if let Some(caret) = caret {
            sent |= controls.update(connection.client, Update::CaretRect(caret))?;
        }
        Ok(sent)
    }

    fn publish_flutter_handwriting_rects(
        &self,
        field: ElementId,
        client: TextInputClientId,
        controls: &mut FlutterTextInputControls,
    ) -> Result<bool, FlutterTextInputControlsError> {
        if self.text_shortcut_platform() != crate::TextShortcutPlatform::Ios
            || cfg!(target_arch = "wasm32")
            || controls.is_web_protocol()
        {
            return Ok(false);
        }
        let Some(rects) = self.tree.flutter_handwriting_rects(field) else {
            return Ok(false);
        };
        controls.publish_selection_rects(client, &rects)
    }

    fn flutter_input_rects(
        &self,
        field: ElementId,
        controller: &TextEditingController,
        composing: Option<TextRange>,
    ) -> (Option<incular_core::Rect>, Option<incular_core::Rect>) {
        let local_caret = |offset| {
            self.tree.editable_local_caret_rect(field, offset, TextAffinity::Downstream)
        };
        let marked = composing
            .and_then(|range| self.tree.editable_composing_rect(field, range))
            .or_else(|| local_caret(composing.map_or(0, |range| range.start)));
        let caret = controller.selection_if_set()
            .and_then(|selection| local_caret(selection.range().start));
        (marked, caret)
    }

    /// Publishes the active laid-out editor to an opt-in Flutter host.
    ///
    /// The host must attach the runtime's client ID with
    /// `FlutterTextInputControls::attach_existing`. Sampling after each frame
    /// emits only changed state and geometry. Unlike the native IME caret
    /// snapshot, this wire state preserves an unset selection as (-1, -1).
    pub fn publish_flutter_text_input_snapshot(
        &self,
        controls: &mut FlutterTextInputControls,
    ) -> Result<bool, FlutterTextInputControlsError> {
        let Some(connection) = self.text_input_client.filter(|connection| {
            self.focused == Some(connection.target) && controls.is_attached(connection.client)
        }) else {
            return Ok(false);
        };
        let field = connection.target;
        let Some((size, transform)) = self.tree.flutter_editable_size_and_transform(field) else {
            return Ok(false);
        };
        let Some(controller) = self.tree.text_controller(field) else {
            return Ok(false);
        };
        let editing =
            controller.with_value(|value| editor_wire(&controller, &value.text, value.composing));
        let editing_changed = controls.publish_editing_state(connection.client, editing)?;
        let [a, b, c, d, x, y] = transform.coefficients();
        let geometry_changed = controls.update(
            connection.client,
            FlutterTextInputControlUpdate::EditableSizeAndTransform {
                size,
                transform: [a, b, 0., 0., c, d, 0., 0., 0., 0., 1., 0., x, y, 0., 1.],
            },
        )?;
        Ok(editing_changed || geometry_changed)
    }

    /// Encodes the focused editor's Flutter `TextInput.setClient` message.
    /// Grouped editors include the registered fields; ordinary editors do
    /// not carry an empty scope. Returns nothing without a mounted client.
    pub fn encode_flutter_text_input_client(
        &self,
        input_options: Map<String, Value>,
    ) -> Result<Option<Vec<u8>>, AutofillMessageError> {
        if self.mounted_autofill_scope().is_some() {
            return self.encode_flutter_scoped_autofill_client(input_options);
        }
        let Some((field, client)) = self.focused.zip(self.text_input_client) else {
            return Ok(None);
        };
        if field != client.target {
            return Ok(None);
        }
        let view_id = input_options.get("viewId").cloned().unwrap_or(Value::Null);
        let Some(mut trigger) = self.flutter_autofill_field(field, &view_id) else {
            return Ok(None);
        };
        trigger.input.extend(input_options);
        trigger.encode_set_client(client.client).map(Some)
    }

    /// Encodes a Flutter `TextInput.setClient` message for the mounted group
    /// of the active editor. An embedder can supply additional options for
    /// that editor (for example its platform view ID or delta preference).
    pub fn encode_flutter_scoped_autofill_client(
        &self,
        input_options: Map<String, Value>,
    ) -> Result<Option<Vec<u8>>, AutofillMessageError> {
        let Some((active, scope)) = self.mounted_autofill_scope() else {
            return Ok(None);
        };
        let view_id = input_options.get("viewId").cloned().unwrap_or(Value::Null);
        let Some(mut trigger) = self.flutter_autofill_field(active, &view_id) else {
            return Ok(None);
        };
        trigger.input.extend(input_options);
        let mut group = FlutterAutofillScope::default();
        for id in self.tree.autofill_clients(scope) {
            if id == active {
                group.register(trigger.clone());
            } else if let Some(field) = self.flutter_autofill_field(id, &view_id) {
                group.register(field);
            }
        }
        let client = self
            .text_input_client
            .expect("active grouped editor has a client");
        group.encode_set_client(client.client, &trigger).map(Some)
    }

    /// Applies a complete Flutter tagged update to eligible mounted fields in
    /// the active scope. This may update a nonfocused editor without moving
    /// focus. A stale tag, another group or a read-only field is ignored.
    pub fn handle_flutter_autofill_update(
        &mut self,
        data: &[u8],
    ) -> Result<usize, AutofillMessageError> {
        let Some((active, scope)) = self.mounted_autofill_scope() else {
            return Ok(0);
        };
        let Some(values) = FlutterAutofillScope::decode_tagged_update(data)? else {
            return Ok(0);
        };
        let candidates: HashMap<_, _> = self
            .tree
            .autofill_clients(scope)
            .into_iter()
            .filter(|&id| self.tree.text_field_is_editable(id))
            .filter_map(|id| Some((self.tree.text_field_client_id(id)?, id)))
            .collect();
        let runtime_tag = self.id().to_string();
        let mut ready = Vec::with_capacity(values.len().min(candidates.len()));
        for (tag, wire) in values {
            let Some((host, field)) = tag.split_once(':') else {
                continue;
            };
            if host != runtime_tag {
                continue;
            }
            let Some(id) = field
                .parse::<u64>()
                .ok()
                .filter(|id| id.to_string() == field)
                .and_then(|tag| candidates.get(&tag).copied())
            else {
                continue;
            };
            let Some(controller) = self.tree.text_controller(id) else {
                continue;
            };
            let mut native = prepare_edit(wire)?;
            // Flutter clears the composing range when the editor is not
            // the active IME client, but it keeps the tagged selection.
            if id != active {
                native.value.composing = None;
            }
            ready.push((id, controller, native));
        }
        let count = ready.len();
        for (id, controller, native) in ready {
            if id == active {
                self.cancel_ime_composition();
                let changed = controller.with_value(|old| {
                    old.text != native.value.text || old.composing != native.value.composing
                });
                if changed {
                    self.hide_text_field_overlays(id);
                }
                self.text_input_state = Some(TextInputState {
                    text: native.value.text.clone(),
                    selection_start: native.value.selection.base,
                    selection_end: native.value.selection.extent,
                    composing: native.value.composing.map(|r| (r.start, r.end)),
                });
            }
            controller.apply_autofill_value(
                native.value,
                native.selection_is_set,
                native.affinity,
                native.selection_is_directional,
            );
            controller.reset_caret(self.input_now());
        }
        if count > 0 {
            self.request_frame();
        }
        Ok(count)
    }

    fn flutter_autofill_field(
        &self,
        id: ElementId,
        view_id: &Value,
    ) -> Option<AutofillFieldConfiguration> {
        let snapshot = self.tree.text_field_input_snapshot(id)?;
        let controller = self.tree.text_controller(id)?;
        let input = field_input(&snapshot, view_id);
        let Some(hints) = snapshot.traits.autofill_hints else {
            return Some(AutofillFieldConfiguration {
                input,
                autofill: AutofillConfiguration::disabled(),
            });
        };
        let editor = editor_wire(&controller, &snapshot.text, snapshot.composing);
        Some(AutofillFieldConfiguration {
            input,
            autofill: AutofillConfiguration::new(
                format!("{}:{}", self.id(), snapshot.client_id),
                hints,
                editor,
            ),
        })
    }
}

struct EditableAutofillValue {
    value: TextEditingValue,
    selection_is_set: bool,
    affinity: TextAffinity,
    selection_is_directional: bool,
}

fn prepare_edit(wire: AutofillEditingValue) -> Result<EditableAutofillValue, AutofillMessageError> {
    let byte_at = |units: i64| {
        let units = usize::try_from(units)
            .map_err(|_| AutofillMessageError::InvalidField("edit offset"))?;
        let offset = byte_offset_at_utf16_boundary(&wire.text, units);
        if wire.text[..offset].encode_utf16().count() != units {
            return Err(AutofillMessageError::InvalidField("UTF-16 edit boundary"));
        }
        Ok(offset)
    };
    let selection_is_set = wire.selection != (-1, -1);
    let selection = if selection_is_set {
        TextSelection::new(byte_at(wire.selection.0)?, byte_at(wire.selection.1)?)
    } else {
        TextSelection::collapsed(wire.text.len())
    };
    let composing = if wire.composing == (-1, -1) {
        None
    } else {
        Some(TextRange::new(
            byte_at(wire.composing.0)?,
            byte_at(wire.composing.1)?,
        ))
    };
    Ok(EditableAutofillValue {
        value: TextEditingValue {
            text: wire.text,
            selection,
            composing,
        },
        selection_is_set,
        selection_is_directional: wire.selection_is_directional,
        affinity: match wire.selection_affinity {
            AutofillSelectionAffinity::Upstream => TextAffinity::Upstream,
            AutofillSelectionAffinity::Downstream => TextAffinity::Downstream,
        },
    })
}

fn editor_wire(
    controller: &TextEditingController,
    text: &str,
    composing: Option<TextRange>,
) -> AutofillEditingValue {
    let units = |i| text[..i].encode_utf16().count() as i64;
    AutofillEditingValue {
        text: text.to_owned(),
        selection: controller
            .selection_if_set()
            .map_or((-1, -1), |s| (units(s.base), units(s.extent))),
        composing: composing.map_or((-1, -1), |range| (units(range.start), units(range.end))),
        selection_affinity: match controller.caret_affinity() {
            TextAffinity::Upstream => AutofillSelectionAffinity::Upstream,
            TextAffinity::Downstream => AutofillSelectionAffinity::Downstream,
        },
        selection_is_directional: controller.selection_is_directional(),
    }
}

fn field_input(field: &TextFieldInputSnapshot, view_id: &Value) -> Map<String, Value> {
    let input_action = resolve_text_input_action(field.input_action, field.input_type);
    let mut fields = FlutterTextInputConfiguration {
        input_type: field.input_type,
        read_only: field.read_only,
        obscure_text: field.obscure_text,
        autocorrect: field.traits.autocorrect,
        smart_dashes_type: Some(field.traits.smart_dashes_type),
        smart_quotes_type: Some(field.traits.smart_quotes_type),
        enable_suggestions: field.traits.enable_suggestions,
        enable_interactive_selection: field.traits.enable_interactive_selection,
        input_action,
        text_capitalization: field.traits.text_capitalization,
        keyboard_appearance: field.traits.keyboard_appearance,
        enable_ime_personalized_learning: field.traits.enable_ime_personalized_learning,
        hint_locales: field.traits.hint_locales.clone(),
        enable_inline_prediction: field.traits.enable_inline_prediction,
        ..FlutterTextInputConfiguration::default()
    }
    .to_json();
    // Retain the original channel identifier supplied by this embedder.
    fields.insert("viewId".into(), view_id.clone());
    fields
}
