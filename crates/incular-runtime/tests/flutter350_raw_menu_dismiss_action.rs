use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_widgets::{
    ActionInvocationPhase, ActionResult, Center, Command, DismissMenuAction, Focus, FocusNode,
    GestureDetector, Intent, MenuController, RawMenuAnchor, Widget, internal::Key,
};

const PANEL: Color = Color::rgba(29, 76, 81, 255);
fn frame(runtime: &mut Runtime) -> bool {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap()
        .0
        .commands()
        .iter()
        .any(|cmd| matches!(cmd, PaintCommand::Rect { color, .. } if *color == PANEL))
}

fn tap(runtime: &mut Runtime, key: Key) {
    let id = runtime
        .tree()
        .element_with_key(&key)
        .expect("live anchor button");
    let bounds = runtime.tree().element_bounds(id).unwrap();
    let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 0,
                kind: PointerDeviceKind::Touch,
                buttons,
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position,
            },
            Instant::now(),
        );
    }
}

fn exercise_dismiss_action(defer: bool) {
    let controllers: Vec<_> = (0..3).map(|_| MenuController::new()).collect();
    let focus = FocusNode::new();
    let close_requests = Rc::new(RefCell::new(Vec::new()));
    let closes = Rc::new(RefCell::new(Vec::new()));
    let action = DismissMenuAction::new(controllers[2].clone());
    let intent = Intent::<Command>::new(Command::new("dismiss"));
    let registered = action.clone().into_action(intent.clone());
    let invocations = Rc::new(RefCell::new(Vec::new()));
    let _invocation_subscription = registered.add_invocation_listener({
        let log = invocations.clone();
        move |phase| log.borrow_mut().push(phase)
    });
    assert!(!action.is_enabled());
    assert!(!registered.is_action_enabled(&intent));
    assert_eq!(registered.invoke(&intent), ActionResult::Ignored);
    assert!(invocations.borrow().is_empty());

    let mut content = Widget::box_(Size::new(50., 50.), PANEL);
    for level in (0..3).rev() {
        let controller = controllers[level].clone();
        let button = Widget::from(
            GestureDetector::new(Widget::box_(Size::new(80., 30.), Color::WHITE))
                .on_tap(move || controller.open(None)),
        )
        .with_key(Key::Value(level as u64));
        let mut anchor =
            RawMenuAnchor::new(controllers[level].clone(), move |_, _| content.clone())
                .child(if level == 2 {
                    Focus::new(button).node(focus.clone()).into()
                } else {
                    button
                })
                .on_close({
                    let closes = closes.clone();
                    move || closes.borrow_mut().push(level)
                });
        if defer {
            let close_requests = close_requests.clone();
            anchor = anchor.on_close_requested(move |_| close_requests.borrow_mut().push(level));
        }
        content = anchor.into();
    }
    let mut runtime = Runtime::new(Center::new(content).into()).unwrap();
    assert!(!frame(&mut runtime));
    for level in 0..3 {
        tap(&mut runtime, Key::Value(level));
        assert_eq!(frame(&mut runtime), level == 2);
    }
    assert!(controllers.iter().all(MenuController::is_open));
    focus.request_focus();
    assert!(frame(&mut runtime));
    assert!(focus.has_primary_focus());

    assert!(action.is_enabled());
    assert!(registered.is_action_enabled(&intent));
    if defer {
        assert_eq!(*close_requests.borrow(), [1, 2]);
    }
    close_requests.borrow_mut().clear();
    assert_eq!(registered.invoke(&intent), ActionResult::Handled);
    assert_eq!(
        *invocations.borrow(),
        [ActionInvocationPhase::Start, ActionInvocationPhase::End]
    );
    assert_eq!(frame(&mut runtime), defer);
    if defer {
        assert_eq!(*close_requests.borrow(), [0, 1, 2]);
        assert!(closes.borrow().is_empty());
    } else {
        assert_eq!(*closes.borrow(), [2, 1, 0]);
        assert!(
            runtime.tree().element_with_key(&Key::Value(2)).is_none(),
            "closed overlay descendants unmount"
        );
        assert!(
            !action.is_enabled(),
            "unmount must detach the descendant controller"
        );
        assert!(!registered.is_action_enabled(&intent));
    }
}

#[test]
fn dismissing_from_the_focused_submenu_closes_all_ancestors() {
    exercise_dismiss_action(false);
}

