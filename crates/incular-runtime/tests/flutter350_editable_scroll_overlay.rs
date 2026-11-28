use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{
    EditableText, SizedBox, Widget,
    internal::{ElementId, Key},
};

fn pointer(runtime: &mut Runtime, phase: PointerPhase, position: Offset, at: Instant) {
    let terminal = matches!(phase, PointerPhase::Up | PointerPhase::Cancel);
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 2,
            kind: PointerDeviceKind::Touch,
            buttons: if terminal { 0 } else { PRIMARY_POINTER_BUTTON },
            button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON),
            phase,
            position,
            sample: PointerSampleMetadata::default(),
        },
        at,
    );
}

fn frame(runtime: &mut Runtime, at: Instant) {
    runtime
        .run_frame_at(Constraints::tight(Size::new(400., 200.)), at)
        .unwrap();
}

fn toolbar_visible(runtime: &Runtime, field: ElementId) -> bool {
    runtime
        .tree()
        .text_field_toolbar_state(field)
        .unwrap()
        .is_showing()
}

#[test]
fn a_short_touch_scroll_hides_then_restores_the_toolbar_without_semantics() {
    const TOOLBAR: &str = "editor scroll test toolbar";
    for platform in [TextShortcutPlatform::Android, TextShortcutPlatform::Ios] {
        let controller =
            TextEditingController::with_text("Lorem ipsum dolor sit amet ".repeat(200));
        let editor = EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .max_lines(None)
            .context_menu_builder(|_, _| Widget::from(SizedBox::shrink()).with_key(TOOLBAR))
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        let mut runtime = Runtime::new(editor.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime.set_platform_semantics_enabled(false);
        assert!(!runtime.semantics_enabled());

        let clock = Instant::now();
        let time = |milliseconds| clock + Duration::from_millis(milliseconds);
        let middle = Offset::new(200., 100.);
        frame(&mut runtime, time(0));
        pointer(&mut runtime, PointerPhase::Down, middle, time(10));
        frame(&mut runtime, time(510));
        pointer(&mut runtime, PointerPhase::Up, middle, time(530));
        frame(&mut runtime, time(530));
        let field = runtime.focused_element().unwrap();
        let selection = controller.selection();
        assert!(!selection.is_collapsed());
        assert!(toolbar_visible(&runtime, field));
        let previous_offset = runtime.tree().text_field_scroll_offset(field).unwrap();

        pointer(&mut runtime, PointerPhase::Down, middle, time(1000));
        let accepted = middle - Offset::new(0., 20.);
        pointer(&mut runtime, PointerPhase::Move, accepted, time(1020));
        frame(&mut runtime, time(1020));
        assert_eq!(
            runtime.tree().text_field_scroll_offset(field),
            Some(previous_offset)
        );
        assert!(!toolbar_visible(&runtime, field));
        let moved = middle - Offset::new(0., 30.);
        pointer(&mut runtime, PointerPhase::Move, moved, time(1030));
        frame(&mut runtime, time(1030));
        assert!(
            runtime.tree().text_field_scroll_offset(field).unwrap().y > previous_offset.y,
            "the editor's viewport must consume the short drag ({platform:?})"
        );
        assert_eq!(controller.selection(), selection);
        assert!(!toolbar_visible(&runtime, field));

        pointer(&mut runtime, PointerPhase::Up, moved, time(1040));
        frame(&mut runtime, time(1040));
        assert!(
            runtime.frame_requested(),
            "restoration must schedule its own paint"
        );
        frame(&mut runtime, time(1060));
        assert!(
            runtime
                .tree()
                .element_with_key(&Key::from(TOOLBAR))
                .is_some()
        );
        assert!(toolbar_visible(&runtime, field));
        assert_eq!(controller.selection(), selection);
    }
}

fn started_with_toolbar() -> (Runtime, TextEditingController, ElementId, Instant) {
    let controller = TextEditingController::with_text("text ".repeat(10_000));
    let editor = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .max_lines(None)
        .context_menu_builder(|_, _| SizedBox::shrink().into())
        .selection_handles_builder(|_, _| SizedBox::shrink().into());
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let clock = Instant::now();
    frame(&mut runtime, clock);
    let first_word = Offset::new(2., 8.);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        first_word,
        clock + Duration::from_millis(10),
    );
    frame(&mut runtime, clock + Duration::from_millis(510));
    pointer(
        &mut runtime,
        PointerPhase::Up,
        first_word,
        clock + Duration::from_millis(530),
    );
    frame(&mut runtime, clock + Duration::from_millis(530));
    let field = runtime.focused_element().unwrap();
    assert_eq!(
        controller.selection(),
        incular_text::TextSelection::new(0, 4)
    );
    assert!(toolbar_visible(&runtime, field));
    (runtime, controller, field, clock)
}

