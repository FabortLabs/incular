use incular_config::{Axis, Constraints, TextDirection};
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_platform::PlatformEvent;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, Expanded, Focus, FocusNode, FocusScope, GlobalKey, ListView, Row,
    ScrollController, SizedBox, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn event(runtime: &mut Runtime, key: NamedKey, code: Code, modifiers: Modifiers, down: bool) {
    let key = KeyboardKey::Named(key);
    let mut event = if down {
        KeyboardEvent::key_down(key, code)
    } else {
        KeyboardEvent::key_up(key, code)
    };
    event.modifiers = modifiers;
    runtime.handle_platform_event(PlatformEvent::Input(InputEvent::Key(event)));
}

fn press(runtime: &mut Runtime, key: NamedKey, code: Code, modifiers: Modifiers) {
    event(runtime, key.clone(), code, modifiers, true);
    event(runtime, key, code, modifiers, false);
    frame(runtime);
}

fn tab(runtime: &mut Runtime, backwards: bool) {
    if backwards {
        event(
            runtime,
            NamedKey::Shift,
            Code::ShiftLeft,
            Modifiers::SHIFT,
            true,
        );
    }
    press(
        runtime,
        NamedKey::Tab,
        Code::Tab,
        if backwards {
            Modifiers::SHIFT
        } else {
            Modifiers::empty()
        },
    );
    if backwards {
        event(
            runtime,
            NamedKey::Shift,
            Code::ShiftLeft,
            Modifiers::empty(),
            false,
        );
        frame(runtime);
    }
}

fn owned_cell(key: GlobalKey, focus: bool, skip: bool) -> Focus {
    Focus::new(Widget::from(SizedBox::square(100.)).with_key(key))
        .autofocus(focus)
        .skip_traversal(skip)
}

fn node(runtime: &Runtime, key: GlobalKey) -> FocusNode {
    let tree = runtime.tree();
    let element = tree.element_with_key(&key.into()).unwrap();
    Focus::of(tree, element, false)
}

#[test]
fn native_tab_shift_tab_and_arrows_traverse_all_four_scoped_controls() {
    let keys: [GlobalKey; 4] = std::array::from_fn(|_| GlobalKey::new());
    let scene = FocusScope::new(Column::new([
        Row::new([
            owned_cell(keys[0], true, false),
            owned_cell(keys[1], false, false),
        ]),
        Row::new([
            owned_cell(keys[2], false, false),
            owned_cell(keys[3], false, false),
        ]),
    ]));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);
    let nodes = keys.map(|key| node(&runtime, key));
    assert!(nodes[0].has_primary_focus());
    for (reverse, order) in [(false, [1, 2, 3, 0]), (true, [3, 2, 1, 0])] {
        for stop in order {
            tab(&mut runtime, reverse);
            assert!(nodes[stop].has_primary_focus(), "Tab target {stop}");
        }
    }
    for (key, code, stop) in [
        (NamedKey::ArrowRight, Code::ArrowRight, 1),
        (NamedKey::ArrowDown, Code::ArrowDown, 3),
        (NamedKey::ArrowLeft, Code::ArrowLeft, 2),
        (NamedKey::ArrowUp, Code::ArrowUp, 0),
    ] {
        press(&mut runtime, key, code, Modifiers::empty());
        assert!(nodes[stop].has_primary_focus(), "spatial target {stop}");
    }
}

#[test]
fn autofocus_can_start_on_an_excluded_node_without_returning_to_it_on_tab() {
    let keys: [GlobalKey; 3] = std::array::from_fn(|_| GlobalKey::new());
    let scene = FocusScope::new(Column::new([Row::new([
        owned_cell(keys[0], true, true),
        owned_cell(keys[1], false, false),
        owned_cell(keys[2], false, false),
    ])]));
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);
    let nodes = keys.map(|key| node(&runtime, key));
    assert!(nodes[0].has_primary_focus());
    for stop in [1, 2, 1, 2] {
        tab(&mut runtime, false);
        assert!(nodes[stop].has_primary_focus());
    }
}

