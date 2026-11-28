use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_platform::{FlutterTextInputControls, FlutterTextInputWireCall, TextInputClientId};
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::{
    Column, EditableText, SizedBox, TranslationController, Widget, internal::Key,
};
use serde_json::{Value, json};

fn geometry(calls: &[FlutterTextInputWireCall]) -> &Value {
    &calls
        .iter()
        .find(|wire| wire.method == "TextInput.setEditableSizeAndTransform")
        .expect("the host received editor geometry")
        .arguments
}

fn box_at(height: f64, x: f64, y: f64) -> Value {
    json!({
        "width": 800.0,
        "height": height,
        "transform": [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            x, y, 0.0, 1.0,
        ]
    })
}

fn show(
    runtime: &mut Runtime,
    controls: &mut FlutterTextInputControls,
    previous: Option<TextInputClientId>,
    key: &str,
) -> TextInputClientId {
    let id = runtime.tree().element_with_key(&Key::from(key)).unwrap();
    assert!(runtime.request_text_input_focus_for(id));
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let (client, configuration) = runtime.flutter_text_input_configuration().unwrap();
    if let Some(previous) = previous {
        assert!(controls.close(previous));
    }
    assert!(controls.attach_existing(client, configuration, |_| {}));
    assert!(runtime.show_flutter_text_input(client, controls).unwrap());
    client
}

#[test]
fn showing_the_keyboard_for_a_tight_editor_sends_its_full_size_at_identity() {
    let root = Widget::from(
        EditableText::new(TextEditingController::new()).text_direction(TextDirection::Ltr),
    )
    .with_key("editor");
    let mut runtime = Runtime::new(root).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let mut controls = FlutterTextInputControls::new(false);
    let _ = show(&mut runtime, &mut controls, None, "editor");
    assert_eq!(
        geometry(&controls.take_platform_calls()),
        &box_at(600., 0., 0.)
    );
}

fn pinned_style() -> Option<TextStyle> {
    let path = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH")?;
    const FONT: &str = "FlutterTest Host Text Geometry";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    Some(TextStyle::new().font_family(FONT).font_size(14.))
}

#[test]
fn changing_connection_resets_its_size_and_transform_in_both_directions() {
    let Some(style) = pinned_style() else {
        return;
    };
    let field = |value, lines| {
        let field = EditableText::new(TextEditingController::with_text(value))
            .style(style.clone())
            .text_direction(TextDirection::Ltr);
        Widget::from(if lines {
            field.min_lines(Some(10)).max_lines(Some(20))
        } else {
            field
        })
        .with_key(value)
    };
    let root = Column::new([
        field("Text1", false),
        Widget::from(SizedBox::new().height(200.)),
        field("Text2", true),
        Widget::from(SizedBox::new().height(100.)),
    ])
    .text_direction(TextDirection::Ltr);
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let mut controls = FlutterTextInputControls::new(false);
    let mut previous = None;
    for (name, height, y) in [
        ("Text1", 14., 0.),
        ("Text2", 140., 214.),
        ("Text1", 14., 0.),
    ] {
        previous = Some(show(&mut runtime, &mut controls, previous, name));
        assert_eq!(
            geometry(&controls.take_platform_calls()),
            &box_at(height, 0., y)
        );
    }
}

#[test]
fn moving_the_mounted_field_sends_only_its_changed_geometry() {
    let Some(style) = pinned_style() else {
        return;
    };
    let motion = TranslationController::new();
    let field = Widget::from(
        EditableText::new(TextEditingController::new())
            .style(style)
            .text_direction(TextDirection::Ltr),
    )
    .with_key("editor");
    let root =
        Column::new([Widget::translate(motion.clone(), field)]).text_direction(TextDirection::Ltr);
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let mut controls = FlutterTextInputControls::new(false);
    let _ = show(&mut runtime, &mut controls, None, "editor");
    assert_eq!(
        geometry(&controls.take_platform_calls()),
        &box_at(14., 0., 0.)
    );

    assert!(motion.set_offset(Offset::new(10., 20.)));
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    assert!(
        runtime
            .publish_flutter_text_input_snapshot(&mut controls)
            .unwrap()
    );
    let calls = controls.take_platform_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(geometry(&calls), &box_at(14., 10., 20.));
}