fn drag_away_from_selection(runtime: &mut Runtime, field: ElementId, clock: Instant) {
    let center = Offset::new(200., 100.);
    let moved = center - Offset::new(0., 200.);
    pointer(
        runtime,
        PointerPhase::Down,
        center,
        clock + Duration::from_millis(1000),
    );
    pointer(
        runtime,
        PointerPhase::Move,
        center - Offset::new(0., 20.),
        clock + Duration::from_millis(1010),
    );
    pointer(
        runtime,
        PointerPhase::Move,
        moved,
        clock + Duration::from_millis(1020),
    );
    frame(runtime, clock + Duration::from_millis(1020));
    assert!(runtime.tree().text_field_scroll_offset(field).unwrap().y > 100.);
    assert!(!toolbar_visible(runtime, field));
    pointer(
        runtime,
        PointerPhase::Up,
        moved,
        clock + Duration::from_millis(1040),
    );
}

#[test]
fn a_value_replacement_before_the_scroll_end_frame_never_reopens_the_old_toolbar() {
    let (mut runtime, controller, field, clock) = started_with_toolbar();
    drag_away_from_selection(&mut runtime, field, clock);
    controller.set_text("a different value");
    for elapsed in [1040, 1060, 1200] {
        frame(&mut runtime, clock + Duration::from_millis(elapsed));
        assert!(!toolbar_visible(&runtime, field));
    }
    assert_eq!(controller.text(), "a different value");
    assert_eq!(runtime.focused_element(), Some(field));
}

#[test]
fn replacing_the_document_between_two_scroll_contacts_does_not_reuse_old_geometry() {
    let (mut runtime, controller, field, clock) = started_with_toolbar();
    drag_away_from_selection(&mut runtime, field, clock);
    controller.set_text("a different value");
    let center = Offset::new(200., 100.);
    let moved = center - Offset::new(0., 100.);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        center,
        clock + Duration::from_millis(1050),
    );
    pointer(
        &mut runtime,
        PointerPhase::Move,
        moved,
        clock + Duration::from_millis(1070),
    );
    frame(&mut runtime, clock + Duration::from_millis(1070));
    pointer(
        &mut runtime,
        PointerPhase::Up,
        moved,
        clock + Duration::from_millis(1090),
    );
    frame(&mut runtime, clock + Duration::from_millis(1090));
    assert!(!toolbar_visible(&runtime, field));
    assert_eq!(controller.text(), "a different value");
    assert_eq!(runtime.focused_element(), Some(field));
}

#[test]
fn an_unchanged_toolbar_waits_until_its_selection_is_back_inside_the_editor() {
    let (mut runtime, controller, field, clock) = started_with_toolbar();
    drag_away_from_selection(&mut runtime, field, clock);
    frame(&mut runtime, clock + Duration::from_millis(1040));
    assert!(!toolbar_visible(&runtime, field));

    let center = Offset::new(200., 100.);
    let moved = center + Offset::new(0., 200.);
    pointer(
        &mut runtime,
        PointerPhase::Down,
        center,
        clock + Duration::from_millis(1100),
    );
    pointer(
        &mut runtime,
        PointerPhase::Move,
        center + Offset::new(0., 20.),
        clock + Duration::from_millis(1110),
    );
    pointer(
        &mut runtime,
        PointerPhase::Move,
        moved,
        clock + Duration::from_millis(1120),
    );
    frame(&mut runtime, clock + Duration::from_millis(1120));
    assert!(!toolbar_visible(&runtime, field));
    pointer(
        &mut runtime,
        PointerPhase::Up,
        moved,
        clock + Duration::from_millis(1140),
    );
    frame(&mut runtime, clock + Duration::from_millis(1140));
    frame(&mut runtime, clock + Duration::from_millis(1160));
    assert!(toolbar_visible(&runtime, field));
    assert_eq!(
        controller.selection(),
        incular_text::TextSelection::new(0, 4)
    );
}

