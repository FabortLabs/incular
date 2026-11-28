//! Observable Flutter action masks, not keyboard focus or OS-specific speech.
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{CustomSemanticsAction, SemanticAction, SemanticActionKind, SemanticRole};
use incular_widgets::{
    AbsorbPointer, Center, Focus, FocusNode, IgnorePointer, Semantics, SizedBox, Visibility, Widget,
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug)]
enum Wrapper {
    Absorb,
    Ignore,
    Visibility,
}

fn check_blocker(kind: Wrapper) {
    const LABEL: &str = "Action node";
    let focus = FocusNode::new();
    let calls = Rc::new(Cell::new(0));
    let custom = CustomSemanticsAction::new("custom action");
    let leaf = Widget::from(
        Semantics::new(
            Focus::new(SizedBox::new().width(60.).height(40.))
                .node(focus.clone())
                .include_semantics(false),
        )
        .container(true)
        .label(LABEL)
        .button(true)
        .enabled(true)
        .on_tap(|| {})
        .on_long_press(|| {})
        .on_increase({
            let calls = Rc::clone(&calls);
            move || calls.set(calls.get() + 1)
        })
        .on_decrease(|| {})
        .on_scroll_left(|| {})
        .custom_action(custom.clone(), || {}),
    );
    let build = |blocked| {
        let child = match kind {
            Wrapper::Absorb => Widget::from(AbsorbPointer::new(leaf.clone()).absorbing(blocked)),
            Wrapper::Ignore => Widget::from(IgnorePointer::new(leaf.clone()).ignoring(blocked)),
            Wrapper::Visibility => Widget::from(
                Visibility::maintain(leaf.clone())
                    .maintain_interactivity(false)
                    .visible(!blocked),
            ),
        };
        Widget::from(Center::new(child))
    };

    let mut runtime = Runtime::new(build(false)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    focus.request_focus();
    runtime.run_frame(constraints).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut retained = None;
    let mut actual = Vec::new();
    for blocked in [false, true, false] {
        runtime.schedule_update(root, build(blocked)).unwrap();
        runtime.run_frame(constraints).unwrap();
        assert!(focus.has_primary_focus());
        let (node, mask, custom_advertised) = {
            let (id, node) = runtime
                .tree()
                .semantics()
                .iter()
                .find(|(_, node)| node.label.as_deref() == Some(LABEL))
                .expect("still published");
            assert_eq!(id, *retained.get_or_insert(id));
            assert_eq!(node.role, SemanticRole::Button);
            assert!(node.state.enabled && node.state.focusable);
            let mask = [
                SemanticActionKind::Focus,
                SemanticActionKind::Activate,
                SemanticActionKind::LongPress,
                SemanticActionKind::Increment,
                SemanticActionKind::Decrement,
                SemanticActionKind::ScrollLeft,
                SemanticActionKind::Custom,
            ]
            .map(|action| node.actions.contains(&action));
            (
                id,
                mask,
                node.state.custom_actions.contains(&custom.get_identifier()),
            )
        };
        let dispatched = runtime.dispatch_semantic_action(node, SemanticAction::Increment);
        actual.push((mask, custom_advertised, dispatched));
    }
    assert_eq!(
        actual,
        [
            ([true; 7], true, true),
            ([false; 7], false, false),
            ([true; 7], true, true)
        ],
        "{kind:?}"
    );
    assert_eq!(calls.get(), 2, "a blocked native action must not execute");
}

#[test]
fn absorbing_blocks_all_published_user_actions() {
    check_blocker(Wrapper::Absorb);
}
#[test]
fn ignoring_blocks_all_published_user_actions() {
    check_blocker(Wrapper::Ignore);
}
#[test]
fn maintaining_semantics_but_not_interactivity_blocks_user_actions() {
    check_blocker(Wrapper::Visibility);
}

#[test]
fn merged_node_keeps_labels_but_cannot_borrow_a_blocked_custom_action() {
    use incular_widgets::{Column, MergeSemantics};
    let action = CustomSemanticsAction::new("conditional");
    let id = action.get_identifier();
    let hits = Rc::new(Cell::new(0));
    let target = Widget::from(
        Semantics::new(SizedBox::new().width(60.).height(40.))
            .label("Blocked target")
            .custom_action(action, {
                let hits = Rc::clone(&hits);
                move || hits.set(hits.get() + 1)
            }),
    );
    let build = |blocked: bool| {
        Widget::from(Center::new(MergeSemantics::new(Column::new([
            Widget::from(IgnorePointer::new(target.clone()).ignoring(blocked)),
            Semantics::new(SizedBox::new().height(10.))
                .label("Plain sibling")
                .into(),
        ]))))
    };
    let constraints = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(build(false)).unwrap();
    runtime.run_frame(constraints).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut retained = None;
    for blocked in [false, true, false] {
        runtime.schedule_update(root, build(blocked)).unwrap();
        runtime.run_frame(constraints).unwrap();
        let (node_id, node) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| {
                node.label.as_deref().is_some_and(|label| {
                    label.contains("Blocked target") && label.contains("Plain sibling")
                })
            })
            .expect("retained merged node");
        assert_eq!(node_id, *retained.get_or_insert(node_id));
        assert_eq!(node.actions.contains(&SemanticActionKind::Custom), !blocked);
        assert_eq!(node.state.custom_actions.contains(&id), !blocked);
        assert_eq!(
            runtime.dispatch_semantic_action(node_id, SemanticAction::Custom(id)),
            !blocked
        );
    }
    assert_eq!(hits.get(), 2);
}

