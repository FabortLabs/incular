use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, RuntimeEnvironment, TextDirection};
use incular_core::Size;
use incular_platform::{
    FlutterTextInputControl, FlutterTextInputControlUpdate, FlutterTextInputControls,
    FlutterTextInputStyle, FlutterTextInputWeight,
};
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::{EditableText, MediaQuery, Widget};

fn style_updates(
    style: TextStyle,
    base: RuntimeEnvironment,
    changed: RuntimeEnvironment,
) -> (FlutterTextInputStyle, FlutterTextInputStyle) {
    style_updates_for_direction(style, base, changed, TextDirection::Ltr)
}

fn style_updates_for_direction(
    style: TextStyle,
    base: RuntimeEnvironment,
    changed: RuntimeEnvironment,
    direction: TextDirection,
) -> (FlutterTextInputStyle, FlutterTextInputStyle) {
    let controller = TextEditingController::new();
    let build = |environment| -> Widget {
        MediaQuery::new(
            environment,
            EditableText::new(controller.clone())
                .text_direction(direction)
                .style(style.clone())
                .autofocus(true),
        )
        .into()
    };
    let mut runtime = Runtime::new(build(base)).unwrap();
    let bounds = Constraints::tight(Size::new(260., 160.));
    runtime.run_frame(bounds).unwrap();
    let root = runtime.tree().root().unwrap();
    let focus = runtime.focused_element().unwrap();
    let (client, configuration) = runtime.flutter_text_input_configuration().unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&seen);
    let mut controls = FlutterTextInputControls::new(false);
    controls.set_input_control(FlutterTextInputControl::custom(move |update| {
        if let FlutterTextInputControlUpdate::Style(style) = update {
            observed.borrow_mut().push(style.clone());
        }
    }));

    assert!(
        !runtime
            .publish_flutter_text_input_style(&mut controls)
            .unwrap()
    );
    assert!(controls.attach_existing(client, configuration, |_| {}));
    assert!(
        runtime
            .publish_flutter_text_input_style(&mut controls)
            .unwrap()
    );
    let initial = controls.take_platform_calls();
    assert_eq!(
        initial.iter().map(|call| call.method).collect::<Vec<_>>(),
        ["TextInput.setClient", "TextInput.setStyle"]
    );
    let initial = seen.borrow_mut().pop().unwrap();

    runtime.schedule_update(root, build(changed)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(runtime.focused_element(), Some(focus));
    assert_eq!(
        runtime.flutter_text_input_configuration().unwrap().0,
        client
    );
    assert!(
        runtime
            .publish_flutter_text_input_style(&mut controls)
            .unwrap()
    );
    assert!(
        !runtime
            .publish_flutter_text_input_style(&mut controls)
            .unwrap()
    );
    let updates = controls.take_platform_calls();
    let changed = seen.borrow_mut().pop().unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].method, "TextInput.setStyle");
    assert_eq!(updates[0].arguments, changed.to_json().unwrap());
    assert!(controls.close(client));
    assert!(
        !runtime
            .publish_flutter_text_input_style(&mut controls)
            .unwrap()
    );
    (initial, changed)
}

#[test]
fn platform_style_changes_to_bold_without_reattaching_the_editor() {
    let (before, after) = style_updates(
        TextStyle::new().font_weight(incular_text::FontWeight::NORMAL),
        RuntimeEnvironment::default(),
        RuntimeEnvironment {
            bold_text: true,
            ..RuntimeEnvironment::default()
        },
    );
    assert_eq!(before.font_weight, Some(FlutterTextInputWeight::NORMAL));
    assert_eq!(after.font_weight, Some(FlutterTextInputWeight::BOLD));
}

#[test]
fn platform_style_replaces_authored_letter_spacing_with_the_media_override() {
    let (before, after) = style_updates(
        TextStyle::new().letter_spacing(1.),
        RuntimeEnvironment::default(),
        RuntimeEnvironment {
            letter_spacing_override: Some(5.),
            ..RuntimeEnvironment::default()
        },
    );
    assert_eq!(before.letter_spacing, Some(1.));
    assert_eq!(after.letter_spacing, Some(5.));
}

#[test]
fn platform_style_replaces_authored_word_spacing_with_the_media_override() {
    let (before, after) = style_updates(
        TextStyle::new().word_spacing(Some(2.)),
        RuntimeEnvironment::default(),
        RuntimeEnvironment {
            word_spacing_override: Some(10.),
            ..RuntimeEnvironment::default()
        },
    );
    assert_eq!(before.word_spacing, Some(2.));
    assert_eq!(after.word_spacing, Some(10.));
}