#[test]
fn dismissing_from_the_focused_submenu_delivers_every_deferred_request() {
    exercise_dismiss_action(true);
}

fn send_key(
    runtime: &mut Runtime,
    logical: incular_core::NamedKey,
    code: incular_core::Code,
) -> bool {
    use incular_core::{KeyboardEvent, KeyboardKey};
    let key = KeyboardKey::Named(logical);
    let handled = runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), code));
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, code));
    handled
}

fn positioned_below(bounds: incular_core::Rect, child: Widget) -> Widget {
    use incular_widgets::{Positioned, Stack};
    Stack::new([Widget::from(
        Positioned::new(child)
            .left(bounds.origin.x)
            .top(bounds.origin.y + bounds.size.height),
    )])
    .into()
}

#[test]
fn escape_from_a_sibling_panel_or_from_a_nested_button_dismisses_the_group() {
    use incular_core::{Code, NamedKey};
    use incular_widgets::{Directionality, RawMenuAnchorGroup, Row, TextDirection};
    let group = MenuController::new();
    let root = MenuController::new();
    let child = MenuController::new();
    let sibling_focus = FocusNode::new();
    let deep_focus = FocusNode::new();
    let popup: Widget = Focus::new(Widget::box_(Size::new(100., 40.), PANEL))
        .node(deep_focus.clone())
        .into();
    let submenu = RawMenuAnchor::new(child.clone(), move |_, info| {
        positioned_below(info.anchor_rect, popup.clone())
    })
    .child(
        Widget::from(
            GestureDetector::new(Widget::box_(Size::new(100., 40.), PANEL)).on_tap({
                let child = child.clone();
                move || child.open(None)
            }),
        )
        .with_key(Key::Value(4)),
    );
    let submenu: Widget = submenu.into();
    let scene = RawMenuAnchorGroup::new(
        group.clone(),
        Row::new([
            Widget::from(
                Focus::new(Widget::box_(Size::new(100., 30.), Color::WHITE))
                    .node(sibling_focus.clone()),
            ),
            RawMenuAnchor::new(root.clone(), move |_, info| {
                positioned_below(info.anchor_rect, submenu.clone())
            })
            .child(
                Widget::from(
                    GestureDetector::new(Widget::box_(Size::new(100., 30.), Color::WHITE)).on_tap(
                        {
                            let root = root.clone();
                            move || root.open(None)
                        },
                    ),
                )
                .with_key(Key::Value(3)),
            )
            .into(),
        ]),
    );
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, Center::new(scene)).into()).unwrap();
    assert!(!frame(&mut runtime));
    root.open(None);
    assert!(frame(&mut runtime));
    sibling_focus.request_focus();
    assert!(frame(&mut runtime));
    assert!(sibling_focus.has_primary_focus());
    assert!(send_key(&mut runtime, NamedKey::Escape, Code::Escape));
    assert!(!frame(&mut runtime));
    assert!(!group.is_open());

    tap(&mut runtime, Key::Value(3));
    assert!(frame(&mut runtime));
    tap(&mut runtime, Key::Value(4));
    assert!(frame(&mut runtime));
    deep_focus.request_focus();
    assert!(frame(&mut runtime));
    assert!(deep_focus.has_primary_focus());
    assert!(group.is_open() && child.is_open());
    assert!(send_key(&mut runtime, NamedKey::Escape, Code::Escape));
    assert!(!frame(&mut runtime));
    assert!(!group.is_open() && !child.is_open());
}

