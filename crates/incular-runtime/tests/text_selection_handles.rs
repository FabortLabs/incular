//! Flutter's touch selection handles on text fields. `EditableTextState`
//! shows them after a touch or stylus long press selects, after a touch
//! double tap selects a word, and after a touch tap that places the caret in
//! a focused field on Android (`_shouldShowSelectionHandles`). A mouse press,
//! a typed edit or losing focus closes them.
//!
//! Touch drags follow `TextSelectionGestureDetectorBuilder`: iOS never selects
//! on a touch drag, and Android moves a collapsed caret only in a field
//! that already had focus.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    EditableText, IgnorePointer, SizedBox, TextDirection, TextSelectionHandle,
    TextSelectionHandles, Widget,
    internal::{TextEditingController, TextSelection},
};
use std::time::{Duration, Instant};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(400., 120.))
}

fn mount(text: &str, platform: TextShortcutPlatform) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., 32.))
        .selection_handles_builder(|_, _| SizedBox::shrink().into());
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

fn pointer(
    runtime: &mut Runtime,
    phase: PointerPhase,
    position: Offset,
    kind: PointerDeviceKind,
    at: Instant,
) {
    let (buttons, button) = match phase {
        PointerPhase::Up => (0, Some(PRIMARY_POINTER_BUTTON)),
        PointerPhase::Down => (PRIMARY_POINTER_BUTTON, Some(PRIMARY_POINTER_BUTTON)),
        _ => (PRIMARY_POINTER_BUTTON, None),
    };
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 3,
            kind,
            buttons,
            button,
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        },
        at,
    );
}

/// A quick press and release at `position`, starting at `at`.
fn tap(runtime: &mut Runtime, position: Offset, kind: PointerDeviceKind, at: Instant) -> Instant {
    pointer(runtime, PointerPhase::Down, position, kind, at);
    let up = at + Duration::from_millis(50);
    pointer(runtime, PointerPhase::Up, position, kind, up);
    runtime.run_frame_at(bounds(), up).unwrap();
    up
}

/// A press held past the long press deadline, then released.
fn long_press(runtime: &mut Runtime, position: Offset, kind: PointerDeviceKind) -> Instant {
    let start = Instant::now();
    pointer(runtime, PointerPhase::Down, position, kind, start);
    let held = start + Duration::from_millis(600);
    runtime.run_frame_at(bounds(), held).unwrap();
    pointer(runtime, PointerPhase::Up, position, kind, held);
    runtime.run_frame_at(bounds(), held).unwrap();
    held
}

/// Down, move and up with a touch drag from `from` to `to`.
fn drag(runtime: &mut Runtime, from: Offset, to: Offset, kind: PointerDeviceKind, at: Instant) {
    pointer(runtime, PointerPhase::Down, from, kind, at);
    pointer(
        runtime,
        PointerPhase::Move,
        to,
        kind,
        at + Duration::from_millis(100),
    );
    pointer(
        runtime,
        PointerPhase::Up,
        to,
        kind,
        at + Duration::from_millis(120),
    );
    runtime.run_frame(bounds()).unwrap();
}

/// The handles showing on the focused field, if any.
fn shown_handles(runtime: &Runtime) -> Option<TextSelectionHandles> {
    let field = runtime.focused_element()?;
    runtime
        .tree()
        .text_field_selection_handles(field)?
        .handles()
}

#[test]
fn touch_long_press_shows_start_and_end_handles() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Windows);
    long_press(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
    );
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
    let handles = shown_handles(&runtime).expect("range handles");
    assert!(handles.end.is_some(), "a range shows an end handle");
}

#[test]
fn touch_tap_on_focused_field_shows_a_collapsed_handle_on_android() {
    let (mut runtime, controller) = mount("hello", TextShortcutPlatform::Android);
    let start = Instant::now();
    let first = tap(
        &mut runtime,
        Offset::new(10., 10.),
        PointerDeviceKind::Touch,
        start,
    );
    assert!(
        shown_handles(&runtime).is_none(),
        "the focusing tap shows none"
    );
    // A second tap well after the double tap window places the caret.
    tap(
        &mut runtime,
        Offset::new(10., 10.),
        PointerDeviceKind::Touch,
        first + Duration::from_secs(1),
    );
    let selection = controller.selection();
    assert_eq!(
        selection.base, selection.extent,
        "the tap collapses the selection"
    );
    let handles = shown_handles(&runtime).expect("collapsed handle");
    assert!(
        handles.end.is_none(),
        "a collapsed selection shows one handle"
    );
}

