use std::{cell::RefCell, rc::Rc};

use incular_accessibility::MobileAccessibilityProjection;
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind, SemanticNode};
use incular_widgets::Semantics;

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn only_node(runtime: &Runtime) -> &SemanticNode {
    let mut nodes = runtime.tree().semantics().iter();
    let (_, node) = nodes.next().expect("the ordinary annotation is exposed");
    assert!(nodes.next().is_none());
    node
}

#[test]
fn callback_replacement_updates_dispatch_without_publishing_a_semantic_change() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let build = |name, long| {
        let tap_trace = events.clone();
        let sem = Semantics::default()
            .container(true)
            .on_tap(move || tap_trace.borrow_mut().push(name));
        if long {
            let trace = events.clone();
            sem.on_long_press(move || trace.borrow_mut().push("longPress"))
        } else {
            sem
        }
    };

    let mut runtime = Runtime::new(build("first", false).into()).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let initial = only_node(&runtime).clone();
    let id = initial.id;
    assert_eq!(initial.actions, [SemanticActionKind::Activate]);
    assert!(initial.label.is_none());
    let mut native = MobileAccessibilityProjection::new();
    assert!(native.sync(runtime.tree().semantics()).is_some());
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));

    for (long, changed, actions) in [
        (false, false, &[SemanticActionKind::Activate][..]),
        (
            true,
            true,
            &[SemanticActionKind::Activate, SemanticActionKind::LongPress][..],
        ),
        (false, true, &[SemanticActionKind::Activate][..]),
    ] {
        let previous = runtime.tree().semantics().revision();
        runtime
            .schedule_update(root, build("second", long).into())
            .unwrap();
        frame(&mut runtime);
        let node = only_node(&runtime);
        assert_eq!(node.id, id);
        assert_eq!(node.actions, actions);
        assert_eq!(node.bounds, initial.bounds);
        assert_eq!(native.sync(runtime.tree().semantics()).is_some(), changed);
        assert!(native.sync(runtime.tree().semantics()).is_none());
        assert_eq!(runtime.tree().semantics().revision() != previous, changed);
        if long {
            assert!(runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
        } else if !changed {
            assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));
        } else {
            assert!(!runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
        }
    }
    assert_eq!(*events.borrow(), ["first", "second", "longPress"]);
}

#[test]
fn every_original_annotation_handler_can_be_added_then_removed_in_place() {
    let actions = [
        SemanticAction::Activate,
        SemanticAction::Dismiss,
        SemanticAction::LongPress,
        SemanticAction::ScrollLeft,
        SemanticAction::ScrollRight,
        SemanticAction::ScrollUp,
        SemanticAction::ScrollDown,
        SemanticAction::Increment,
        SemanticAction::Decrement,
        SemanticAction::Copy,
        SemanticAction::Cut,
        SemanticAction::Paste,
        SemanticAction::SetSelection { base: 0, extent: 0 },
        SemanticAction::DidGainAccessibilityFocus,
        SemanticAction::DidLoseAccessibilityFocus,
    ];
    let events = Rc::new(RefCell::new(Vec::new()));
    let build = |action: Option<SemanticActionKind>| {
        let sem = Semantics::default()
            .container(true)
            .label("foo")
            .text_direction(TextDirection::Ltr);
        let trace = events.clone();
        let record = move || trace.borrow_mut().push(action.unwrap());
        match action {
            None => sem,
            Some(SemanticActionKind::Activate) => sem.on_tap(record),
            Some(SemanticActionKind::Dismiss) => sem.on_dismiss(record),
            Some(SemanticActionKind::LongPress) => sem.on_long_press(record),
            Some(SemanticActionKind::ScrollLeft) => sem.on_scroll_left(record),
            Some(SemanticActionKind::ScrollRight) => sem.on_scroll_right(record),
            Some(SemanticActionKind::ScrollUp) => sem.on_scroll_up(record),
            Some(SemanticActionKind::ScrollDown) => sem.on_scroll_down(record),
            Some(SemanticActionKind::Increment) => sem.on_increase(record),
            Some(SemanticActionKind::Decrement) => sem.on_decrease(record),
            Some(SemanticActionKind::Copy) => sem.on_copy(record),
            Some(SemanticActionKind::Cut) => sem.on_cut(record),
            Some(SemanticActionKind::Paste) => sem.on_paste(record),
            Some(SemanticActionKind::SetSelection) => sem.on_set_selection(move |selection| {
                assert_eq!((selection.base, selection.extent), (0, 0));
                record();
            }),
            Some(SemanticActionKind::DidGainAccessibilityFocus) => {
                sem.on_did_gain_accessibility_focus(record)
            }
            Some(SemanticActionKind::DidLoseAccessibilityFocus) => {
                sem.on_did_lose_accessibility_focus(record)
            }
            Some(other) => unreachable!("unexpected action {other:?}"),
        }
    };

    let mut runtime = Runtime::new(build(None).into()).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = only_node(&runtime).id;
    assert!(only_node(&runtime).actions.is_empty());
    for action in &actions {
        let kind = action.kind();
        for present in [true, false] {
            runtime
                .schedule_update(root, build(present.then_some(kind)).into())
                .unwrap();
            frame(&mut runtime);
            let node = only_node(&runtime);
            assert_eq!(node.id, id);
            assert_eq!(node.label.as_deref(), Some("foo"));
            assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
            assert_eq!(
                node.actions.as_slice(),
                if present {
                    std::slice::from_ref(&kind)
                } else {
                    &[]
                }
            );
            assert_eq!(
                runtime.dispatch_semantic_action(id, action.clone()),
                present
            );
        }
    }
    assert_eq!(*events.borrow(), actions.map(|action| action.kind()));
}
