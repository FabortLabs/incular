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
