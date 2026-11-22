//! Material's text selection handles (`material/text_selection.dart`
//! `MaterialTextSelectionHandleControls`): a touch long press shows a start
//! and an end teardrop in the theme's primary colour, each placed so its tip
//! sits on the bottom of the endpoint's caret line (`getHandleAnchor`).

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_material::{TextField, ThemeData};
use incular_rendering::{Brush, PaintCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    Directionality, SelectionHandleType, TextDirection, Widget, internal::TextEditingController,
};
use std::time::{Duration, Instant};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(400., 120.))
}

fn mount(text: &str) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let field = Widget::from(TextField::new(controller.clone()));
    let root = Widget::from(Directionality::new(TextDirection::Ltr, field));
    let mut runtime = Runtime::new(root).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

fn pointer(runtime: &mut Runtime, phase: PointerPhase, position: Offset, at: Instant) {
    let (buttons, button) = match phase {
        PointerPhase::Up => (0, Some(PRIMARY_POINTER_BUTTON)),
        PointerPhase::Down => (PRIMARY_POINTER_BUTTON, Some(PRIMARY_POINTER_BUTTON)),
        _ => (PRIMARY_POINTER_BUTTON, None),
    };
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 3,
            kind: PointerDeviceKind::Touch,
            buttons,
            button,
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        },
        at,
    );
}

/// Long press on the first word, which selects "hello" and shows handles.
fn select_first_word(runtime: &mut Runtime) {
    let start = Instant::now();
    let held = start + Duration::from_millis(600);
    pointer(runtime, PointerPhase::Down, Offset::new(40., 28.), start);
    runtime.run_frame_at(bounds(), held).unwrap();
    pointer(runtime, PointerPhase::Up, Offset::new(40., 28.), held);
    runtime.run_frame_at(bounds(), held).unwrap();
}

/// The number of solid fills in the theme's primary colour that paint now.
fn primary_fills(runtime: &mut Runtime) -> usize {
    let primary: Color = ThemeData::light().core().color_scheme.primary;
    runtime
        .tree_mut()
        .paint()
        .commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                PaintCommand::FillPath {
                    brush: Brush::Solid(color),
                    ..
                } if *color == primary
            )
        })
        .count()
}

#[test]
fn touch_selection_paints_a_start_and_an_end_handle_in_the_primary_colour() {
    let (mut runtime, _controller) = mount("hello world");
    assert_eq!(
        primary_fills(&mut runtime),
        0,
        "no handles before a selection"
    );

    select_first_word(&mut runtime);
    assert_eq!(
        primary_fills(&mut runtime),
        2,
        "a range paints two teardrops"
    );
}

#[test]
fn start_handle_box_hangs_left_of_its_anchor_as_material_places_it() {
    let (mut runtime, _controller) = mount("hello world");
    select_first_word(&mut runtime);
    let field = runtime.focused_element().expect("focused field");
    let handles = runtime
        .tree()
        .text_field_selection_handle_geometry(field)
        .expect("handle geometry");
    let start = handles.start;
    assert_eq!(start.handle_type, SelectionHandleType::Left);
    // `getHandleAnchor(left)` is (size, 0): the box's top-right corner sits on
    // the anchor, so the box lies below and left of the selection start.
    let rect = start.rect();
    assert_eq!(rect.size, Size::new(22., 22.));
    assert_eq!(
        rect.origin,
        Offset::new(start.anchor.x - 22., start.anchor.y)
    );
    let end = handles.end.expect("end handle");
    assert_eq!(end.handle_type, SelectionHandleType::Right);
    // `getHandleAnchor(right)` is (0, 0): the top-left corner is the anchor.
    assert_eq!(end.rect().origin, end.anchor);
}

#[test]
fn typing_removes_the_painted_handles() {
    let (mut runtime, _controller) = mount("hello world");
    select_first_word(&mut runtime);
    assert_eq!(primary_fills(&mut runtime), 2);
    let _ = runtime.handle_input(InputEvent::Text("x".into()));
    runtime.run_frame(bounds()).unwrap();
    assert_eq!(primary_fills(&mut runtime), 0);
}
