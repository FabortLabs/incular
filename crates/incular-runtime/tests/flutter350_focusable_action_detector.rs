use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset, PointerPhase, Size,
    time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    ActionScope, Actions, Center, Column, Directionality, Focus, FocusHighlightManager,
    FocusHighlightStrategy, FocusNode, FocusableActionDetector, LogicalShortcutKey, MouseCursor,
    MouseRegion, PointerDeviceKind, Shortcuts, SizedBox, Text, Widget, internal::RawPointerEvent,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn move_mouse(runtime: &mut Runtime, pos: Offset) {
    let _ = runtime.handle_raw_pointer_event(RawPointerEvent::new(
        1,
        1,
        PointerDeviceKind::Mouse,
        0,
        pos,
        PointerPhase::Move,
        Instant::now(),
    ));
    frame(runtime);
}

fn enter(runtime: &mut Runtime) {
    let key = KeyboardKey::Named(NamedKey::Enter);
    runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), Code::Enter));
    runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, Code::Enter));
    frame(runtime);
}

fn hover_focus_through_disabled_detector(outer_scope: bool) {
    let mode = FocusHighlightManager::new();
    mode.set_strategy(FocusHighlightStrategy::AlwaysTraditional);
    let focus = FocusNode::new();
    let hovered = Rc::new(Cell::new(false));
    let focused = Rc::new(Cell::new(false));
    let called = Rc::new(Cell::new(false));
    let mut actions = Actions::new();
    let invoked = called.clone();
    actions.register("run", move || invoked.set(true));
    let actions = Rc::new(actions);
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Named(NamedKey::Enter), Modifiers::empty()),
        "run",
    );
    let shortcuts = Rc::new(shortcuts);
    let build = |enabled| {
        let hover = hovered.clone();
        let focus_highlight = focused.clone();
        let detector = FocusableActionDetector::new(SizedBox::new().width(100.).height(100.))
            .enabled(enabled)
            .focus_node(focus.clone())
            .shortcuts(shortcuts.clone())
            .actions(actions.clone())
            .on_show_hover_highlight(move |show| hover.set(show))
            .on_show_focus_highlight(move |show| focus_highlight.set(show));
        if outer_scope {
            let empty = Rc::new(Actions::new());
            let dispatcher =
                Rc::new(|call: incular_widgets::ActionDispatchInvocation<'_>| call.invoke());
            Center::new(ActionScope::new(empty, detector).dispatcher(dispatcher)).into()
        } else {
            Center::new(detector).into()
        }
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    let root = runtime.tree().root().unwrap();
    frame(&mut runtime);
    focus.request_focus();
    frame(&mut runtime);
    move_mouse(&mut runtime, Offset::new(400., 300.));
    enter(&mut runtime);
    assert!(hovered.get());
    assert!(focused.get());
    assert!(called.get());

    called.set(false);
    runtime.schedule_update(root, build(false)).unwrap();
    frame(&mut runtime);
    assert!(!hovered.get());
    assert!(!focused.get());
    enter(&mut runtime);
    assert!(!called.get());

    runtime.schedule_update(root, build(true)).unwrap();
    frame(&mut runtime);
    assert!(hovered.get());
    assert!(!focused.get());
    runtime.schedule_update(root, build(false)).unwrap();
    frame(&mut runtime);
    assert!(!hovered.get());
    assert!(!focused.get());
    move_mouse(&mut runtime, Offset::ZERO);
    runtime.schedule_update(root, build(true)).unwrap();
    frame(&mut runtime);
    assert!(!hovered.get());
    assert!(!focused.get());
}

#[test]
fn retained_detector_updates_focus_and_hover_highlights_when_enabled_changes() {
    hover_focus_through_disabled_detector(false);
    hover_focus_through_disabled_detector(true);
}

#[test]
fn detector_mouse_cursor_defaults_back_to_an_enclosing_region() {
    let scene = |custom| {
        let child = SizedBox::new().width(20.).height(20.);
        let detector = FocusableActionDetector::new(child);
        let detector = if custom {
            detector.mouse_cursor(MouseCursor::Text)
        } else {
            detector
        };
        MouseRegion::new(detector)
            .cursor(MouseCursor::NotAllowed)
            .into()
    };
    let mut runtime = Runtime::new(scene(true)).unwrap();
    frame(&mut runtime);
    let at = Offset::new(1., 1.);
    move_mouse(&mut runtime, at);
    assert_eq!(runtime.tree().mouse_cursor_at(at), MouseCursor::Text);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(false)).unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.tree().mouse_cursor_at(at), MouseCursor::NotAllowed);
}

#[test]
fn skipping_detector_traversal_keeps_explicit_focus_possible() {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let middle = FocusNode::new();
    let scene = |skip| {
        let item = |node: &FocusNode| Focus::new(SizedBox::new().height(30.)).node(node.clone());
        let detector =
            FocusableActionDetector::new(SizedBox::new().height(30.)).focus_node(middle.clone());
        let detector = if skip {
            detector.skip_traversal(true)
        } else {
            detector
        };
        let list = Column::new([Widget::from(item(&a)), detector.into(), item(&b).into()]);
        Directionality::new(TextDirection::Ltr, list).into()
    };
    let mut runtime = Runtime::new(scene(false)).unwrap();
    frame(&mut runtime);
    a.request_focus();
    frame(&mut runtime);
    assert!(runtime.next_focus_from(&a));
    assert!(middle.has_primary_focus());
    assert!(!b.has_focus());

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(true)).unwrap();
    frame(&mut runtime);
    a.request_focus();
    frame(&mut runtime);
    assert!(runtime.next_focus_from(&a));
    assert!(b.has_primary_focus());
    assert!(!middle.has_focus());
    middle.request_focus();
    frame(&mut runtime);
    assert!(middle.has_primary_focus());
}

#[test]
fn disabling_detector_focus_semantics_keeps_descendant_labels() {
    let scene = |include| {
        let child = Column::new([Text::new("First"), Text::new("Second")]);
        let detector = FocusableActionDetector::new(child);
        let detector = if include {
            detector
        } else {
            detector.include_focus_semantics(false)
        };
        assert_eq!(detector.includes_focus_semantics(), include);
        Directionality::new(TextDirection::Ltr, detector).into()
    };
    let mut runtime = Runtime::new(scene(true)).unwrap();
    frame(&mut runtime);
    let check = |runtime: &Runtime, focusable| {
        let semantics = runtime.tree().semantics();
        assert_eq!(
            semantics.iter().any(|(_, node)| node.state.focusable),
            focusable
        );
        let labels: Vec<_> = semantics
            .iter()
            .filter_map(|(_, node)| node.label.as_deref())
            .collect();
        // A focus annotation can combine the descendant labels into one node.
        assert!(
            labels
                .iter()
                .flat_map(|label| label.split_whitespace())
                .eq(["First", "Second"])
        );
    };
    check(&runtime, true);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(false)).unwrap();
    frame(&mut runtime);
    check(&runtime, false);
}
