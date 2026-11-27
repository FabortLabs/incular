use std::rc::Rc;

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::{KeyMessage, Runtime};
use incular_widgets::{
    Column, DirectionalFocusIntent, Directionality, EditableText, FocusNode, FocusScope,
    FocusableActionDetector, LogicalShortcutKey, RawKeyEvent, Row, Shortcuts, SizedBox,
    TextDirection, TraversalDirection, Widget, internal::TextEditingController,
};

const ARROWS: [(Code, NamedKey, TraversalDirection); 4] = [
    (
        Code::ArrowRight,
        NamedKey::ArrowRight,
        TraversalDirection::Right,
    ),
    (
        Code::ArrowDown,
        NamedKey::ArrowDown,
        TraversalDirection::Down,
    ),
    (
        Code::ArrowLeft,
        NamedKey::ArrowLeft,
        TraversalDirection::Left,
    ),
    (Code::ArrowUp, NamedKey::ArrowUp, TraversalDirection::Up),
];

fn run_shortcut_variant(raw_only: bool) {
    let nodes = std::array::from_fn::<_, 4, _>(|_| FocusNode::new());
    let controllers = std::array::from_fn::<_, 4, _>(|_| TextEditingController::new());
    let host = FocusNode::new();
    host.set_skip_traversal(true);
    let widget = |ignore_fields| {
        let input = |index: usize| {
            SizedBox::new().width(100.).height(100.).child(
                EditableText::new(controllers[index].clone())
                    .focus_node(nodes[index].clone())
                    .autofocus(index == 0),
            )
        };
        let mut shortcuts = Shortcuts::<DirectionalFocusIntent>::typed();
        for (_, key, direction) in ARROWS {
            shortcuts.bind_logical(
                LogicalShortcutKey::new(KeyboardKey::Named(key), Modifiers::empty()),
                DirectionalFocusIntent::new(direction).ignore_text_fields(ignore_fields),
            );
        }
        let scene = FocusScope::new(Column::new([
            Row::new([input(0), input(1)]),
            Row::new([input(2), input(3)]),
        ]));
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            FocusableActionDetector::new(scene)
                .focus_node(host.clone())
                .shortcuts(Rc::new(shortcuts)),
        ))
    };
    let mut runtime = Runtime::new(widget(false)).unwrap();
    let view = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(view).unwrap();
    assert!(nodes[0].has_primary_focus());
    let send = |runtime: &mut Runtime, key, code| {
        let key = KeyboardKey::Named(key);
        for (typed, down) in [
            (KeyboardEvent::key_down(key.clone(), code), true),
            (KeyboardEvent::key_up(key, code), false),
        ] {
            let raw = RawKeyEvent::from(typed.clone());
            let event = if raw_only {
                KeyMessage::Raw(raw)
            } else {
                KeyMessage::TypedAndRaw { typed, raw }
            };
            let handled = runtime.dispatch_key_message(event);
            if down {
                assert!(handled);
            }
        }
        runtime.run_frame(view).unwrap();
    };

    for ((code, name, _), expected) in ARROWS.into_iter().zip([1, 3, 2, 0]) {
        send(&mut runtime, name, code);
        assert!(nodes[expected].has_primary_focus(), "{code:?}");
    }
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, widget(true)).unwrap();
    runtime.run_frame(view).unwrap();
    assert!(nodes[0].has_primary_focus());
    for (code, name, _) in ARROWS {
        send(&mut runtime, name, code);
        assert!(nodes[0].has_primary_focus(), "{code:?}");
        assert!(nodes[1..].iter().all(|node| !node.has_focus()));
    }
}

#[test]
fn editable_directional_shortcuts_opt_in_with_raw_keys() {
    run_shortcut_variant(true);
}

#[test]
fn editable_directional_shortcuts_opt_in_with_paired_keys() {
    run_shortcut_variant(false);
}

#[test]
fn a_custom_keyboard_listener_action_precedes_the_builtin_directional_intent() {
    use incular_widgets::{Actions, Focus, KeyboardListener};
    use std::cell::Cell;

    let first = FocusNode::new();
    let second = FocusNode::new();
    let calls = Rc::new(Cell::new(0));
    let binding = DirectionalFocusIntent::new(TraversalDirection::Right);
    let build = |custom| {
        let key =
            LogicalShortcutKey::new(KeyboardKey::Named(NamedKey::ArrowRight), Modifiers::empty());
        let mut shortcuts = Shortcuts::<DirectionalFocusIntent>::typed();
        shortcuts.bind_logical(key, binding);
        let mut actions = Actions::<DirectionalFocusIntent>::typed();
        if custom {
            let calls = calls.clone();
            actions.register(binding, move || calls.set(calls.get() + 1));
        }
        let stop = |node: &FocusNode| {
            Focus::new(SizedBox::new().width(100.).height(100.)).node(node.clone())
        };
        let row = FocusScope::new(Row::new([stop(&first), stop(&second)]));
        Directionality::new(
            TextDirection::Ltr,
            KeyboardListener::new(row).with_shortcuts(Rc::new(shortcuts), Rc::new(actions)),
        )
        .into()
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(build(false)).unwrap();
    runtime.run_frame(bounds).unwrap();
    let right = |runtime: &mut Runtime| {
        let key = KeyboardKey::Named(NamedKey::ArrowRight);
        assert!(
            runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), Code::ArrowRight))
        );
        runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, Code::ArrowRight));
    };
    first.request_focus();
    runtime.run_frame(bounds).unwrap();
    right(&mut runtime);
    assert!(second.has_primary_focus());
    assert_eq!(calls.get(), 0);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(true)).unwrap();
    first.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(first.has_primary_focus());
    right(&mut runtime);
    assert!(first.has_primary_focus());
    assert_eq!(calls.get(), 1);
}

#[test]
fn an_unbound_arrow_still_moves_the_editable_caret() {
    use incular_widgets::internal::TextSelection;

    let node = FocusNode::new();
    let controller = TextEditingController::new();
    controller.set_text("abc");
    let editable = EditableText::new(controller.clone()).focus_node(node.clone());
    let root = Directionality::new(TextDirection::Ltr, editable);
    let mut runtime = Runtime::new(root.into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    node.request_focus();
    runtime.run_frame(bounds).unwrap();
    controller.set_selection(TextSelection::collapsed(1));
    let key = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::ArrowRight), Code::ArrowRight);
    assert!(runtime.dispatch_keyboard_event(key));
    assert!(node.has_primary_focus());
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
}