#[test]
fn mouse_click_shows_no_handles() {
    let (mut runtime, _controller) = mount("hello", TextShortcutPlatform::Windows);
    let start = Instant::now();
    let first = tap(
        &mut runtime,
        Offset::new(10., 10.),
        PointerDeviceKind::Mouse,
        start,
    );
    tap(
        &mut runtime,
        Offset::new(20., 10.),
        PointerDeviceKind::Mouse,
        first + Duration::from_secs(1),
    );
    assert!(shown_handles(&runtime).is_none());
}

#[test]
fn typing_hides_the_handles() {
    let (mut runtime, _controller) = mount("hello world", TextShortcutPlatform::Android);
    long_press(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
    );
    assert!(shown_handles(&runtime).is_some());
    let _ = runtime.handle_input(InputEvent::Text("x".into()));
    runtime.run_frame(bounds()).unwrap();
    assert!(shown_handles(&runtime).is_none());
}

#[test]
fn ios_touch_drag_does_not_change_the_selection() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Ios);
    drag(
        &mut runtime,
        Offset::new(12., 10.),
        Offset::new(150., 10.),
        PointerDeviceKind::Touch,
        Instant::now(),
    );
    let selection = controller.selection();
    assert_eq!(
        selection.base, selection.extent,
        "iOS touch drag selects nothing"
    );
}

#[test]
fn android_touch_drag_moves_a_collapsed_caret_only_in_a_focused_field() {
    let (mut runtime, controller) = mount("I love flutter!", TextShortcutPlatform::Android);
    let field = runtime.tree().text_field_at(Offset::new(12., 10.)).unwrap();
    let mut point = |offset| {
        controller.set_selection(TextSelection::collapsed(offset));
        runtime.run_frame(bounds()).unwrap();
        let caret = runtime
            .tree()
            .text_field_input_geometry(field)
            .unwrap()
            .caret_rect;
        Offset::new(caret.origin.x, caret.origin.y + caret.size.height / 2.0)
    };
    let from = point(4);
    let to = point(10);
    controller.set_text("I love flutter!");
    runtime.run_frame(bounds()).unwrap();
    assert!(controller.selection_if_set().is_none());

    let start = Instant::now();
    drag(&mut runtime, from, to, PointerDeviceKind::Touch, start);
    assert!(runtime.focused_element().is_none());
    assert!(controller.selection_if_set().is_none());

    tap(
        &mut runtime,
        from,
        PointerDeviceKind::Touch,
        start + Duration::from_secs(1),
    );
    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(controller.selection(), TextSelection::collapsed(4));
    drag(
        &mut runtime,
        from,
        to,
        PointerDeviceKind::Touch,
        start + Duration::from_secs(2),
    );
    assert_eq!(controller.selection(), TextSelection::collapsed(10));
}

/// Like [`mount`], with a touch toolbar installed so a handle tap can open it.
fn mount_with_toolbar(
    text: &str,
    platform: TextShortcutPlatform,
) -> (Runtime, TextEditingController) {
    let controller = TextEditingController::with_text(text);
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., 32.))
        .selection_handles_builder(|_, _| SizedBox::shrink().into())
        .context_menu_builder(|_, _| IgnorePointer::new(SizedBox::shrink()).into());
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

/// Whether the focused field's toolbar is open.
fn toolbar_open(runtime: &Runtime) -> bool {
    let field = runtime.focused_element().expect("focused field");
    runtime
        .tree()
        .text_field_context_menu(field)
        .is_some_and(|menu| menu.is_showing())
}

/// The centre of a handle's knob, where a finger grabs it.
fn knob_centre(handle: TextSelectionHandle) -> Offset {
    let rect = handle.rect();
    Offset::new(
        rect.origin.x + rect.size.width / 2.0,
        rect.origin.y + rect.size.height / 2.0,
    )
}

/// Focuses an Android field and places its caret with a second, later tap.
fn place_caret(runtime: &mut Runtime, at: Offset) {
    let start = Instant::now();
    let first = tap(runtime, at, PointerDeviceKind::Touch, start);
    tap(
        runtime,
        at,
        PointerDeviceKind::Touch,
        first + Duration::from_secs(1),
    );
}

