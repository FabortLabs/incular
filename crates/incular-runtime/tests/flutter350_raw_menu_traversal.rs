use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Code, Color, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset,
    PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Actions, Center, Column, Command, Focus, FocusNode, GestureDetector, KeyboardListener,
    LogicalShortcutKey, MainAxisSize, MenuController, Positioned, RawMenuAnchor,
    RawMenuAnchorGroup, Shortcuts, Stack, Widget, internal::Key,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn key(runtime: &mut Runtime, logical: NamedKey, code: Code, mods: Modifiers) -> bool {
    let logical = KeyboardKey::Named(logical);
    let mut down = KeyboardEvent::key_down(logical.clone(), code);
    down.modifiers = mods;
    let mut up = KeyboardEvent::key_up(logical, code);
    up.modifiers = mods;
    let handled = runtime.dispatch_keyboard_event(down);
    let _ = runtime.dispatch_keyboard_event(up);
    handled
}

fn click_anchor(runtime: &mut Runtime) {
    let id = runtime
        .tree()
        .element_with_key(&Key::from("anchor"))
        .unwrap();
    let bounds = runtime.tree().element_bounds(id).unwrap();
    let position = bounds.origin + Offset::new(10., 10.);
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 0,
            kind: PointerDeviceKind::Touch,
            buttons,
            button: Some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        });
    }
    frame(runtime);
}

fn focus_box(node: &FocusNode) -> Widget {
    Focus::new(Widget::box_(Size::new(120., 30.), Color::WHITE))
        .node(node.clone())
        .into()
}

fn check_actions(grouped: bool) {
    let controller = MenuController::new();
    let anchor_focus = FocusNode::new();
    let popup_focus = FocusNode::new();
    let popup: Widget = Column::new([
        focus_box(&FocusNode::new()),
        focus_box(&popup_focus),
        focus_box(&FocusNode::new()),
    ])
    .main_axis_size(MainAxisSize::Min)
    .into();
    let anchor = RawMenuAnchor::new(controller.clone(), move |_, info| {
        let bounds = info.anchor_rect;
        Stack::new([Widget::from(
            Positioned::new(popup.clone())
                .left(bounds.origin.x)
                .top(bounds.origin.y + bounds.size.height),
        )])
        .into()
    })
    .child(
        Widget::from(GestureDetector::new(focus_box(&anchor_focus)).on_tap({
            let controller = controller.clone();
            move || controller.open(None)
        }))
        .with_key(Key::from("anchor")),
    );
    let anchor = if grouped {
        Widget::from(RawMenuAnchorGroup::new(MenuController::new(), anchor))
    } else {
        Widget::from(anchor)
    };

    let log = Rc::new(RefCell::new(Vec::new()));
    let mut actions = Actions::new();
    let mut shortcuts = Shortcuts::new();
    for (intent, logical, mods) in [
        ("left", NamedKey::ArrowLeft, Modifiers::empty()),
        ("next", NamedKey::Tab, Modifiers::empty()),
        ("previous", NamedKey::Tab, Modifiers::SHIFT),
    ] {
        let command = Command::new(intent);
        let log = log.clone();
        actions.register(command.clone(), move || log.borrow_mut().push(intent));
        shortcuts.bind_logical(
            LogicalShortcutKey::new(KeyboardKey::Named(logical), mods),
            command,
        );
    }
    let scene = KeyboardListener::new(anchor).with_shortcuts(Rc::new(shortcuts), Rc::new(actions));
    let mut runtime = Runtime::new(Center::new(scene).into()).unwrap();
    frame(&mut runtime);
    if !grouped {
        controller.open(None);
        frame(&mut runtime);
    }

    for (index, node) in [anchor_focus, popup_focus].into_iter().enumerate() {
        if grouped && index == 1 {
            click_anchor(&mut runtime);
        }
        node.request_focus();
        frame(&mut runtime);
        assert!(node.has_primary_focus());
        if !grouped {
            assert!(key(
                &mut runtime,
                NamedKey::ArrowLeft,
                Code::ArrowLeft,
                Modifiers::empty()
            ));
            assert!(node.has_primary_focus());
        }
        assert!(key(
            &mut runtime,
            NamedKey::Tab,
            Code::Tab,
            Modifiers::empty()
        ));
        assert!(node.has_primary_focus());
        assert!(key(
            &mut runtime,
            NamedKey::Tab,
            Code::Tab,
            Modifiers::SHIFT
        ));
        assert!(node.has_primary_focus());
        if !grouped {
            for (logical, code) in [(NamedKey::Home, Code::Home), (NamedKey::End, Code::End)] {
                assert!(!key(&mut runtime, logical, code, Modifiers::empty()));
                assert!(node.has_primary_focus());
            }
        }
    }
    let expected = if grouped {
        &["next", "previous", "next", "previous"][..]
    } else {
        &["left", "next", "previous", "left", "next", "previous"][..]
    };
    assert_eq!(&*log.borrow(), expected);
    assert!(controller.is_open());
}

#[test]
fn a_raw_anchor_does_not_override_application_focus_shortcuts() {
    check_actions(false);
}