#[test]
fn the_open_menu_gets_one_escape_request_but_closed_and_unrelated_focus_route_outward() {
    use incular_core::{Code, KeyboardKey, Modifiers, NamedKey};
    use incular_widgets::{CallbackShortcuts, Column, SingleActivator};
    let controller = MenuController::new();
    let nested = MenuController::new();
    let anchor_focus = FocusNode::new();
    let outside_focus = FocusNode::new();
    let shortcut_log = Rc::new(RefCell::new(Vec::new()));
    let menu_log = Rc::new(RefCell::new(Vec::new()));
    let nested_menu: Widget = RawMenuAnchor::new(nested.clone(), |_, _| {
        Widget::box_(Size::new(80., 30.), PANEL)
    })
    .child(Widget::box_(Size::new(100., 30.), Color::WHITE))
    .on_close_requested({
        let log = menu_log.clone();
        move |_| log.borrow_mut().push("nested")
    })
    .into();
    let root = RawMenuAnchor::new(controller.clone(), move |_, info| {
        positioned_below(info.anchor_rect, nested_menu.clone())
    })
    .child(Focus::new(Widget::box_(Size::new(100., 30.), Color::WHITE)).node(anchor_focus.clone()))
    .on_close_requested({
        let log = menu_log.clone();
        move |_| log.borrow_mut().push("root")
    });
    let scene = Column::new([
        Widget::from(root),
        Focus::new(Widget::box_(Size::new(100., 30.), Color::WHITE))
            .node(outside_focus.clone())
            .into(),
    ]);
    let mut shortcuts = CallbackShortcuts::new(scene);
    for key in [
        NamedKey::ArrowUp,
        NamedKey::Tab,
        NamedKey::ArrowLeft,
        NamedKey::Escape,
    ] {
        let log = shortcut_log.clone();
        shortcuts = shortcuts.shortcut(
            SingleActivator::new(KeyboardKey::Named(key), Modifiers::empty()),
            move || log.borrow_mut().push(key),
        );
    }
    let mut runtime = Runtime::new(Center::new(shortcuts).into()).unwrap();
    assert!(!frame(&mut runtime));
    anchor_focus.request_focus();
    assert!(!frame(&mut runtime));
    assert!(anchor_focus.has_primary_focus());
    for (key, code) in [
        (NamedKey::ArrowUp, Code::ArrowUp),
        (NamedKey::Tab, Code::Tab),
        (NamedKey::ArrowLeft, Code::ArrowLeft),
        (NamedKey::Escape, Code::Escape),
    ] {
        assert!(send_key(&mut runtime, key, code));
    }
    assert_eq!(
        *shortcut_log.borrow(),
        [
            NamedKey::ArrowUp,
            NamedKey::Tab,
            NamedKey::ArrowLeft,
            NamedKey::Escape
        ]
    );
    shortcut_log.borrow_mut().clear();
    assert!(menu_log.borrow().is_empty());

    controller.open(None);
    frame(&mut runtime);
    nested.open(None);
    frame(&mut runtime);
    assert!(controller.is_open() && nested.is_open());
    menu_log.borrow_mut().clear();
    assert!(send_key(&mut runtime, NamedKey::Escape, Code::Escape));
    assert_eq!(*menu_log.borrow(), ["root", "nested"]);
    assert!(shortcut_log.borrow().is_empty());
    assert!(controller.is_open() && nested.is_open());

    outside_focus.request_focus();
    frame(&mut runtime);
    assert!(outside_focus.has_primary_focus());
    assert!(send_key(&mut runtime, NamedKey::Escape, Code::Escape));
    assert_eq!(*shortcut_log.borrow(), [NamedKey::Escape]);
    assert_eq!(*menu_log.borrow(), ["root", "nested"]);
    assert!(controller.is_open() && nested.is_open());
}

#[test]
fn close_children_preserves_the_focused_submenu_anchor_and_the_parent() {
    let parent = MenuController::new();
    let child = MenuController::new();
    let focus = FocusNode::new();
    let submenu_button = Widget::from(
        GestureDetector::new(Widget::box_(Size::new(100., 30.), Color::WHITE)).on_tap({
            let child = child.clone();
            move || child.open(None)
        }),
    )
    .with_key(Key::Value(8));
    let submenu: Widget = RawMenuAnchor::new(child.clone(), |_, info| {
        positioned_below(info.anchor_rect, Widget::box_(Size::new(100., 30.), PANEL))
    })
    .child(Focus::new(submenu_button).node(focus.clone()))
    .into();
    let menu = RawMenuAnchor::new(parent.clone(), move |_, info| {
        positioned_below(info.anchor_rect, submenu.clone())
    })
    .child(
        Widget::from(
            GestureDetector::new(Widget::box_(Size::new(100., 30.), Color::WHITE)).on_tap({
                let parent = parent.clone();
                move || parent.open(None)
            }),
        )
        .with_key(Key::Value(7)),
    );
    let mut runtime = Runtime::new(Center::new(menu).into()).unwrap();
    assert!(!frame(&mut runtime));
    tap(&mut runtime, Key::Value(7));
    assert!(!frame(&mut runtime));
    tap(&mut runtime, Key::Value(8));
    assert!(frame(&mut runtime));
    focus.request_focus();
    assert!(frame(&mut runtime));
    assert!(focus.has_primary_focus());

    parent.close_children();
    assert!(!frame(&mut runtime));
    assert!(parent.is_open() && !child.is_open());
    assert!(runtime.tree().element_with_key(&Key::Value(8)).is_some());
    assert!(focus.has_primary_focus());
}