#[test]
fn updating_drag_start_behavior_changes_live_handle_origin_without_remounting() {
    use incular_config::Alignment;
    use incular_core::DragStartBehavior;
    use incular_text::{TextSelection, TextStyle};
    use incular_widgets::{Align, FocusNode};

    let controller =
        TextEditingController::with_text("first line\nsecond line\nthird line\nfourth line");
    let focus = FocusNode::new();
    let build = |behavior| {
        Widget::from(Align::new(
            Alignment::TOP_LEFT,
            EditableText::new(controller.clone())
                .focus_node(focus.clone())
                .text_direction(TextDirection::Ltr)
                .max_lines(Some(10))
                .style(TextStyle::new().font_size(14.).height(Some(1.)))
                .selection_handles_builder(|_, _| SizedBox::shrink().into())
                .drag_start_behavior(behavior),
        ))
    };
    let mut runtime = Runtime::new(build(DragStartBehavior::Down)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let start = Instant::now();
    focus.request_focus();
    frame(&mut runtime, start);
    let field = runtime.focused_element().unwrap();
    let root = runtime.tree().root().unwrap();

    for (iteration, (behavior, later_line)) in [
        (DragStartBehavior::Down, true),
        (DragStartBehavior::Start, false),
    ]
    .into_iter()
    .enumerate()
    {
        let at = start + Duration::from_millis(iteration as u64 * 600);
        runtime.schedule_update(root, build(behavior)).unwrap();
        controller.set_selection(TextSelection::new(0, 4));
        frame(&mut runtime, at);
        assert_eq!(runtime.focused_element(), Some(field));
        assert_eq!(
            runtime.tree().editable_drag_start_behavior(field),
            Some(behavior)
        );
        let geometry = runtime
            .tree()
            .text_field_selection_handle_geometry(field)
            .unwrap();
        let state = runtime.tree().text_field_selection_handles(field).unwrap();
        state.show(geometry);
        frame(&mut runtime, at);
        let end = geometry.end.unwrap();
        let area = end.rect();
        let knob = area.origin + Offset::new(area.size.width * 0.5, area.size.height * 0.5);
        let moved = knob + Offset::new(0., end.line_height * 2.2);
        pointer(
            &mut runtime,
            PointerPhase::Down,
            knob,
            at + Duration::from_millis(10),
        );
        pointer(
            &mut runtime,
            PointerPhase::Move,
            moved,
            at + Duration::from_millis(30),
        );
        assert_eq!(controller.selection().base, 0);
        assert_eq!(
            controller.selection().extent > "first line".len(),
            later_line
        );
        pointer(
            &mut runtime,
            PointerPhase::Up,
            moved,
            at + Duration::from_millis(50),
        );
        frame(&mut runtime, at + Duration::from_millis(50));
        assert_eq!(runtime.focused_element(), Some(field));
    }
}

#[test]
fn a_toolbar_is_not_restored_while_device_padding_hides_its_selection() {
    use incular_config::EdgeInsets;
    use incular_text::{TextRange, TextSelection, TextStyle};
    use incular_widgets::FocusNode;
    use serde_json::json;

    let controller = TextEditingController::with_text(vec!["abcd"; 20].join("\n"));
    controller.set_selection(TextSelection::new(50, 54));
    let focus = FocusNode::new();
    const TOOLBAR: &str = "device-clipped toolbar";
    let editor = EditableText::new(controller.clone())
        .focus_node(focus.clone())
        .text_direction(TextDirection::Ltr)
        .style(TextStyle::new().font_size(14.).height(Some(1.)))
        .max_lines(None)
        .allow_implicit_caret_scrolling(false)
        .context_menu_builder(|_, _| Widget::from(SizedBox::shrink()).with_key(TOOLBAR));
    let mut runtime = Runtime::new(editor.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let mut environment = runtime.environment();
    environment.viewport = Size::new(400., 200.);
    environment.safe_insets = EdgeInsets::only(0., 0., 0., 120.);
    assert!(runtime.set_environment(environment.clone()));
    let now = Instant::now();
    let time = |ms| now + Duration::from_millis(ms);
    focus.request_focus();
    frame(&mut runtime, time(0));
    let field = runtime.focused_element().unwrap();
    let geometry = runtime.tree().editable_selection_menu_rect(field).unwrap();
    assert!(geometry.origin.y > 120. && geometry.origin.y < 200.);
    let client = runtime.flutter_text_input_configuration().unwrap().0.get();
    let request =
        serde_json::to_vec(&json!({"method":"TextInputClient.showToolbar","args":[client]}))
            .unwrap();
    assert_eq!(
        runtime.handle_flutter_text_toolbar_call(&request).unwrap(),
        Some(true)
    );
    frame(&mut runtime, time(10));
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from(TOOLBAR))
            .is_some()
    );

    let drag = |runtime: &mut Runtime, signed: f32, start| {
        let point = Offset::new(180., 60.);
        pointer(runtime, PointerPhase::Down, point, time(start));
        pointer(
            runtime,
            PointerPhase::Move,
            point + Offset::new(0., signed * 22.),
            time(start + 50),
        );
        frame(runtime, time(start + 50));
        let moved = point + Offset::new(0., signed * 32.);
        pointer(runtime, PointerPhase::Move, moved, time(start + 100));
        frame(runtime, time(start + 100));
        assert!(!toolbar_visible(runtime, field));
        pointer(runtime, PointerPhase::Up, moved, time(start + 130));
        frame(runtime, time(start + 130));
        frame(runtime, time(start + 150));
    };
    drag(&mut runtime, -1., 300);
    assert_eq!(
        runtime.tree().text_field_scroll_offset(field).unwrap().y,
        10.
    );
    assert_eq!(controller.selection().range(), TextRange::new(50, 54));
    assert!(!toolbar_visible(&runtime, field));
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from(TOOLBAR))
            .is_none()
    );

    environment.safe_insets = EdgeInsets::ZERO;
    assert!(runtime.set_environment(environment));
    drag(&mut runtime, 1., 700);
    assert!(toolbar_visible(&runtime, field));
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from(TOOLBAR))
            .is_some()
    );
}