#[test]
fn platform_style_reports_laid_out_line_height_after_the_media_override() {
    let (before, after) = style_updates(
        TextStyle::new().font_size(20.).line_height_multiplier(1.),
        RuntimeEnvironment {
            line_height_scale_factor_override: Some(1.),
            ..RuntimeEnvironment::default()
        },
        RuntimeEnvironment {
            line_height_scale_factor_override: Some(2.),
            ..RuntimeEnvironment::default()
        },
    );
    assert_eq!(before.font_size, Some(20.));
    assert_eq!(after.font_size, Some(20.));
    assert_eq!(before.line_height, Some(20.));
    assert_eq!(after.line_height, Some(40.));
}

#[test]
fn initial_style_wire_preserves_rtl_font_fields_and_media_bold_override() {
    use incular_text::FontWeight;
    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    incular_text::register_application_font_as("Roboto", std::fs::read(font).unwrap());
    let (before, after) = style_updates_for_direction(
        TextStyle::new()
            .font_size(20.)
            .font_family("Roboto")
            .font_weight(FontWeight(600)),
        RuntimeEnvironment::default(),
        RuntimeEnvironment {
            bold_text: true,
            ..RuntimeEnvironment::default()
        },
        TextDirection::Rtl,
    );
    let expected = |index| {
        serde_json::json!({
            "fontSize": 20.0,
            "fontFamily": "Roboto",
            "fontWeightIndex": index,
            "textAlignIndex": 4,
            "textDirectionIndex": 0,
            "letterSpacing": null,
            "wordSpacing": null,
            "lineHeight": 20.0,
        })
    };
    assert_eq!(before.to_json().unwrap(), expected(5));
    assert_eq!(after.to_json().unwrap(), expected(6));
}

#[test]
fn changing_keyboard_type_updates_the_existing_client_without_another_show() {
    use incular_core::{InputEvent, Offset, PointerPhase};
    use incular_platform::{TextInputCommand, TextInputType};
    use incular_widgets::FocusNode;
    use std::time::{Duration, Instant};

    let controller = TextEditingController::with_text("Lorem");
    let focus = FocusNode::new();
    let editor = |input_kind| {
        Widget::from(
            EditableText::new(controller.clone())
                .focus_node(focus.clone())
                .text_direction(TextDirection::Ltr)
                .input_type(input_kind),
        )
    };
    let mut runtime = Runtime::new(editor(TextInputType::Text)).unwrap();
    let viewport = Constraints::tight(Size::new(260., 160.));
    let start = Instant::now();
    runtime.run_frame_at(viewport, start).unwrap();
    for (index, phase) in [PointerPhase::Down, PointerPhase::Up]
        .into_iter()
        .enumerate()
    {
        let _ = runtime.handle_input_at(
            InputEvent::Pointer {
                phase,
                position: Offset::new(2., 5.),
            },
            start + Duration::from_millis(index as u64 + 16),
        );
    }
    runtime
        .run_frame_at(viewport, start + Duration::from_millis(32))
        .unwrap();
    let (client, prior) = runtime.flutter_text_input_configuration().unwrap();
    assert_eq!(prior.to_json()["inputType"]["name"], "TextInputType.text");
    let mut controls = FlutterTextInputControls::new(false);
    assert!(controls.attach_existing(client, prior, |_| {}));
    controls.take_platform_calls();
    runtime.take_text_input_commands();

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(
            root,
            editor(TextInputType::Number {
                signed: false,
                decimal: false,
            }),
        )
        .unwrap();
    runtime
        .run_frame_at(viewport, start + Duration::from_millis(48))
        .unwrap();
    let (retained, next) = runtime.flutter_text_input_configuration().unwrap();
    assert_eq!(client, retained);
    assert!(focus.has_primary_focus());
    assert_eq!(controller.text(), "Lorem");
    assert!(
        !runtime
            .take_text_input_commands()
            .iter()
            .any(|command| matches!(
                command,
                TextInputCommand::Show { .. } | TextInputCommand::Clear { .. }
            ))
    );
    controls
        .update(client, FlutterTextInputControlUpdate::Configuration(next))
        .unwrap();
    let log = controls.take_platform_calls();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].method, "TextInput.updateConfig");
    assert_eq!(
        log[0].arguments["inputType"]["name"],
        "TextInputType.number"
    );
}