#[test]
fn a_groups_state_follows_padded_descendants_and_only_the_children_can_open_it() {
    use incular_config::EdgeInsets;
    use incular_widgets::{Directionality, Padding, RawMenuAnchorGroup, Row, TextDirection};
    let group = MenuController::new();
    let (a, b) = (MenuController::new(), MenuController::new());
    let entry = |controller: &MenuController, button: u64, panel: u64| {
        let anchor = RawMenuAnchor::new(controller.clone(), move |_, info| {
            positioned_below(
                info.anchor_rect,
                Widget::box_(Size::new(60., 40.), PANEL).with_key(Key::Value(panel)),
            )
        })
        .child(
            Widget::from(
                GestureDetector::new(Widget::box_(Size::new(60., 30.), Color::WHITE)).on_tap({
                    let controller = controller.clone();
                    move || {
                        if controller.is_open() {
                            controller.close()
                        } else {
                            controller.open(None)
                        }
                    }
                }),
            )
            .with_key(Key::Value(button)),
        );
        Widget::from(anchor)
    };
    let scene = RawMenuAnchorGroup::new(
        group.clone(),
        Row::new([
            entry(&a, 1, 11),
            Padding::new(EdgeInsets::all(8.), entry(&b, 2, 12)).into(),
        ]),
    );
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, Center::new(scene)).into()).unwrap();
    let shows =
        |runtime: &Runtime, key: u64| runtime.tree().element_with_key(&Key::Value(key)).is_some();
    assert!(!frame(&mut runtime));
    assert!(!group.is_open() && shows(&runtime, 1) && shows(&runtime, 2));

    group.open(None);
    assert!(!frame(&mut runtime));
    assert!(!group.is_open() && !a.is_open() && !b.is_open());
    assert!(shows(&runtime, 1) && shows(&runtime, 2));

    tap(&mut runtime, Key::Value(1));
    assert!(frame(&mut runtime));
    assert!(group.is_open() && shows(&runtime, 11) && !shows(&runtime, 12));
    tap(&mut runtime, Key::Value(2));
    assert!(frame(&mut runtime));
    assert!(group.is_open() && !shows(&runtime, 11) && shows(&runtime, 12));
    tap(&mut runtime, Key::Value(2));
    assert!(!frame(&mut runtime));
    assert!(!group.is_open() && !shows(&runtime, 12));

    a.open(None);
    assert!(frame(&mut runtime));
    assert!(group.is_open() && a.is_open() && shows(&runtime, 11));
    group.close();
    assert!(!frame(&mut runtime));
    assert!(!group.is_open() && !a.is_open() && shows(&runtime, 1));
    b.open(None);
    assert!(frame(&mut runtime));
    assert!(group.is_open() && b.is_open() && shows(&runtime, 12));
    group.close_children();
    assert!(!frame(&mut runtime));
    assert!(!group.is_open() && !b.is_open() && shows(&runtime, 2));
}

fn toggle(label: u64, controller: &MenuController) -> Widget {
    let controller = controller.clone();
    Widget::from(
        GestureDetector::new(Widget::box_(Size::new(100., 30.), Color::WHITE)).on_tap(move || {
            if controller.is_open() {
                controller.close()
            } else {
                controller.open(None)
            }
        }),
    )
    .with_key(Key::Value(label))
}

fn tagged_panel(label: u64) -> Widget {
    Widget::box_(Size::new(70., 30.), PANEL).with_key(Key::Value(label))
}

fn alive(runtime: &Runtime, key: u64) -> bool {
    runtime.tree().element_with_key(&Key::Value(key)).is_some()
}