#[test]
fn a_retained_handle_uses_the_latest_tap_callback_without_treating_a_drag_as_a_tap() {
    use incular_config::Alignment;
    use incular_widgets::{Align, FocusNode};
    use std::{cell::Cell, rc::Rc};

    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let count = Rc::new(Cell::new(0));
    let build = |with_callback| {
        let mut editor = EditableText::new(controller.clone())
            .focus_node(focus.clone())
            .text_direction(TextDirection::Ltr)
            .max_lines(Some(10))
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        if with_callback {
            let count = count.clone();
            editor = editor.on_selection_handle_tapped(move || count.set(count.get() + 1));
        }
        Widget::from(Align::new(Alignment::TOP_LEFT, editor))
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let now = Instant::now();
    let at = |ms| now + Duration::from_millis(ms);
    focus.request_focus();
    frame(&mut runtime, at(0));
    let field = runtime.focused_element().unwrap();
    let handles = runtime
        .tree()
        .text_field_selection_handle_geometry(field)
        .unwrap();
    runtime
        .tree()
        .text_field_selection_handles(field)
        .unwrap()
        .show(handles);
    frame(&mut runtime, at(0));
    let area = handles.start.rect();
    let point = area.origin + Offset::new(area.size.width * 0.5, area.size.height * 0.5);
    let tap = |runtime: &mut Runtime, start| {
        pointer(runtime, PointerPhase::Down, point, at(start));
        pointer(runtime, PointerPhase::Up, point, at(start + 20));
        frame(runtime, at(start + 20));
    };

    tap(&mut runtime, 10);
    assert_eq!(count.get(), 0);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(true)).unwrap();
    frame(&mut runtime, at(100));
    assert_eq!(runtime.focused_element(), Some(field));
    assert!(
        runtime
            .tree()
            .text_field_selection_handles(field)
            .unwrap()
            .is_showing()
    );
    tap(&mut runtime, 120);
    assert_eq!(count.get(), 1);

    pointer(&mut runtime, PointerPhase::Down, point, at(300));
    let moved = point + Offset::new(40., 0.);
    pointer(&mut runtime, PointerPhase::Move, moved, at(350));
    pointer(&mut runtime, PointerPhase::Up, moved, at(380));
    assert_eq!(count.get(), 1);

    runtime.schedule_update(root, build(false)).unwrap();
    frame(&mut runtime, at(500));
    tap(&mut runtime, 550);
    assert_eq!(count.get(), 1);
    assert!(focus.has_primary_focus());
}