#[test]
fn ignore_pointer_toggles_tap_on_three_retained_nested_nodes() {
    let view = |ignoring| {
        let mut subtree: Widget = SizedBox::square(10.).into();
        for label in ["3", "2", "1"] {
            subtree = Semantics::new(subtree)
                .label(label)
                .on_tap(|| {})
                .container(true)
                .into();
        }
        IgnorePointer::new(subtree).ignoring(ignoring).into()
    };
    let mut runtime = Runtime::new(view(true)).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut ids = None;
    for ignoring in [true, false, true, false] {
        runtime.schedule_update(root, view(ignoring)).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let tree = runtime.tree().semantics();
        let actual = ["1", "2", "3"].map(|label| {
            let (id, node) = tree
                .iter()
                .find(|(_, node)| node.label.as_deref() == Some(label))
                .unwrap();
            assert_eq!(
                node.actions.contains(&SemanticActionKind::Activate),
                !ignoring
            );
            id
        });
        assert_eq!(actual, *ids.get_or_insert(actual));
        assert_eq!(tree.node(actual[0]).unwrap().children, [actual[1]]);
        assert_eq!(tree.node(actual[1]).unwrap().children, [actual[2]]);
    }
}

#[test]
fn ignore_pointer_changes_button_actions_without_changing_focusability_or_enabled_state() {
    use incular_widgets::{Directionality, GestureDetector, Text, TextDirection};

    let focus = FocusNode::new();
    let pressed = Rc::new(Cell::new(0));
    let on_tap = {
        let pressed = pressed.clone();
        move || pressed.set(pressed.get() + 1)
    };
    let leaf = Widget::from(
        Semantics::new(
            Focus::new(GestureDetector::new(Text::new("button")).on_tap(|| {}))
                .node(focus)
                .include_semantics(false),
        )
        .label("button")
        .button(true)
        .focusable(true)
        .enabled(true)
        .on_tap(on_tap),
    );
    for ignoring in [false, true] {
        let wrapper = IgnorePointer::new(leaf.clone());
        let wrapper = if ignoring {
            wrapper
        } else {
            wrapper.ignoring(false)
        };
        let scene = Directionality::new(TextDirection::Ltr, wrapper);
        let mut runtime = Runtime::new(scene.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let (id, node) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.role == SemanticRole::Button)
            .unwrap();
        assert_eq!(node.label.as_deref(), Some("button"));
        assert!(node.state.focusable && node.state.enabled && node.state.has_enabled_state);
        assert_eq!(
            node.actions.contains(&SemanticActionKind::Activate),
            !ignoring
        );
        assert_eq!(node.actions.contains(&SemanticActionKind::Focus), !ignoring);
        let before = pressed.get();
        assert_eq!(
            runtime.dispatch_semantic_action(id, SemanticAction::Activate),
            !ignoring
        );
        assert_eq!(pressed.get(), before + u32::from(!ignoring));
    }
}