#[test]
fn toggling_a_menu_by_its_anchor_tracks_actual_overlay_visibility() {
    let controller = MenuController::new();
    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let menu = RawMenuAnchor::new(controller.clone(), |_, info| {
        positioned_below(info.anchor_rect, tagged_panel(11))
    })
    .child(toggle(1, &controller))
    .on_open({
        let callbacks = callbacks.clone();
        move || callbacks.borrow_mut().push("open")
    })
    .on_close({
        let callbacks = callbacks.clone();
        move || callbacks.borrow_mut().push("close")
    });
    let mut runtime = Runtime::new(Center::new(menu).into()).unwrap();
    assert!(!frame(&mut runtime));
    tap(&mut runtime, Key::Value(1));
    assert!(frame(&mut runtime));
    assert!(controller.is_open() && alive(&runtime, 11));
    assert_eq!(*callbacks.borrow(), ["open"]);
    tap(&mut runtime, Key::Value(1));
    assert!(!frame(&mut runtime));
    assert!(!controller.is_open() && !alive(&runtime, 11));
    assert_eq!(*callbacks.borrow(), ["open", "close"]);
    controller.open(None);
    assert!(frame(&mut runtime));
    assert_eq!(*callbacks.borrow(), ["open", "close", "open"]);
    controller.close();
    assert!(!frame(&mut runtime));
    assert_eq!(*callbacks.borrow(), ["open", "close", "open", "close"]);
}

#[test]
fn direct_close_hides_all_descendants_but_a_nested_close_keeps_the_parent() {
    use incular_widgets::Column;
    let parent = MenuController::new();
    let child = MenuController::new();
    let nested: Widget = RawMenuAnchor::new(child.clone(), |_, info| {
        positioned_below(info.anchor_rect, tagged_panel(22))
    })
    .child(toggle(2, &child))
    .into();
    let content: Widget = Column::new([tagged_panel(11), nested])
        .main_axis_size(incular_widgets::MainAxisSize::Min)
        .into();
    let menu = RawMenuAnchor::new(parent.clone(), move |_, info| {
        positioned_below(info.anchor_rect, content.clone())
    })
    .child(toggle(1, &parent));
    let mut runtime = Runtime::new(Center::new(menu).into()).unwrap();
    assert!(!frame(&mut runtime));
    assert!(alive(&runtime, 1));
    assert!(!parent.is_open() && !alive(&runtime, 11));

    parent.open(None);
    frame(&mut runtime);
    assert!(parent.is_open() && alive(&runtime, 11));
    assert!(!child.is_open() && !alive(&runtime, 22));
    child.open(None);
    frame(&mut runtime);
    assert!(parent.is_open() && child.is_open() && alive(&runtime, 11) && alive(&runtime, 22));
    parent.close();
    frame(&mut runtime);
    assert!(!parent.is_open() && !child.is_open() && !alive(&runtime, 11) && !alive(&runtime, 22));

    parent.open(None);
    frame(&mut runtime);
    child.open(None);
    frame(&mut runtime);
    assert!(parent.is_open() && child.is_open() && alive(&runtime, 11) && alive(&runtime, 22));
    child.close();
    frame(&mut runtime);
    assert!(parent.is_open() && !child.is_open() && alive(&runtime, 11) && !alive(&runtime, 22));
}

#[test]
fn tapping_one_child_anchor_then_another_switches_the_visible_submenu() {
    use incular_widgets::{Directionality, RawMenuAnchorGroup, Row, TextDirection};
    for grouped in [false, true] {
        let parent = MenuController::new();
        let (a, b) = (MenuController::new(), MenuController::new());
        let make = |controller: &MenuController, key, panel| {
            Widget::from(
                RawMenuAnchor::new(controller.clone(), move |_, info| {
                    positioned_below(info.anchor_rect, tagged_panel(panel))
                })
                .child(toggle(key, controller)),
            )
        };
        let branch: Widget = Row::new([make(&a, 2, 12), make(&b, 3, 13)])
            .main_axis_size(incular_widgets::MainAxisSize::Min)
            .into();
        let scene = if grouped {
            Widget::from(RawMenuAnchorGroup::new(parent.clone(), branch))
        } else {
            Widget::from(
                RawMenuAnchor::new(parent.clone(), move |_, info| {
                    positioned_below(info.anchor_rect, branch.clone())
                })
                .child(toggle(1, &parent)),
            )
        };
        let mut runtime =
            Runtime::new(Directionality::new(TextDirection::Ltr, Center::new(scene)).into())
                .unwrap();
        frame(&mut runtime);
        if !grouped {
            tap(&mut runtime, Key::Value(1));
            frame(&mut runtime);
        }
        assert!(alive(&runtime, 2) && alive(&runtime, 3));
        assert!(!alive(&runtime, 12) && !alive(&runtime, 13));
        for (button, panel) in [(2, 12), (3, 13)] {
            tap(&mut runtime, Key::Value(button));
            frame(&mut runtime);
            assert!(parent.is_open() && alive(&runtime, panel));
            assert!(!alive(&runtime, if panel == 12 { 13 } else { 12 }));
        }
    }
}