#[test]
fn a_custom_toolbar_only_mounts_when_shown_and_uses_its_updated_builder() {
    use incular_config::Alignment;
    use incular_widgets::{Align, FocusNode};

    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let build = |key| {
        Widget::from(Align::new(
            Alignment::TOP_LEFT,
            EditableText::new(controller.clone())
                .focus_node(focus.clone())
                .autofocus(true)
                .text_direction(TextDirection::Ltr)
                .size(Size::new(400., 100.))
                .context_menu_builder(move |_, _| {
                    Widget::from(SizedBox::new().width(10.).height(10.)).with_key(key)
                }),
        ))
    };
    let mut runtime = Runtime::new(build("old toolbar")).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let start = Instant::now();
    let at = |ms| start + Duration::from_millis(ms);
    frame(&mut runtime, at(0));
    let field = runtime.focused_element().unwrap();
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("old toolbar"))
            .is_none()
    );
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("new toolbar"))
            .is_none()
    );

    let point = Offset::new(200., 40.);
    pointer(&mut runtime, PointerPhase::Down, point, at(10));
    frame(&mut runtime, at(510));
    pointer(&mut runtime, PointerPhase::Up, point, at(530));
    frame(&mut runtime, at(530));
    let old = runtime
        .tree()
        .element_with_key(&Key::from("old toolbar"))
        .unwrap();
    assert_eq!(
        runtime.tree().element_layout_size(old),
        Some(Size::new(400., 200.))
    );
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("new toolbar"))
            .is_none()
    );

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build("new toolbar")).unwrap();
    frame(&mut runtime, at(560));
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("old toolbar"))
            .is_none()
    );
    let new = runtime
        .tree()
        .element_with_key(&Key::from("new toolbar"))
        .unwrap();
    assert_eq!(
        runtime.tree().element_layout_size(new),
        Some(Size::new(400., 200.))
    );
    assert_eq!(runtime.focused_element(), Some(field));
}

#[test]
fn an_inline_menu_builder_update_preserves_the_open_menu_child_state() {
    use incular_config::Alignment;
    use incular_widgets::{Align, FocusNode, StateSetter, StatefulBuilder};
    use serde_json::json;
    use std::{cell::RefCell, rc::Rc};

    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let first_setter = Rc::new(RefCell::new(None::<StateSetter>));
    let build = |current| {
        let first_setter = first_setter.clone();
        Widget::from(Align::new(
            Alignment::TOP_LEFT,
            EditableText::new(controller.clone())
                .focus_node(focus.clone())
                .autofocus(true)
                .text_direction(TextDirection::Ltr)
                .context_menu_builder(move |_, _| {
                    let first_setter = first_setter.clone();
                    Widget::from(StatefulBuilder::new(move |_, setter| {
                        let mut first = first_setter.borrow_mut();
                        if first.is_none() {
                            *first = Some(setter);
                        }
                        Widget::from(SizedBox::shrink()).with_key(current)
                    }))
                    .with_key("stable stateful menu")
                }),
        ))
    };
    let mut runtime = Runtime::new(build("current 1")).unwrap();
    let start = Instant::now();
    frame(&mut runtime, start);
    let field = runtime.focused_element().unwrap();
    let client = runtime.flutter_text_input_configuration().unwrap().0.get();
    let show = serde_json::to_vec(&json!({"method":"TextInputClient.showToolbar","args":[client]}))
        .unwrap();
    assert_eq!(
        runtime.handle_flutter_text_toolbar_call(&show).unwrap(),
        Some(true)
    );
    frame(&mut runtime, start + Duration::from_millis(20));
    let first = first_setter.borrow().as_ref().unwrap().clone();
    let menu = runtime
        .tree()
        .element_with_key(&Key::from("stable stateful menu"))
        .unwrap();
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("current 1"))
            .is_some()
    );

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build("current 2")).unwrap();
    frame(&mut runtime, start + Duration::from_millis(40));
    assert_eq!(
        runtime
            .tree()
            .element_with_key(&Key::from("stable stateful menu")),
        Some(menu)
    );
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("current 1"))
            .is_none()
    );
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("current 2"))
            .is_some()
    );
    assert!(
        first.set_state(|| {}),
        "the original state is still mounted"
    );
    assert_eq!(runtime.focused_element(), Some(field));
}

