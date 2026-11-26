//! Live action binding changes must not republish the whole semantic tree.

use std::{cell::RefCell, rc::Rc};

use incular_accessibility::MobileAccessibilityProjection;
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind, SemanticNode};
use incular_widgets::{
    CustomScrollView, Directionality, Semantics, SizedBox, Sliver, SliverSemantics,
    SliverToBoxAdapter, Widget,
};

fn scene(annotations: impl Fn() -> Semantics + 'static) -> Widget {
    let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverSemantics::new(
        annotations,
        SliverToBoxAdapter::new(SizedBox::new().height(10.)),
    ))];
    Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn node<'a>(runtime: &'a Runtime, label: &str) -> &'a SemanticNode {
    runtime
        .tree()
        .semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap()
}

#[test]
fn replacing_only_a_tap_handler_uses_the_new_callback_without_a_native_update() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let build = |tap: &'static str, long: bool| {
        let trace = events.clone();
        scene(move || {
            let tap_log = trace.clone();
            let sem = Semantics::default()
                .container(true)
                .label("handler")
                .on_tap(move || tap_log.borrow_mut().push(tap));
            if long {
                let trace = trace.clone();
                sem.on_long_press(move || trace.borrow_mut().push("long press"))
            } else {
                sem
            }
        })
    };
    let mut runtime = Runtime::new(build("first", false)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = node(&runtime, "handler").id;
    let initial_revision = runtime.tree().semantics().revision();
    let mut projection = MobileAccessibilityProjection::new();
    assert!(projection.sync(runtime.tree().semantics()).is_some());
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));
    assert_eq!(&*events.borrow(), &["first"]);
    assert!(projection.sync(runtime.tree().semantics()).is_none());

    runtime
        .schedule_update(root, build("second", false))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(node(&runtime, "handler").id, id);
    assert_eq!(runtime.tree().semantics().revision(), initial_revision);
    assert!(projection.sync(runtime.tree().semantics()).is_none());
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));
    assert_eq!(&*events.borrow(), &["first", "second"]);

    runtime
        .schedule_update(root, build("second", true))
        .unwrap();
    frame(&mut runtime);
    let updated = node(&runtime, "handler");
    assert_eq!(updated.id, id);
    assert!(updated.actions.contains(&SemanticActionKind::LongPress));
    assert!(runtime.tree().semantics().revision() > initial_revision);
    assert!(projection.sync(runtime.tree().semantics()).is_some());
    assert!(projection.sync(runtime.tree().semantics()).is_none());
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
    assert_eq!(&*events.borrow(), &["first", "second", "long press"]);

    runtime
        .schedule_update(root, build("second", false))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(node(&runtime, "handler").id, id);
    assert!(
        !node(&runtime, "handler")
            .actions
            .contains(&SemanticActionKind::LongPress)
    );
    assert!(projection.sync(runtime.tree().semantics()).is_some());
    assert!(!runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
    assert_eq!(&*events.borrow(), &["first", "second", "long press"]);
}