#[test]
fn dragging_the_end_handle_right_extends_the_selection() {
    let (mut runtime, controller) =
        mount_with_toolbar("hello world", TextShortcutPlatform::Windows);
    long_press(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
    );
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
    assert!(toolbar_open(&runtime), "the long press shows the toolbar");
    let end = shown_handles(&runtime)
        .and_then(|h| h.end)
        .expect("end handle");
    let from = knob_centre(end);
    drag(
        &mut runtime,
        from,
        Offset::new(from.x + 60., from.y),
        PointerDeviceKind::Touch,
        Instant::now(),
    );
    let selection = controller.selection();
    assert_eq!(selection.base, 0, "the start stays put");
    assert!(selection.extent > 5, "the end handle moved right");
    assert!(
        toolbar_open(&runtime),
        "the toolbar shows again after the drag"
    );
}

#[test]
fn dragging_the_start_handle_left_extends_the_selection() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Windows);
    long_press(
        &mut runtime,
        Offset::new(50., 10.),
        PointerDeviceKind::Touch,
    );
    let before = controller.selection();
    assert!(before.base > 0, "a word past the first is selected");
    let start = shown_handles(&runtime).expect("range handles").start;
    let from = knob_centre(start);
    drag(
        &mut runtime,
        from,
        Offset::new(from.x - 40., from.y),
        PointerDeviceKind::Touch,
        Instant::now(),
    );
    let selection = controller.selection();
    assert!(selection.base < before.base, "the start handle moved left");
    assert_eq!(selection.extent, before.extent, "the end stays put");
}

#[test]
fn dragging_the_end_handle_before_the_start_does_not_invert_the_selection() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Windows);
    long_press(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Touch,
    );
    let end = shown_handles(&runtime)
        .and_then(|h| h.end)
        .expect("end handle");
    let from = knob_centre(end);
    drag(
        &mut runtime,
        from,
        Offset::new(0., from.y),
        PointerDeviceKind::Touch,
        Instant::now(),
    );
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
}

#[test]
fn dragging_the_collapsed_handle_moves_the_caret() {
    let (mut runtime, controller) = mount("hello", TextShortcutPlatform::Android);
    place_caret(&mut runtime, Offset::new(10., 10.));
    let before = controller.selection();
    assert_eq!(before.base, before.extent, "the caret is collapsed");
    let caret = shown_handles(&runtime).expect("collapsed handle").start;
    let from = knob_centre(caret);
    drag(
        &mut runtime,
        from,
        Offset::new(from.x + 40., from.y),
        PointerDeviceKind::Touch,
        Instant::now(),
    );
    let after = controller.selection();
    assert_eq!(after.base, after.extent, "the caret stays collapsed");
    assert!(after.extent > before.extent, "the caret moved right");
}

#[test]
fn tapping_the_collapsed_handle_toggles_the_toolbar() {
    let (mut runtime, _controller) = mount_with_toolbar("hello", TextShortcutPlatform::Android);
    place_caret(&mut runtime, Offset::new(10., 10.));
    let knob = knob_centre(shown_handles(&runtime).expect("collapsed handle").start);
    assert!(!toolbar_open(&runtime));
    let first = tap(&mut runtime, knob, PointerDeviceKind::Touch, Instant::now());
    assert!(toolbar_open(&runtime), "a handle tap shows the toolbar");
    assert!(shown_handles(&runtime).is_some(), "the handle stays");
    tap(
        &mut runtime,
        knob,
        PointerDeviceKind::Touch,
        first + Duration::from_secs(1),
    );
    assert!(!toolbar_open(&runtime), "a second handle tap hides it");
    assert!(shown_handles(&runtime).is_none(), "and the handles");
}

#[test]
fn pressing_a_handle_neither_refocuses_nor_starts_a_long_press() {
    let (mut runtime, controller) = mount("hello world", TextShortcutPlatform::Windows);
    long_press(
        &mut runtime,
        Offset::new(50., 10.),
        PointerDeviceKind::Touch,
    );
    let before = controller.selection();
    let focused = runtime.focused_element();
    let knob = knob_centre(shown_handles(&runtime).expect("range handles").start);
    let at = Instant::now();
    pointer(
        &mut runtime,
        PointerPhase::Down,
        knob,
        PointerDeviceKind::Touch,
        at,
    );
    let held = at + Duration::from_millis(600);
    runtime.run_frame_at(bounds(), held).unwrap();
    pointer(
        &mut runtime,
        PointerPhase::Up,
        knob,
        PointerDeviceKind::Touch,
        held,
    );
    runtime.run_frame_at(bounds(), held).unwrap();
    assert_eq!(
        controller.selection(),
        before,
        "no long press word selection"
    );
    assert_eq!(runtime.focused_element(), focused, "focus does not move");
}