#[test]
fn a_raw_group_leaves_tab_traversal_to_the_application_in_anchor_and_panel() {
    check_actions(true);
}

#[test]
fn a_closed_anchor_does_not_shadow_programmatically_invoked_application_intents() {
    use incular_widgets::{ActionResult, DismissIntent, FocusableActionDetector, Intent};

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    enum Traversal {
        Up,
        Next,
        Previous,
    }

    let log = Rc::new(RefCell::new(Vec::new()));
    let mut navigation = Actions::<Traversal>::typed();
    for (intent, label) in [
        (Traversal::Up, "up"),
        (Traversal::Next, "next"),
        (Traversal::Previous, "previous"),
    ] {
        let log = log.clone();
        navigation.register(intent, move || log.borrow_mut().push(label));
    }
    let mut dismiss = Actions::<DismissIntent>::typed();
    dismiss.register(DismissIntent, {
        let log = log.clone();
        move || log.borrow_mut().push("dismiss")
    });
    let focus = FocusNode::new();
    let menu = MenuController::new();
    let anchor = RawMenuAnchor::new(menu.clone(), |_, _| {
        Widget::box_(Size::new(70., 30.), Color::WHITE)
    })
    .child(Focus::new(Widget::box_(Size::new(100., 30.), Color::WHITE)).node(focus.clone()));
    let scene = FocusableActionDetector::new(
        FocusableActionDetector::new(RawMenuAnchorGroup::new(MenuController::new(), anchor))
            .actions(Rc::new(dismiss)),
    )
    .actions(Rc::new(navigation));
    let mut runtime = Runtime::new(Center::new(scene).into()).unwrap();
    frame(&mut runtime);
    focus.request_focus();
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    let id = runtime.focused_element().unwrap();

    for command in [Traversal::Up, Traversal::Next, Traversal::Previous] {
        assert_eq!(
            runtime.invoke_action_at(id, &Intent::<Traversal>::new(command)),
            Some(ActionResult::Handled),
        );
    }
    assert_eq!(
        runtime.invoke_action_at(id, &Intent::<DismissIntent>::new(DismissIntent)),
        Some(ActionResult::Handled),
    );
    assert_eq!(*log.borrow(), ["up", "next", "previous", "dismiss"]);

    menu.open(None);
    frame(&mut runtime);
    assert!(menu.is_open());
    assert_eq!(
        runtime.invoke_action_at(id, &Intent::<DismissIntent>::new(DismissIntent)),
        Some(ActionResult::Handled),
    );
    frame(&mut runtime);
    assert!(!menu.is_open());
    assert_eq!(*log.borrow(), ["up", "next", "previous", "dismiss"]);
}

#[test]
fn anchor_and_panel_can_invoke_the_same_inherited_action_with_distinct_payloads() {
    use incular_widgets::{Action, ActionResult, FocusableActionDetector, Intent};
    use std::{
        cell::Cell,
        hash::{Hash, Hasher},
    };

    // The command type identifies the action. Each invocation supplies a
    // different callback payload, like Flutter's VoidCallbackIntent.
    #[derive(Clone)]
    struct Callback(Rc<dyn Fn()>);
    impl PartialEq for Callback {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }
    impl Eq for Callback {}
    impl Hash for Callback {
        fn hash<H: Hasher>(&self, state: &mut H) {
            0_u8.hash(state);
        }
    }
    let mut actions = Actions::<Callback>::typed();
    actions.register_action(Action::with_handler(Callback(Rc::new(|| {})), |intent| {
        (intent.command().0)();
        ActionResult::Handled
    }));

    let anchor_node = FocusNode::new();
    let popup_node = FocusNode::new();
    let menu = MenuController::new();
    let popup = focus_box(&popup_node);
    let anchor = RawMenuAnchor::new(menu.clone(), move |_, info| {
        let rect = info.anchor_rect;
        Stack::new([Widget::from(
            Positioned::new(popup.clone())
                .left(rect.origin.x)
                .top(rect.origin.y + rect.size.height),
        )])
        .into()
    })
    .child(
        Widget::from(GestureDetector::new(focus_box(&anchor_node)).on_tap({
            let menu = menu.clone();
            move || menu.open(None)
        }))
        .with_key(Key::from("anchor")),
    );
    let scene = FocusableActionDetector::new(anchor).actions(Rc::new(actions));
    let mut runtime = Runtime::new(Center::new(scene).into()).unwrap();
    frame(&mut runtime);
    click_anchor(&mut runtime);
    assert!(menu.is_open());

    let mut observed = Vec::new();
    for node in [anchor_node, popup_node] {
        node.request_focus();
        frame(&mut runtime);
        assert!(node.has_primary_focus());
        let invoked = Rc::new(Cell::new(0));
        let intent = Intent::<Callback>::new(Callback({
            let count = invoked.clone();
            Rc::new(move || count.set(count.get() + 1))
        }));
        assert_eq!(
            runtime.invoke_action_at(runtime.focused_element().unwrap(), &intent),
            Some(ActionResult::Handled),
        );
        assert!(menu.is_open());
        observed.push(invoked);
    }
    assert_eq!([observed[0].get(), observed[1].get()], [1, 1]);
}