#[test]
fn sliver_annotation_adds_dispatches_and_removes_each_supported_callback() {
    use SemanticActionKind::*;

    let actions = [
        (Activate, SemanticAction::Activate),
        (Dismiss, SemanticAction::Dismiss),
        (LongPress, SemanticAction::LongPress),
        (Increment, SemanticAction::Increment),
        (Decrement, SemanticAction::Decrement),
        (ScrollLeft, SemanticAction::ScrollLeft),
        (ScrollRight, SemanticAction::ScrollRight),
        (ScrollUp, SemanticAction::ScrollUp),
        (ScrollDown, SemanticAction::ScrollDown),
    ];
    let trace = Rc::new(RefCell::new(Vec::new()));
    let build = |action: Option<SemanticActionKind>| {
        let trace = trace.clone();
        scene(move || {
            let sem = Semantics::default()
                .container(true)
                .label("foo")
                .text_direction(TextDirection::Ltr);
            let trace = trace.clone();
            let on_action = move || trace.borrow_mut().push(action.unwrap());
            match action {
                Some(Activate) => sem.on_tap(on_action),
                Some(Dismiss) => sem.on_dismiss(on_action),
                Some(LongPress) => sem.on_long_press(on_action),
                Some(Increment) => sem.on_increase(on_action),
                Some(Decrement) => sem.on_decrease(on_action),
                Some(ScrollLeft) => sem.on_scroll_left(on_action),
                Some(ScrollRight) => sem.on_scroll_right(on_action),
                Some(ScrollUp) => sem.on_scroll_up(on_action),
                Some(ScrollDown) => sem.on_scroll_down(on_action),
                _ => sem,
            }
        })
    };
    let mut runtime = Runtime::new(build(None)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = node(&runtime, "foo").id;
    for (i, (kind, data)) in actions.iter().enumerate() {
        runtime.schedule_update(root, build(Some(*kind))).unwrap();
        frame(&mut runtime);
        let current = node(&runtime, "foo");
        assert_eq!(current.id, id);
        assert_eq!(current.state.text_direction, Some(TextDirection::Ltr));
        for (other, _) in &actions {
            assert_eq!(
                current.actions.contains(other),
                other == kind,
                "enabled {kind:?}, checked {other:?}"
            );
        }
        assert!(
            runtime.dispatch_semantic_action(id, data.clone()),
            "{kind:?}"
        );
        assert_eq!(trace.borrow()[i], *kind);

        runtime.schedule_update(root, build(None)).unwrap();
        frame(&mut runtime);
        let current = node(&runtime, "foo");
        assert_eq!(current.id, id);
        assert!(!current.actions.contains(kind), "{kind:?}");
        assert!(
            !runtime.dispatch_semantic_action(id, data.clone()),
            "stale {kind:?}"
        );
        assert_eq!(trace.borrow().len(), i + 1);
    }
}

#[test]
fn standard_action_hints_describe_tap_and_long_press_without_a_duplicate_action() {
    use incular_semantics::CustomSemanticsAction;

    let hits = Rc::new(RefCell::new(Vec::new()));
    let build = |long| {
        let hits = hits.clone();
        scene(move || {
            let hits = hits.clone();
            let base = Semantics::default().container(true).label("target");
            if long {
                base.on_long_press(move || hits.borrow_mut().push("long press"))
                    .on_long_press_hint("foo")
            } else {
                base.on_tap(move || hits.borrow_mut().push("tap"))
                    .on_tap_hint("test")
            }
        })
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    for (long, kind, action, hint) in [
        (
            false,
            SemanticActionKind::Activate,
            SemanticAction::Activate,
            "test",
        ),
        (
            true,
            SemanticActionKind::LongPress,
            SemanticAction::LongPress,
            "foo",
        ),
    ] {
        if long {
            runtime.schedule_update(root, build(true)).unwrap();
            frame(&mut runtime);
        }
        let target = node(&runtime, "target");
        assert!(target.actions.contains(&kind));
        assert!(!target.actions.contains(&SemanticActionKind::Custom));
        let [hint_id] = target.state.custom_actions.as_ref() else {
            panic!("expected exactly one hint override");
        };
        let info = CustomSemanticsAction::get_action(*hint_id).unwrap();
        assert_eq!(
            (info.action(), info.hint(), info.label()),
            (Some(kind), Some(hint), None)
        );
        let id = target.id;
        let hint_id = *hint_id;
        assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Custom(hint_id)));
        assert!(runtime.dispatch_semantic_action(id, action));
    }
    assert_eq!(&*hits.borrow(), &["tap", "long press"]);
}