#[test]
fn a_context_menu_button_can_itself_contain_another_context_menu() {
    use incular_widgets::Column;
    let (first, second) = (MenuController::new(), MenuController::new());
    let second_menu: Widget = RawMenuAnchor::new(second.clone(), |_, info| {
        positioned_below(info.anchor_rect, tagged_panel(12))
    })
    .child(toggle(2, &second))
    .into();
    let first_menu = RawMenuAnchor::new(first.clone(), |_, info| {
        // Offset the first menu so both anchor buttons stay reachable.
        let mut rect = info.anchor_rect;
        rect.origin = rect.origin + Offset::new(120., 0.);
        positioned_below(rect, tagged_panel(11))
    })
    .builder({
        let first = first.clone();
        move |_, discovered, _| {
            assert!(discovered == &first);
            Column::new([toggle(1, discovered), second_menu.clone()])
                .main_axis_size(incular_widgets::MainAxisSize::Min)
                .into()
        }
    });
    let mut runtime = Runtime::new(Center::new(first_menu).into()).unwrap();
    frame(&mut runtime);
    tap(&mut runtime, Key::Value(1));
    frame(&mut runtime);
    assert!(alive(&runtime, 11));
    tap(&mut runtime, Key::Value(2));
    frame(&mut runtime);
    assert!(second.is_open() && alive(&runtime, 12));
}

fn mounted_group(
    group: &MenuController,
    menu: &MenuController,
    group_key: u64,
    menu_key: u64,
) -> Widget {
    use incular_widgets::RawMenuAnchorGroup;
    let anchor = Widget::from(
        RawMenuAnchor::new(menu.clone(), |_, info| {
            positioned_below(info.anchor_rect, tagged_panel(11))
        })
        .child(toggle(1, menu)),
    )
    .with_key(Key::Value(menu_key));
    Widget::from(RawMenuAnchorGroup::new(group.clone(), anchor)).with_key(Key::Value(group_key))
}

#[test]
fn replacing_group_and_anchor_controllers_does_not_hide_the_open_panel() {
    let (old_group, old_menu) = (MenuController::new(), MenuController::new());
    let (group, menu) = (MenuController::new(), MenuController::new());
    let mut runtime =
        Runtime::new(Center::new(mounted_group(&old_group, &old_menu, 100, 101)).into()).unwrap();
    frame(&mut runtime);
    tap(&mut runtime, Key::Value(1));
    assert!(frame(&mut runtime));
    assert!(old_group.is_open() && old_menu.is_open());
    assert!(!group.is_open() && !menu.is_open());

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(
            root,
            Center::new(mounted_group(&group, &menu, 100, 101)).into(),
        )
        .unwrap();
    assert!(frame(&mut runtime));
    assert!(!old_group.is_open() && !old_menu.is_open());
    assert!(group.is_open() && menu.is_open());
    group.close();
    assert!(!frame(&mut runtime));
    assert!(!group.is_open() && !menu.is_open());
}

#[test]
fn a_controller_can_move_to_a_newly_keyed_group_or_menu_without_keeping_old_state() {
    for replace_group in [false, true] {
        let (group, menu) = (MenuController::new(), MenuController::new());
        let old_widget = Center::new(mounted_group(&group, &menu, 100, 101));
        let mut runtime = Runtime::new(old_widget.into()).unwrap();
        frame(&mut runtime);
        tap(&mut runtime, Key::Value(1));
        assert!(frame(&mut runtime));
        assert!(group.is_open() && menu.is_open());

        let (group_key, menu_key) = if replace_group {
            (102, 101)
        } else {
            (100, 102)
        };
        let root = runtime.tree().root().unwrap();
        runtime
            .schedule_update(
                root,
                Center::new(mounted_group(&group, &menu, group_key, menu_key)).into(),
            )
            .unwrap();
        assert!(!frame(&mut runtime));
        assert!(!group.is_open() && !menu.is_open());
        tap(&mut runtime, Key::Value(1));
        assert!(frame(&mut runtime));
        assert!(group.is_open() && menu.is_open());
        if replace_group {
            group.close()
        } else {
            menu.close()
        }
        assert!(!frame(&mut runtime));
        assert!(!group.is_open() && !menu.is_open());
    }
}