fn directional_handles(
    text: &str,
    direction: TextDirection,
    platform: TextShortcutPlatform,
    selected_chars: std::ops::Range<usize>,
) -> (Runtime, TextEditingController) {
    use incular_text::{TextStyle, register_application_font_as};

    let font = std::env::var_os("INCULAR_FLUTTER_RTL_FONT_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".into());
    let mut style = TextStyle::new().font_size(32.);
    if let Ok(bytes) = std::fs::read(font) {
        const FAMILY: &str = "Incular RTL handles fixture";
        register_application_font_as(FAMILY, bytes);
        style = style.font_family(FAMILY);
    }
    let controller = TextEditingController::with_text(text);
    let node = incular_widgets::FocusNode::new();
    let field = EditableText::new(controller.clone())
        .text_direction(direction)
        .focus_node(node.clone())
        .style(style)
        .size(Size::new(400., 42.))
        .select_all_on_focus(false)
        .selection_handles_builder(|_, _| SizedBox::shrink().into());
    let mut runtime = Runtime::new(field.into()).unwrap();
    runtime.set_text_shortcut_platform(platform);
    node.request_focus();
    runtime.run_frame(bounds()).unwrap();
    let byte = |character| {
        text.char_indices()
            .nth(character)
            .map_or(text.len(), |(byte, _)| byte)
    };
    controller.set_selection(TextSelection::new(
        byte(selected_chars.start),
        byte(selected_chars.end),
    ));
    runtime.run_frame(bounds()).unwrap();
    let field = runtime
        .focused_element()
        .expect("actual retained editor focus");
    let tree = runtime.tree();
    tree.text_field_selection_handles(field)
        .unwrap()
        .show(tree.text_field_selection_handle_geometry(field).unwrap());
    // The runtime refreshes the visible overlay with the simulated
    // platform's actual handle policy.
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller)
}

#[test]
fn android_selection_handle_artwork_follows_the_shaped_endpoint_directions() {
    use TextDirection::{Ltr, Rtl};
    use incular_widgets::SelectionHandleType::{Left, Right};

    let cases = [
        (Ltr, "Hello World", 5, (Ltr, Ltr), (Left, Right)),
        (Rtl, "Hello World", 5, (Ltr, Ltr), (Left, Right)),
        (Rtl, "مرحبا بالعالم", 5, (Rtl, Rtl), (Right, Left)),
        (Ltr, "مرحبا بالعالم", 5, (Rtl, Rtl), (Right, Left)),
        (Ltr, "abc مرحبا", 9, (Ltr, Rtl), (Left, Left)),
        (Rtl, "abc مرحبا", 9, (Rtl, Ltr), (Right, Right)),
    ];
    for (field, text, end, directions, art) in cases {
        let (runtime, _) = directional_handles(text, field, TextShortcutPlatform::Android, 0..end);
        let handles = shown_handles(&runtime).expect("show the mounted selection");
        let end = handles.end.unwrap();
        assert_eq!(
            (handles.start.direction, end.direction),
            directions,
            "{field:?}/{text}"
        );
        assert_eq!(
            (handles.start.handle_type, end.handle_type),
            art,
            "{field:?}/{text}"
        );
    }
}

#[test]
fn ios_uses_the_field_direction_for_artwork_without_changing_range_endpoints() {
    use incular_widgets::SelectionHandleType::{Left, Right};
    let (android, _) = directional_handles(
        "abc مرحبا",
        TextDirection::Ltr,
        TextShortcutPlatform::Android,
        0..9,
    );
    let (ios, _) = directional_handles(
        "abc مرحبا",
        TextDirection::Ltr,
        TextShortcutPlatform::Ios,
        0..9,
    );
    let reference = shown_handles(&android).unwrap();
    let handles = shown_handles(&ios).unwrap();
    let (start, end) = (handles.start, handles.end.unwrap());
    assert_eq!(
        (start.direction, end.direction),
        (TextDirection::Ltr, TextDirection::Rtl)
    );
    assert_eq!((start.handle_type, end.handle_type), (Left, Right));
    assert_eq!(start.anchor, reference.start.anchor);
    assert_eq!(end.anchor, reference.end.unwrap().anchor);
}

#[test]
fn dragging_rtl_start_artwork_moves_the_start_and_keeps_the_other_endpoint() {
    use incular_text::TextAffinity;
    use incular_widgets::SelectionHandleType::{Left, Right};

    let (mut runtime, controller) = directional_handles(
        "אבגדהוזחט",
        TextDirection::Ltr,
        TextShortcutPlatform::Android,
        3..7,
    );
    let handles = shown_handles(&runtime).unwrap();
    assert_eq!(
        (handles.start.handle_type, handles.end.unwrap().handle_type),
        (Right, Left)
    );
    let field = runtime.focused_element().unwrap();
    let first = runtime
        .tree()
        .text_field_offset_rect(field, 0, TextAffinity::Downstream)
        .unwrap();
    let before = controller.selection();
    let from = knob_centre(handles.start);
    let target = Offset::new(first.origin.x, from.y);
    assert!((target.x - from.x).abs() > 18.);
    drag(
        &mut runtime,
        from,
        target,
        PointerDeviceKind::Touch,
        Instant::now(),
    );
    assert_eq!(controller.selection(), TextSelection::new(0, before.extent));
}

