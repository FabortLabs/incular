use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{FlutterTextInputControls, FlutterTextInputWireCall};
use incular_runtime::Runtime;
use incular_text::{TextEditingController, TextEditingValue, TextRange, TextSelection};
use incular_widgets::{EditableText, FittedBox, SizedBox, Widget, internal::Key};
use serde_json::Value;

fn with_marked_range() -> TextEditingValue {
    TextEditingValue {
        text: "a".repeat(100),
        selection: TextSelection::collapsed(100),
        composing: Some(TextRange::new(0, 10)),
    }
}

fn setup(controller: &TextEditingController, zero: bool) -> (Runtime, FlutterTextInputControls) {
    let field =
        Widget::from(EditableText::new(controller.clone()).text_direction(TextDirection::Ltr))
            .with_key("editor");
    let root: Widget = if zero {
        FittedBox::new(SizedBox::new().width(0.).height(0.).child(field)).into()
    } else {
        field
    };
    let mut runtime = Runtime::new(root).unwrap();
    frame(&mut runtime);
    let editor = runtime
        .tree()
        .element_with_key(&Key::from("editor"))
        .unwrap();
    assert!(runtime.request_text_input_focus_for(editor));
    frame(&mut runtime);
    let (client, config) = runtime.flutter_text_input_configuration().unwrap();
    let mut controls = FlutterTextInputControls::new(false);
    assert!(controls.attach_existing(client, config, |_| {}));
    assert!(
        runtime
            .show_flutter_text_input(client, &mut controls)
            .unwrap()
    );
    (runtime, controls)
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn marked(calls: &[FlutterTextInputWireCall]) -> &Value {
    &calls
        .iter()
        .find(|call| call.method == "TextInput.setMarkedTextRect")
        .expect("one composing rectangle")
        .arguments
}

#[test]
fn changing_composing_range_replaces_the_fallback_caret_with_real_text_bounds() {
    let controller = TextEditingController::with_text("a".repeat(100));
    let (mut runtime, mut controls) = setup(&controller, false);
    let opening = controls.take_platform_calls();
    let caret = marked(&opening);
    assert!(caret["width"].as_f64().unwrap() <= 5.);
    assert!(caret["x"].as_f64().unwrap() <= 1.);

    controller.set_value(with_marked_range());
    frame(&mut runtime);
    assert!(
        runtime
            .publish_flutter_text_input_rects(&mut controls)
            .unwrap()
    );
    let updated = controls.take_platform_calls();
    assert!(marked(&updated)["width"].as_f64().unwrap() >= 10.);
}

#[test]
fn unchanged_composing_bounds_do_not_emit_on_the_next_frame() {
    let controller = TextEditingController::new();
    controller.set_value(with_marked_range());
    let (mut runtime, mut controls) = setup(&controller, false);
    assert!(marked(&controls.take_platform_calls()).is_object());
    frame(&mut runtime);
    assert!(
        !runtime
            .publish_flutter_text_input_rects(&mut controls)
            .unwrap()
    );
    assert!(controls.take_platform_calls().is_empty());
}

#[test]
fn a_zero_paint_transform_still_sends_finite_text_input_rects() {
    let controller = TextEditingController::new();
    controller.set_value(with_marked_range());
    let (_, mut controls) = setup(&controller, true);
    let calls = controls.take_platform_calls();
    let rect = marked(&calls);
    for key in ["x", "y", "width", "height"] {
        assert!(rect[key].as_f64().unwrap().is_finite(), "{rect:?}");
    }
}
