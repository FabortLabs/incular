use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::FlutterTextInputControls;
use incular_runtime::Runtime;
use incular_text::{FontWeight, TextEditingController, TextStyle};
use incular_widgets::{Column, EditableText, FocusNode, Widget, internal::Key};

fn font_fixture() -> Option<Vec<u8>> {
    let path = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH")?;
    Some(std::fs::read(path).unwrap())
}

#[test]
fn rebuilding_active_style_sends_one_update_to_the_same_client() {
    let Some(font) = font_fixture() else {
        return;
    };
    incular_text::register_application_font_as("RobotoMono", font.clone());
    incular_text::register_application_font_as("Raleway", font);
    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let field = |style| {
        Widget::from(
            EditableText::new(controller.clone())
                .autofocus(true)
                .focus_node(focus.clone())
                .text_direction(TextDirection::Ltr)
                .style(style),
        )
    };
    let mut runtime = Runtime::new(field(
        TextStyle::new()
            .font_size(20.)
            .font_family("RobotoMono")
            .font_weight(FontWeight(600)),
    ))
    .unwrap();
    let size = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(size).unwrap();
    let (client, config) = runtime.flutter_text_input_configuration().unwrap();
    let mut host = FlutterTextInputControls::new(false);
    assert!(host.attach_existing(client, config, |_| {}));
    assert!(runtime.show_flutter_text_input(client, &mut host).unwrap());
    host.take_platform_calls();

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(
            root,
            field(
                TextStyle::new()
                    .font_size(20.)
                    .font_family("Raleway")
                    .font_weight(FontWeight(700))
                    .letter_spacing(1.)
                    .word_spacing(Some(2.)),
            ),
        )
        .unwrap();
    runtime.run_frame(size).unwrap();
    assert!(focus.has_primary_focus());
    assert_eq!(
        runtime.flutter_text_input_configuration().unwrap().0,
        client
    );
    assert!(runtime.publish_flutter_text_input_style(&mut host).unwrap());
    let calls = host.take_platform_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].method, "TextInput.setStyle");
    assert_eq!(
        calls[0].arguments,
        serde_json::json!({
            "fontSize": 20.0, "fontFamily": "Raleway", "fontWeightIndex": 6,
            "textAlignIndex": 4, "textDirectionIndex": 1,
            "letterSpacing": 1.0, "wordSpacing": 2.0, "lineHeight": 20.0
        })
    );
    assert!(!runtime.publish_flutter_text_input_style(&mut host).unwrap());
    assert!(host.take_platform_calls().is_empty());
}

#[test]
fn replacing_the_inactive_editors_controller_and_style_relayouts_without_stealing_focus() {
    let Some(font) = font_fixture() else {
        return;
    };
    const FONT: &str = "Pinned FlutterTest for dormant editor";
    incular_text::register_application_font_as(FONT, font);
    let [first, second] = [FocusNode::new(), FocusNode::new()];
    let controllers = std::array::from_fn::<_, 4, _>(|_| TextEditingController::new());
    let field = |key, index: usize, node: &FocusNode, size| {
        Widget::from(
            EditableText::new(controllers[index].clone())
                .focus_node(node.clone())
                .text_direction(TextDirection::Ltr)
                .style(TextStyle::new().font_family(FONT).font_size(size)),
        )
        .with_key(key)
    };
    let build = |update: bool| {
        Widget::from(
            Column::new([
                field(
                    "old editor",
                    if update { 2 } else { 0 },
                    &first,
                    if update { 20. } else { 9. },
                ),
                field("current editor", if update { 3 } else { 1 }, &second, 9.),
            ])
            .text_direction(TextDirection::Ltr),
        )
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    let base = Instant::now();
    let viewport = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame_at(viewport, base).unwrap();
    let lookup = |rt: &Runtime, key| rt.tree().element_with_key(&Key::from(key)).unwrap();
    let old = lookup(&runtime, "old editor");
    let current = lookup(&runtime, "current editor");
    let height = |rt: &Runtime, element| rt.tree().element_bounds(element).unwrap().size.height;
    assert_eq!(height(&runtime, old), 9.);
    let tap = |rt: &mut Runtime, y, at| {
        for (index, phase) in [PointerPhase::Down, PointerPhase::Up]
            .into_iter()
            .enumerate()
        {
            let _ = rt.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: Offset::new(20., y),
                },
                base + Duration::from_millis(at + index as u64),
            );
        }
        rt.run_frame_at(viewport, base + Duration::from_millis(at + 2))
            .unwrap();
    };
    tap(&mut runtime, 4., 10);
    assert!(first.has_primary_focus());
    controllers[0].set_text("test");
    runtime
        .run_frame_at(viewport, base + Duration::from_millis(20))
        .unwrap();
    tap(&mut runtime, 13., 30);
    assert!(!first.has_focus() && second.has_primary_focus());

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(true)).unwrap();
    runtime
        .run_frame_at(viewport, base + Duration::from_millis(45))
        .unwrap();
    assert_eq!(lookup(&runtime, "old editor"), old);
    assert_eq!(lookup(&runtime, "current editor"), current);
    assert!(second.has_primary_focus() && !first.has_focus());
    assert_eq!(height(&runtime, old), 20.);
    assert_eq!(height(&runtime, current), 9.);
}