fn drag_through_unmeasurable_editor_geometry(transform_collapses: bool) {
    use incular_core::Transform as Affine;
    use incular_text::{TextAffinity, TextScaler, TextStyle, register_application_font_as};
    use incular_widgets::{Alignment, Center, FocusNode, Transform};

    let text = "The quick fox jumps over the fence.";
    let controller = TextEditingController::with_text(text);
    let focus = FocusNode::new();
    let mut style = TextStyle::new().font_size(14.).height(Some(1.));
    if let Some(path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") {
        let family = "Drag degeneracy pinned Ahem";
        register_application_font_as(family, std::fs::read(path).unwrap());
        style = style.font_family(family);
    }
    let scene = |scale_y, font_scale| {
        let field = EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .text_direction(TextDirection::Ltr)
            .style(style.clone())
            .text_scaler(TextScaler::linear(font_scale))
            .max_lines(None)
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        let child = SizedBox::new().width(300.).height(200.).child(field);
        Center::new(
            Transform::new(
                Affine::from_coefficients([1., 0., 0., scale_y, 0., 0.]),
                child,
            )
            .alignment(Alignment::CENTER),
        )
        .into()
    };
    let mut runtime = Runtime::new(scene(1., 1.)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let bounds = Constraints::tight(Size::new(800., 600.));
    let start = Instant::now();
    focus.request_focus();
    runtime.run_frame_at(bounds, start).unwrap();
    let field = runtime.focused_element().unwrap();
    let root = runtime.tree().root().unwrap();
    let initial = TextSelection::new(14, 19);
    controller.set_selection(initial);
    runtime.run_frame_at(bounds, start).unwrap();
    let tree = runtime.tree();
    tree.text_field_selection_handles(field)
        .unwrap()
        .show(tree.text_field_selection_handle_geometry(field).unwrap());
    runtime.run_frame_at(bounds, start).unwrap();
    let end_handle = shown_handles(&runtime).unwrap().end.unwrap();
    let handle = knob_centre(end_handle);

    let send = |runtime: &mut Runtime, phase, offset, ms| {
        pointer(
            runtime,
            phase,
            handle + Offset::new(offset, 0.),
            PointerDeviceKind::Touch,
            start + Duration::from_millis(ms),
        );
    };
    send(&mut runtime, PointerPhase::Down, 0., 10);
    send(&mut runtime, PointerPhase::Move, 20., 30);
    let measured = controller.selection();
    assert_eq!(measured.base, initial.base);
    assert!(
        measured.extent > initial.extent,
        "valid drag actually extends the handle"
    );

    let degenerate = if transform_collapses {
        scene(1e-310, 1.)
    } else {
        scene(1., 0.)
    };
    runtime.schedule_update(root, degenerate).unwrap();
    runtime
        .run_frame_at(bounds, start + Duration::from_millis(40))
        .unwrap();
    if !transform_collapses {
        let collapsed = runtime
            .tree()
            .editable_selection_endpoints(
                field,
                TextSelection::collapsed(0),
                TextAffinity::Downstream,
            )
            .unwrap();
        assert_eq!(
            collapsed[0].point.y, 0.,
            "zero scaled line height reaches actual layout"
        );
    }
    assert_eq!(
        runtime
            .tree()
            .text_field_position_at(field, handle + Offset::new(50., 0.)),
        None
    );
    send(&mut runtime, PointerPhase::Move, 50., 55);
    send(&mut runtime, PointerPhase::Move, 70., 65);
    assert_eq!(controller.selection(), measured);
    assert!(focus.has_primary_focus());
    assert_eq!(runtime.focused_element(), Some(field));

    runtime.schedule_update(root, scene(1., 1.)).unwrap();
    runtime
        .run_frame_at(bounds, start + Duration::from_millis(80))
        .unwrap();
    send(&mut runtime, PointerPhase::Move, -30., 100);
    send(&mut runtime, PointerPhase::Up, -30., 115);
    runtime
        .run_frame_at(bounds, start + Duration::from_millis(120))
        .unwrap();
    let recovered = controller.selection();
    assert_eq!(recovered.base, initial.base);
    assert!(recovered.extent > initial.base && recovered.extent < measured.extent);
    assert_eq!(controller.text(), text);
    assert!(focus.has_primary_focus());
}

#[test]
fn subnormal_editor_transform_does_not_corrupt_an_in_flight_handle_drag() {
    drag_through_unmeasurable_editor_geometry(true);
}

#[test]
fn zero_line_height_does_not_corrupt_an_in_flight_handle_drag() {
    drag_through_unmeasurable_editor_geometry(false);
}

#[test]
fn a_live_two_x_y_transform_doubles_the_global_line_jump_distance() {
    use incular_core::Transform as Affine;
    use incular_text::{TextAffinity, TextStyle};
    use incular_widgets::{Center, FocusNode, Transform};

    let text = "0123456789\n0123456789\n0123456789";
    let controller = TextEditingController::with_text(text);
    let focus = FocusNode::new();
    let scene = |scale_y, pivot| {
        let editor = EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::new().font_size(14.).height(Some(1.)))
            .max_lines(None)
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        let field = SizedBox::new().width(300.).height(200.).child(editor);
        let affine = Affine::from_coefficients([1., 0., 0., scale_y, 0., (1. - scale_y) * pivot]);
        Center::new(Transform::new(affine, field)).into()
    };
    let mut runtime = Runtime::new(scene(1., 0.)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let constraints = Constraints::tight(Size::new(800., 600.));
    let start = Instant::now();
    focus.request_focus();
    runtime.run_frame_at(constraints, start).unwrap();
    controller.set_selection(TextSelection::new(2, 7));
    runtime.run_frame_at(constraints, start).unwrap();
    let field = runtime.focused_element().unwrap();
    let root = runtime.tree().root().unwrap();
    let tree = runtime.tree();
    let points = tree
        .editable_selection_endpoints(field, TextSelection::collapsed(0), TextAffinity::Downstream)
        .unwrap();
    let row = points[0].point.y;
    assert!(row > 0. && row.is_finite());
    tree.text_field_selection_handles(field)
        .unwrap()
        .show(tree.text_field_selection_handle_geometry(field).unwrap());
    runtime.run_frame_at(constraints, start).unwrap();
    let end_handle = shown_handles(&runtime).unwrap().end.unwrap();
    let handle = knob_centre(end_handle);
    let origin_y = runtime.tree().element_bounds(field).unwrap().origin.y;
    let pivot = f64::from(end_handle.anchor.y - origin_y);
    let send = |runtime: &mut Runtime, phase, x, y, ms| {
        pointer(
            runtime,
            phase,
            handle + Offset::new(x, y),
            PointerDeviceKind::Touch,
            start + Duration::from_millis(ms),
        );
    };
    send(&mut runtime, PointerPhase::Down, 0., 0., 10);
    send(&mut runtime, PointerPhase::Move, 20., 0., 30);
    runtime.schedule_update(root, scene(2., pivot)).unwrap();
    runtime
        .run_frame_at(constraints, start + Duration::from_millis(40))
        .unwrap();

    // The first move would cross one old unscaled line; it must not yet
    // move onto the second line in the doubled editor.
    send(&mut runtime, PointerPhase::Move, 0., 1.4 * row, 55);
    let first = controller.selection();
    assert_eq!(first.base, 2);
    assert!((3..=10).contains(&first.extent), "first row: {first:?}");
    send(&mut runtime, PointerPhase::Move, 0., 2.1 * row, 65);
    let second = controller.selection();
    assert_eq!(second.base, 2);
    assert_eq!(
        second.extent,
        first.extent + 11,
        "same column one row later"
    );
    send(&mut runtime, PointerPhase::Up, 0., 2.1 * row, 80);
    assert!(focus.has_primary_focus());
    assert_eq!(runtime.focused_element(), Some(field));
}

#[test]
fn visible_touch_handles_follow_the_retained_editors_larger_font() {
    use incular_config::Alignment;
    use incular_text::{TextStyle, register_application_font_as};
    use incular_widgets::Align;

    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FAMILY: &str = "Pinned editable handle relayout FlutterTest";
    register_application_font_as(FAMILY, std::fs::read(font).unwrap());
    let controller = TextEditingController::with_text("initial value");
    let widget = |font_size| {
        Align::new(
            Alignment::TOP_LEFT,
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .style(TextStyle::new().font_family(FAMILY).font_size(font_size))
                .selection_handles_builder(|_, _| SizedBox::shrink().into()),
        )
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(widget(18.)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let now = Instant::now();
    runtime.run_frame_at(bounds, now).unwrap();
    let point = Offset::new(1., 1.);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        point,
        PointerDeviceKind::Touch,
        now,
    );
    let held = now + Duration::from_millis(600);
    runtime.run_frame_at(bounds, held).unwrap();
    pointer(
        &mut runtime,
        PointerPhase::Up,
        point,
        PointerDeviceKind::Touch,
        held,
    );
    runtime.run_frame_at(bounds, held).unwrap();
    let field = runtime
        .focused_element()
        .expect("long press focuses the editor");
    let render = runtime.tree().render_id(field);
    let selection = TextSelection::new(0, 7);
    assert_eq!(controller.selection(), selection);
    let before = shown_handles(&runtime).expect("original handles");
    let before_end = before.end.unwrap();
    assert_eq!([before.start.anchor.x, before_end.anchor.x], [0., 126.]);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, widget(30.)).unwrap();
    runtime
        .run_frame_at(bounds, held + Duration::from_millis(16))
        .unwrap();
    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(runtime.tree().render_id(field), render);
    assert_eq!(controller.selection(), selection);
    let after = shown_handles(&runtime).expect("handles survive editor relayout");
    let after_end = after.end.unwrap();
    assert_eq!([after.start.anchor.x, after_end.anchor.x], [0., 210.]);
    assert_eq!(after.start.anchor.y - before.start.anchor.y, 12.);
    assert_eq!(after_end.anchor.y - before_end.anchor.y, 12.);
}

#[test]
fn drag_device_controls_whether_an_editor_extends_a_range_or_moves_the_caret() {
    use PointerDeviceKind::{Mouse, Stylus, Touch, Unknown};
    use incular_config::TargetPlatform::{Android, Fuchsia, Linux, MacOs, Windows};
    use incular_widgets::TextSelectionGestureDetectorBuilder;

    for (os, kind, renderer_ignores_pointer, expected) in [
        (Android, Mouse, false, TextSelection::new(4, 10)),
        (Android, Touch, false, TextSelection::collapsed(10)),
        (Android, Touch, true, TextSelection::new(2, 14)),
        (Android, Unknown, false, TextSelection::collapsed(10)),
        (Android, Stylus, false, TextSelection::new(4, 10)),
        (Fuchsia, Stylus, false, TextSelection::new(4, 10)),
        (Linux, Stylus, false, TextSelection::new(4, 10)),
        (MacOs, Stylus, false, TextSelection::new(4, 10)),
        (Windows, Stylus, false, TextSelection::new(4, 10)),
    ] {
        let controller = TextEditingController::with_text("I love flutter!");
        let field = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .size(Size::new(400., 32.))
            .renderer_ignores_pointer(renderer_ignores_pointer)
            .selection_handles_builder(|_, _| SizedBox::shrink().into())
            .context_menu_builder(|_, _| IgnorePointer::new(SizedBox::shrink()).into());
        let field = TextSelectionGestureDetectorBuilder::new().build_gesture_detector(field);
        let mut runtime = Runtime::new(field.into()).unwrap();
        runtime.set_text_target_platform(os);
        runtime.run_frame(bounds()).unwrap();
        let id = runtime.tree().text_field_at(Offset::new(12., 10.)).unwrap();
        let [from, middle, to] = [4, 7, 10].map(|offset| {
            controller.set_selection(TextSelection::collapsed(offset));
            runtime.run_frame(bounds()).unwrap();
            let rect = runtime
                .tree()
                .text_field_input_geometry(id)
                .unwrap()
                .caret_rect;
            Offset::new(rect.origin.x, rect.origin.y + rect.size.height / 2.)
        });
        controller.clear_selection();
        runtime.run_frame(bounds()).unwrap();
        assert!(controller.selection_if_set().is_none(), "{os:?} {kind:?}");

        let start = Instant::now();
        let up = tap(&mut runtime, from, Touch, start);
        assert_eq!(
            controller.selection(),
            TextSelection::collapsed(4),
            "{os:?} {kind:?}"
        );
        let delay = if kind == Mouse { 301 } else { 1 };
        let down = up + Duration::from_millis(delay);
        for (phase, at, elapsed) in [
            (PointerPhase::Down, from, 0),
            (PointerPhase::Move, middle, 16),
            (PointerPhase::Move, to, 32),
            (PointerPhase::Up, to, 48),
        ] {
            let at_time = down + Duration::from_millis(elapsed);
            pointer(&mut runtime, phase, at, kind, at_time);
            runtime.run_frame_at(bounds(), at_time).unwrap();
        }
        assert_eq!(controller.selection(), expected, "{os:?} {kind:?}");
        if kind == Mouse {
            assert!(shown_handles(&runtime).is_none());
            assert!(!toolbar_open(&runtime));
        }
    }
}

#[test]
fn a_read_only_mouse_drag_leaves_mobile_handles_and_toolbar_hidden() {
    let controller = TextEditingController::with_text("I love Flutter!");
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., 32.))
        .read_only(true)
        .selection_handles_builder(|_, _| SizedBox::shrink().into())
        .context_menu_builder(|_, _| IgnorePointer::new(SizedBox::shrink()).into());
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime.run_frame(bounds()).unwrap();
    let field = runtime.tree().text_field_at(Offset::new(1., 1.)).unwrap();
    let handles = runtime.tree().text_field_selection_handles(field).unwrap();
    let menu = runtime.tree().text_field_context_menu(field).unwrap();
    drag(
        &mut runtime,
        Offset::new(1., 1.),
        Offset::new(51., 1.),
        PointerDeviceKind::Mouse,
        Instant::now(),
    );
    assert!(!controller.selection().is_collapsed());
    assert!(!handles.is_showing());
    assert!(!menu.is_showing());
}