fn run_private_axis_drag(
    platform: TextShortcutPlatform,
    multiline: bool,
) -> (Offset, f32, bool, bool) {
    use incular_config::Alignment;
    use incular_text::TextStyle;
    use incular_widgets::Align;

    let text = TextEditingController::with_text(if multiline {
        "XXXXX\nXXXXX\nXXXXX"
    } else {
        "This is a long string that should overflow the TextField."
    });
    let mut style = TextStyle::new().font_size(14.);
    if let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") {
        const FONT: &str = "FlutterTest editable scroll axis";
        incular_text::register_application_font_as(FONT, std::fs::read(font).unwrap());
        style = style.font_family(FONT);
    }
    let field_widget = EditableText::new(text.clone())
        .text_direction(TextDirection::Ltr)
        .max_lines(Some(if multiline { 2 } else { 1 }))
        .style(style);
    let field_widget = Widget::from(field_widget).with_key("scrolling edit");
    let mut runtime = Runtime::new(
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(100.).child(field_widget),
        )
        .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    let start = Instant::now();
    let time = |ms| start + Duration::from_millis(ms);
    frame(&mut runtime, time(0));
    let field = runtime
        .tree()
        .element_with_key(&Key::from("scrolling edit"))
        .unwrap();
    assert_eq!(
        runtime.tree().text_field_scroll_offset(field),
        Some(Offset::ZERO)
    );
    let max = runtime.tree().editable_max_scroll_extent(field).unwrap();
    assert!(max > 0.);
    assert!(runtime.focused_element().is_none());
    let down = Offset::new(50., if multiline { 14. } else { 7. });
    let at_delta = |distance| {
        down - if multiline {
            Offset::new(0., distance)
        } else {
            Offset::new(distance, 0.)
        }
    };
    for (phase, distance, ms) in [
        (PointerPhase::Down, 0., 10),
        (PointerPhase::Move, 20., 40),
        (PointerPhase::Move, 100., 80),
        (PointerPhase::Up, 100., 120),
    ] {
        pointer(&mut runtime, phase, at_delta(distance), time(ms));
        frame(&mut runtime, time(ms));
    }
    frame(&mut runtime, time(500));
    (
        runtime.tree().text_field_scroll_offset(field).unwrap(),
        max,
        runtime.focused_element().is_some(),
        text.selection_if_set().is_some(),
    )
}

#[test]
fn ios_single_line_does_not_consume_horizontal_finger_scroll() {
    let (offset, max, _, _) = run_private_axis_drag(TextShortcutPlatform::Ios, false);
    assert!(max > 100.);
    assert_eq!(offset, Offset::ZERO);
}

#[test]
fn ios_two_line_editor_scrolls_its_last_line_into_view_on_finger_drag() {
    let (offset, max, _, _) = run_private_axis_drag(TextShortcutPlatform::Ios, true);
    assert_eq!(offset, Offset::new(0., max));
}

#[test]
fn android_single_line_scrolls_horizontally_without_selecting_or_focusing() {
    let (offset, max, focused, selected) =
        run_private_axis_drag(TextShortcutPlatform::Android, false);
    assert!(max > 100.);
    assert_eq!(offset, Offset::new(80., 0.));
    assert!(!focused);
    assert!(!selected);
}