#[test]
fn named_custom_actions_retain_independent_dispatch() {
    use incular_semantics::CustomSemanticsAction;

    let hits = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = Runtime::new(scene({
        let hits = hits.clone();
        move || {
            let foo = hits.clone();
            let bar = hits.clone();
            Semantics::default()
                .container(true)
                .label("target")
                .custom_action(CustomSemanticsAction::new("foo"), move || {
                    foo.borrow_mut().push("foo")
                })
                .custom_action(CustomSemanticsAction::new("bar"), move || {
                    bar.borrow_mut().push("bar")
                })
        }
    }))
    .unwrap();
    frame(&mut runtime);
    let target = node(&runtime, "target");
    assert!(target.actions.contains(&SemanticActionKind::Custom));
    let id = target.id;
    let mut mapped = target
        .state
        .custom_actions
        .iter()
        .map(|id| {
            let action = CustomSemanticsAction::get_action(*id).unwrap();
            assert_eq!(action.action(), None);
            (action.label().unwrap().to_owned(), *id)
        })
        .collect::<Vec<_>>();
    mapped.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        mapped
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["bar", "foo"]
    );
    for (_, action) in mapped {
        assert!(runtime.dispatch_semantic_action(id, SemanticAction::Custom(action)));
    }
    assert_eq!(&*hits.borrow(), &["bar", "foo"]);
}

#[test]
fn tooltip_and_adjacent_range_values_remain_separate_semantic_properties() {
    let mut runtime = Runtime::new(scene(|| {
        Semantics::default()
            .container(true)
            .label("target")
            .tooltip("test1")
    }))
    .unwrap();
    frame(&mut runtime);
    let tooltip = node(&runtime, "target");
    assert_eq!(tooltip.state.tooltip.as_deref(), Some("test1"));
    assert_eq!(tooltip.description, None);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(
            root,
            scene(|| {
                Semantics::default()
                    .container(true)
                    .label("target")
                    .value("10s")
                    .increased_value("11s")
                    .decreased_value("9s")
                    .on_increase(|| {})
                    .on_decrease(|| {})
            }),
        )
        .unwrap();
    frame(&mut runtime);
    let range = node(&runtime, "target");
    assert_eq!(range.state.tooltip, None);
    assert_eq!(
        (
            range.value.as_deref(),
            range.state.increased_value.as_deref(),
            range.state.decreased_value.as_deref()
        ),
        (Some("10s"), Some("11s"), Some("9s"))
    );
    assert!(range.actions.contains(&SemanticActionKind::Increment));
    assert!(range.actions.contains(&SemanticActionKind::Decrement));
}

#[test]
fn excluding_sliver_semantics_keeps_its_annotation_and_visual_input() {
    use std::cell::Cell;

    use incular_core::InputEvent;
    use incular_core::{Color, Offset, PointerPhase};
    use incular_rendering::PaintCommand;
    use incular_widgets::GestureDetector;

    let hits = Rc::new(Cell::new(0));
    let fill = Color::rgba(201, 66, 10, 255);
    let build = |exclude| {
        let plated = GestureDetector::new(Widget::box_(Size::new(800., 10.), fill)).on_tap({
            let hits = hits.clone();
            move || hits.set(hits.get() + 1)
        });
        let child = SliverSemantics::new(
            || Semantics::default().container(true).label("other label"),
            SliverToBoxAdapter::new(plated),
        );
        let parent = SliverSemantics::new(|| Semantics::default().label("label"), child)
            .exclude_semantics(exclude);
        let slivers: [Box<dyn Sliver>; 1] = [Box::new(parent)];
        Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    let (display, _) = runtime.run_frame(bounds).unwrap();
    let outer = node(&runtime, "label").id;
    assert!(
        !runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, n)| n.label.as_deref() == Some("other label"))
    );
    assert!(display.commands().iter().any(|command| matches!(
        command, PaintCommand::Rect { color, rect } if *color == fill && rect.size.height == 10.
    )));
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(200., 5.),
        });
    }
    assert_eq!(hits.get(), 1);

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(false)).unwrap();
    frame(&mut runtime);
    let parent = node(&runtime, "label");
    assert_eq!(parent.id, outer);
    assert_eq!(parent.children, [node(&runtime, "other label").id]);
}