fn reveal_list(axis: Axis) {
    let first = FocusNode::new();
    let last = FocusNode::new();
    let items: [FocusNode; 11] = std::array::from_fn(|_| FocusNode::new());
    let controller = ScrollController::new();
    let block = |node: &FocusNode| -> Widget {
        let size = match axis {
            Axis::Vertical => Size::new(800., 100.),
            Axis::Horizontal => Size::new(100., 600.),
        };
        Focus::new(SizedBox::from_size(size))
            .node(node.clone())
            .into()
    };
    let list = ListView::new(items.iter().map(block))
        .scroll_direction(axis)
        .controller(controller.clone());
    let children = [block(&first), Expanded::new(list).into(), block(&last)];
    let root: Widget = match axis {
        Axis::Vertical => Column::new(children).into(),
        Axis::Horizontal => Row::new(children).into(),
    };
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, root).into()).unwrap();
    frame(&mut runtime);
    let (forward, reverse, visible) = match axis {
        Axis::Vertical => (
            (NamedKey::ArrowDown, Code::ArrowDown),
            (NamedKey::ArrowUp, Code::ArrowUp),
            4,
        ),
        Axis::Horizontal => (
            (NamedKey::ArrowRight, Code::ArrowRight),
            (NamedKey::ArrowLeft, Code::ArrowLeft),
            6,
        ),
    };
    let mut send = |reverse_direction: bool, target: &FocusNode, offset: f32, description: &str| {
        let (key, code) = if reverse_direction {
            &reverse
        } else {
            &forward
        };
        press(&mut runtime, key.clone(), *code, Modifiers::empty());
        assert!(
            target.has_primary_focus(),
            "{description}: target missed, offset {}",
            controller.offset()
        );
        assert_eq!(controller.offset(), offset, "{description}");
    };
    assert_eq!(controller.offset(), 0.);
    send(false, &first, 0., "enter at leading outer control");
    for (index, item) in items.iter().enumerate() {
        let offset = (index + 1).saturating_sub(visible) as f32 * 100.;
        send(false, item, offset, &format!("forward item {index}"));
    }
    let extent = (items.len() - visible) as f32 * 100.;
    send(false, &last, extent, "trailing outer control");
    send(
        true,
        items.last().unwrap(),
        extent,
        "return to last list item",
    );
    for (index, item) in items.iter().enumerate().rev().skip(1) {
        send(
            true,
            item,
            extent.min(index as f32 * 100.),
            &format!("reverse item {index}"),
        );
    }
    send(true, &first, 0., "return to leading outer control");
}

#[test]
fn vertical_arrows_reveal_each_lazy_item_and_the_outer_controls() {
    reveal_list(Axis::Vertical);
}

#[test]
fn horizontal_arrows_reveal_each_lazy_item_and_the_outer_controls() {
    reveal_list(Axis::Horizontal);
}