#[test]
fn an_ios_selection_handle_does_not_steal_taps_outside_its_local_hit_box() {
    use incular_widgets::{Column, GestureDetector, HitTestBehavior, TextSelectionHandleLayout};
    use std::{cell::Cell, rc::Rc};

    let handle_taps = Rc::new(Cell::new(0));
    let sibling_taps = Rc::new(Cell::new(0));
    let editor = TextEditingController::with_text("a");
    let layout = TextSelectionHandleLayout::new(Size::new(20., 20.), Offset::new(10., 0.));
    let field = EditableText::new(editor.clone())
        .autofocus(true)
        .size(Size::new(200., 80.))
        .text_direction(TextDirection::Ltr)
        .selection_handles_builder(|_, _| SizedBox::square(20.).into())
        .collapsed_selection_handle_layout(layout)
        .on_selection_handle_tapped({
            let taps = handle_taps.clone();
            move || taps.set(taps.get() + 1)
        });
    let sibling = GestureDetector::new(SizedBox::new().width(200.).height(40.))
        .behavior(HitTestBehavior::Opaque)
        .on_tap({
            let taps = sibling_taps.clone();
            move || taps.set(taps.get() + 1)
        });
    let mut runtime =
        Runtime::new(Column::new([Widget::from(field), sibling.into()]).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    let constraints = Constraints::tight(Size::new(400., 200.));
    runtime.run_frame(constraints).unwrap();
    let focused = runtime.focused_element().expect("autofocused editor");
    let field_bounds = runtime.tree().element_bounds(focused).unwrap();
    let point = field_bounds.origin + Offset::new(6., 7.);
    let start = Instant::now();
    pointer(
        &mut runtime,
        PointerPhase::Down,
        point,
        PointerDeviceKind::Touch,
        start,
    );
    let held = start + Duration::from_millis(520);
    runtime.run_frame_at(constraints, held).unwrap();
    pointer(
        &mut runtime,
        PointerPhase::Up,
        point,
        PointerDeviceKind::Touch,
        held,
    );
    runtime.run_frame_at(constraints, held).unwrap();

    let state = runtime
        .tree()
        .text_field_selection_handles(focused)
        .unwrap();
    let handles = state.handles().expect("long press shows a handle");
    assert!(handles.end.is_none());
    let hit = layout.interactive_rect(handles.start.anchor);
    assert!(hit.size.width <= field_bounds.size.width);
    assert!(hit.size.height <= field_bounds.size.height);
    assert_eq!(hit.size, Size::new(48., 48.));
    let inside = knob_centre(handles.start);
    let outside = Offset::new(200., field_bounds.origin.y + field_bounds.size.height + 20.);
    assert!(state.hit_test(inside).is_some());
    assert!(state.hit_test(outside).is_none());
    for (point, elapsed) in [(inside, 700), (outside, 1050)] {
        for (phase, extra) in [(PointerPhase::Down, 0), (PointerPhase::Up, 40)] {
            let time = start + Duration::from_millis(elapsed + extra);
            pointer(&mut runtime, phase, point, PointerDeviceKind::Touch, time);
            runtime.run_frame_at(constraints, time).unwrap();
        }
    }
    assert_eq!(handle_taps.get(), 1);
    assert_eq!(sibling_taps.get(), 1);
}
