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
    let controller = TextEditingController::new();
    let build = |environment| -> Widget {
        MediaQuery::new(
            environment,
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
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