fn check_mouse_passthrough(absorb: bool) {
    use incular_core::{
        InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
        PointerSampleMetadata,
    };
    use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
    use incular_widgets::{
        Align, Alignment, Directionality, Listener, MouseCursor, MouseRegion, Stack, TextDirection,
    };
    use std::cell::RefCell;

    let log = Rc::new(RefCell::new(Vec::new()));
    let region = |name, cursor, child: Widget| {
        let (enter, exit, down) = (log.clone(), log.clone(), log.clone());
        Listener::new(
            MouseRegion::new(child)
                .cursor(cursor)
                .on_enter(move |_| enter.borrow_mut().push(("enter", name)))
                .on_exit(move |_| exit.borrow_mut().push(("exit", name))),
        )
        .on_pointer_down(move |_| down.borrow_mut().push(("down", name)))
    };
    let build = |blocked| {
        let below = Widget::from(region(
            "under",
            MouseCursor::Click,
            SizedBox::expand().into(),
        ));
        let above = region("top", MouseCursor::Text, SizedBox::expand().into());
        let above = if absorb {
            Widget::from(AbsorbPointer::new(above).absorbing(blocked))
        } else {
            Widget::from(IgnorePointer::new(above).ignoring(blocked))
        };
        let stack = Widget::from(Stack::new([below, above]));
        Align::new(
            Alignment::TOP_LEFT,
            Directionality::new(
                TextDirection::Ltr,
                SizedBox::square(100.).child(region("outer", MouseCursor::NotAllowed, stack)),
            ),
        )
        .into()
    };

    let viewport = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(build(true)).unwrap();
    runtime.run_frame(viewport).unwrap();
    let _ = runtime.handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
        pointer: 1,
        device: 1,
        kind: PointerDeviceKind::Mouse,
        position: Offset::new(200., 200.),
        change: HoverDeviceChange::Added,
    }));
    assert!(log.take().is_empty());
    let pointer = |runtime: &mut Runtime, phase| {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind: PointerDeviceKind::Mouse,
            position: Offset::new(50., 50.),
            phase,
            buttons: if phase == PointerPhase::Down {
                PRIMARY_POINTER_BUTTON
            } else {
                0
            },
            button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
                .then_some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
        });
    };
    pointer(&mut runtime, PointerPhase::Move);
    assert_eq!(
        log.take(),
        if absorb {
            vec![("enter", "outer")]
        } else {
            vec![("enter", "outer"), ("enter", "under")]
        }
    );
    pointer(&mut runtime, PointerPhase::Down);
    assert_eq!(
        log.take(),
        if absorb {
            vec![("down", "outer")]
        } else {
            vec![("down", "under"), ("down", "outer")]
        }
    );
    pointer(&mut runtime, PointerPhase::Up);
    assert!(log.take().is_empty());

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(false)).unwrap();
    runtime.run_frame(viewport).unwrap();
    assert_eq!(
        log.take(),
        if absorb {
            vec![("enter", "top")]
        } else {
            vec![("exit", "under"), ("enter", "top")]
        }
    );
    pointer(&mut runtime, PointerPhase::Down);
    assert_eq!(log.take(), [("down", "top"), ("down", "outer")]);
    pointer(&mut runtime, PointerPhase::Up);
    assert!(log.take().is_empty());

    runtime.schedule_update(root, build(true)).unwrap();
    runtime.run_frame(viewport).unwrap();
    assert_eq!(
        log.take(),
        if absorb {
            vec![("exit", "top")]
        } else {
            vec![("exit", "top"), ("enter", "under")]
        }
    );
}

#[test]
fn ignoring_pointer_passes_clicks_and_hover_to_the_sibling_below() {
    check_mouse_passthrough(false);
}

#[test]
fn absorbing_pointer_blocks_the_sibling_below_but_not_its_ancestor() {
    check_mouse_passthrough(true);
}