fn reveal_nested_lists(axis: Axis) {
    let cross_axis = match axis {
        Axis::Vertical => Axis::Horizontal,
        Axis::Horizontal => Axis::Vertical,
    };
    let cross_count = if axis == Axis::Vertical { 5 } else { 10 };
    let sticky = FocusNode::new();
    let outer = ScrollController::new();
    let inner: Vec<_> = (0..10).map(|_| ScrollController::new()).collect();
    let nodes: Vec<Vec<FocusNode>> = (0..10)
        .map(|_| (0..cross_count).map(|_| FocusNode::new()).collect())
        .collect();
    let band = move |length| match axis {
        Axis::Vertical => SizedBox::new().height(length),
        Axis::Horizontal => SizedBox::new().width(length),
    };
    let outer_list = ListView::separated(
        nodes.len(),
        {
            let nodes = nodes.clone();
            let inner = inner.clone();
            move |index| {
                let row = nodes[index].clone();
                band(100.).child(
                    ListView::builder(cross_count, move |slot| {
                        Focus::new(SizedBox::square(100.)).node(row[slot].clone())
                    })
                    .scroll_direction(cross_axis)
                    .controller(inner[index].clone()),
                )
            }
        },
        move |_| band(32.),
    )
    .scroll_direction(axis)
    .controller(outer.clone());
    let sticky_size = match axis {
        Axis::Vertical => Size::new(800., 100.),
        Axis::Horizontal => Size::new(100., 600.),
    };
    let sticky_widget = Focus::new(SizedBox::from_size(sticky_size)).node(sticky.clone());
    let children: [Widget; 2] = [sticky_widget.into(), Expanded::new(outer_list).into()];
    let scene: Widget = match axis {
        Axis::Vertical => Column::new(children).into(),
        Axis::Horizontal => Row::new(children).into(),
    };
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap();
    frame(&mut runtime);
    sticky.request_focus();
    frame(&mut runtime);
    assert!(sticky.has_primary_focus());
    let (forward, reverse, scroll_at) = match axis {
        Axis::Vertical => (
            (NamedKey::ArrowDown, Code::ArrowDown),
            (NamedKey::ArrowUp, Code::ArrowUp),
            5,
        ),
        Axis::Horizontal => (
            (NamedKey::ArrowRight, Code::ArrowRight),
            (NamedKey::ArrowLeft, Code::ArrowLeft),
            6,
        ),
    };
    press(
        &mut runtime,
        forward.0.clone(),
        forward.1,
        Modifiers::empty(),
    );
    let cross_slot = nodes[0]
        .iter()
        .position(FocusNode::has_primary_focus)
        .expect("first arrow enters first band");
    for (index, strip) in nodes.iter().enumerate().skip(1) {
        press(
            &mut runtime,
            forward.0.clone(),
            forward.1,
            Modifiers::empty(),
        );
        assert!(
            strip[cross_slot].has_primary_focus(),
            "{axis:?} forward band {index}, offset {}",
            outer.offset()
        );
        if index >= scroll_at {
            assert!(outer.offset() > 0., "{axis:?} must reveal band {index}");
        }
    }
    for (index, strip) in nodes.iter().enumerate().rev().skip(1) {
        press(
            &mut runtime,
            reverse.0.clone(),
            reverse.1,
            Modifiers::empty(),
        );
        assert!(
            strip[cross_slot].has_primary_focus(),
            "{axis:?} reverse band {index}, offset {}",
            outer.offset()
        );
    }
    press(&mut runtime, reverse.0, reverse.1, Modifiers::empty());
    assert!(sticky.has_primary_focus());
}

#[test]
fn vertical_arrows_keep_the_same_column_across_lazy_horizontal_lists() {
    reveal_nested_lists(Axis::Vertical);
}

#[test]
fn horizontal_arrows_keep_the_same_row_across_lazy_vertical_lists() {
    reveal_nested_lists(Axis::Horizontal);
}

#[test]
fn arrow_events_reach_hardware_handlers_without_focusable_widgets() {
    use std::{cell::RefCell, rc::Rc};

    use incular_core::KeyState;
    use incular_widgets::{Container, Placeholder};

    // The two original registrations use different non-focusable children.
    for child in [Widget::from(Container::new()), Placeholder::new().into()] {
        let mut runtime = Runtime::new(FocusScope::new(child).into()).unwrap();
        frame(&mut runtime);
        let before = runtime.focused_element();
        let keys = Rc::new(RefCell::new(Vec::new()));
        let _handler = runtime.add_hardware_key_handler({
            let keys = keys.clone();
            move |key| {
                keys.borrow_mut().push((key.code, key.state));
                true
            }
        });
        press(
            &mut runtime,
            NamedKey::ArrowRight,
            Code::ArrowRight,
            Modifiers::empty(),
        );
        assert_eq!(
            *keys.borrow(),
            [
                (Code::ArrowRight, KeyState::Down),
                (Code::ArrowRight, KeyState::Up)
            ]
        );
        assert_eq!(runtime.focused_element(), before);
    }
}

#[test]
fn tab_leaves_primary_focus_unchanged_when_the_group_has_no_traversal_stop() {
    use incular_config::Alignment;
    use incular_widgets::{Align, FocusTraversalGroup, Text};

    let empty = SizedBox::expand().child(Align::new(Alignment::CENTER, Text::new("title")));
    let empty = Directionality::new(TextDirection::Ltr, empty);
    let group = FocusTraversalGroup::new(empty);
    let mut runtime = Runtime::new(FocusScope::new(group).into()).unwrap();
    frame(&mut runtime);
    let before = runtime.focused_element();
    assert!(runtime.tree().focusable_elements().is_empty());
    tab(&mut runtime, false);
    assert_eq!(runtime.focused_element(), before);
}
