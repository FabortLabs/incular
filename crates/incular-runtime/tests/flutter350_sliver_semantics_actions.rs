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

struct BoxedSliver(Box<dyn Sliver>);

impl Sliver for BoxedSliver {
    fn build(&self, ctl: &incular_widgets::ScrollController) -> Widget {
        self.0.build(ctl)
    }

    fn create_render_sliver(
        &self,
        ctl: &incular_widgets::ScrollController,
        axis: incular_config::Axis,
        reverse: bool,
    ) -> Box<dyn incular_widgets::internal::RenderSliver> {
        self.0.create_render_sliver(ctl, axis, reverse)
    }
}

fn blocked_actions_scene(
    boxed: bool,
    block_parent: bool,
    block_first: bool,
    physical_hits: Rc<std::cell::Cell<u32>>,
    semantic_hits: Rc<RefCell<Vec<usize>>>,
) -> Widget {
    use incular_core::Color;
    use incular_widgets::{GestureDetector, SliverList, SliverMainAxisGroup};

    let content = move || {
        Widget::from(
            GestureDetector::new(Widget::box_(Size::new(800., 10.), Color::WHITE)).on_tap({
                let hits = physical_hits.clone();
                move || hits.set(hits.get() + 1)
            }),
        )
    };
    let annotation = move |index, child: Widget| {
        let hits = semantic_hits.clone();
        Semantics::new(child)
            .label(if index == 0 { "label1" } else { "label2" })
            .block_user_actions(index == 0 && block_first)
            .on_tap(move || hits.borrow_mut().push(index))
    };
    let group: Box<dyn Sliver> = if boxed {
        Box::new(SliverList::builder(2, move |index| {
            annotation(index, content())
        }))
    } else {
        Box::new(SliverMainAxisGroup::new((0..2).map(
            |index| -> Box<dyn Sliver> {
                let annotation = annotation.clone();
                Box::new(SliverSemantics::new(
                    move || annotation(index, SizedBox::shrink().into()),
                    SliverToBoxAdapter::new(content()),
                ))
            },
        )))
    };
    let root = SliverSemantics::new(
        move || {
            Semantics::default()
                .container(true)
                .label("group")
                .explicit_child_nodes(block_parent)
                .block_user_actions(block_parent)
        },
        BoxedSliver(group),
    );
    let slivers: [Box<dyn Sliver>; 1] = [Box::new(root)];
    Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
}

#[test]
fn semantic_sliver_block_preserves_labels_and_physical_pointer_input() {
    use incular_core::{InputEvent, Offset, PointerPhase};
    use std::cell::Cell;

    for boxed in [false, true] {
        let physical = Rc::new(Cell::new(0));
        let semantic = Rc::new(RefCell::new(Vec::new()));
        let view =
            |block| blocked_actions_scene(boxed, block, false, physical.clone(), semantic.clone());
        let mut runtime = Runtime::new(view(true)).unwrap();
        frame(&mut runtime);
        let root = runtime.tree().root().unwrap();
        let children = ["label1", "label2"].map(|name| node(&runtime, name).id);
        assert_eq!(node(&runtime, "group").children, children);
        for id in children {
            assert!(
                !runtime
                    .tree()
                    .semantics()
                    .node(id)
                    .unwrap()
                    .actions
                    .contains(&SemanticActionKind::Activate)
            );
            assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Activate));
        }
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = runtime.handle_input(InputEvent::Pointer {
                phase,
                position: Offset::new(250., 5.),
            });
        }
        assert_eq!(physical.get(), 1);
        assert!(semantic.borrow().is_empty());
        runtime.schedule_update(root, view(false)).unwrap();
        frame(&mut runtime);
        assert_eq!(
            ["label1", "label2"].map(|name| node(&runtime, name).id),
            children
        );
        for id in children {
            assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));
        }
        assert_eq!(&*semantic.borrow(), &[0, 1]);
    }
}

#[test]
fn tap_action_conflict_stays_separate_even_if_one_annotation_blocks_it() {
    use std::cell::Cell;

    for boxed in [false, true] {
        let semantic = Rc::new(RefCell::new(Vec::new()));
        let scene =
            blocked_actions_scene(boxed, false, true, Rc::new(Cell::new(0)), semantic.clone());
        let mut runtime = Runtime::new(scene).unwrap();
        frame(&mut runtime);
        let first = node(&runtime, "label1").id;
        let second = node(&runtime, "label2").id;
        assert_eq!(node(&runtime, "group").children, [first, second]);
        assert!(!runtime.dispatch_semantic_action(first, SemanticAction::Activate));
        assert!(runtime.dispatch_semantic_action(second, SemanticAction::Activate));
        assert_eq!(&*semantic.borrow(), &[1]);
    }
}

fn disjoint_actions_scene(
    boxed: bool,
    block_tap: bool,
    second_is_tap: bool,
    marker: &'static str,
    trace: Rc<RefCell<Vec<&'static str>>>,
) -> Widget {
    use incular_widgets::{SliverList, SliverMainAxisGroup};

    let annotation = move |index, child: Widget| {
        let trace = trace.clone();
        let sem = Semantics::new(child).label(if index == 0 { "label1" } else { "label2" });
        if index == 0 {
            sem.block_user_actions(block_tap)
                .on_tap(move || trace.borrow_mut().push("tap"))
        } else if second_is_tap {
            sem.on_tap(move || trace.borrow_mut().push(marker))
        } else {
            sem.on_long_press(move || trace.borrow_mut().push(marker))
        }
    };
    let group: Box<dyn Sliver> = if boxed {
        Box::new(SliverList::builder(2, move |i| {
            annotation(i, SizedBox::new().height(10.).into())
        }))
    } else {
        Box::new(SliverMainAxisGroup::new((0..2).map(
            |i| -> Box<dyn Sliver> {
                let annotation = annotation.clone();
                Box::new(SliverSemantics::new(
                    move || annotation(i, SizedBox::shrink().into()),
                    SliverToBoxAdapter::new(SizedBox::new().height(10.)),
                ))
            },
        )))
    };
    let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverSemantics::new(
        || Semantics::default().container(true),
        BoxedSliver(group),
    ))];
    Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
}

#[test]
fn blocked_tap_merges_its_label_with_a_long_press_but_not_its_callback() {
    for boxed in [false, true] {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let build =
            |block, marker| disjoint_actions_scene(boxed, block, false, marker, trace.clone());
        let mut runtime = Runtime::new(build(true, "long")).unwrap();
        frame(&mut runtime);
        let merged = node(&runtime, "label1\nlabel2");
        let id = merged.id;
        assert!(merged.children.is_empty());
        assert!(!merged.actions.contains(&SemanticActionKind::Activate));
        assert!(merged.actions.contains(&SemanticActionKind::LongPress));
        assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Activate));
        assert!(runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
        assert_eq!(&*trace.borrow(), &["long"]);

        let root = runtime.tree().root().unwrap();
        let revision = runtime.tree().semantics().revision();
        runtime
            .schedule_update(root, build(true, "replacement"))
            .unwrap();
        frame(&mut runtime);
        assert_eq!(node(&runtime, "label1\nlabel2").id, id);
        assert_eq!(runtime.tree().semantics().revision(), revision);
        assert!(runtime.dispatch_semantic_action(id, SemanticAction::LongPress));
        assert_eq!(&*trace.borrow(), &["long", "replacement"]);

        runtime
            .schedule_update(root, build(false, "replacement"))
            .unwrap();
        frame(&mut runtime);
        let merged = node(&runtime, "label1\nlabel2");
        assert_eq!(merged.id, id);
        assert!(merged.actions.contains(&SemanticActionKind::Activate));
        assert!(runtime.dispatch_semantic_action(id, SemanticAction::Activate));
        assert_eq!(&*trace.borrow(), &["long", "replacement", "tap"]);
    }
}

#[test]
fn declarative_tap_conflict_is_not_removed_by_blocking_one_callback() {
    for boxed in [false, true] {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = Runtime::new(disjoint_actions_scene(
            boxed,
            true,
            true,
            "second tap",
            trace.clone(),
        ))
        .unwrap();
        frame(&mut runtime);
        let first = node(&runtime, "label1").id;
        let second = node(&runtime, "label2").id;
        assert!(
            runtime
                .tree()
                .semantics()
                .iter()
                .any(|(_, parent)| parent.children == [first, second])
        );
        assert!(!runtime.dispatch_semantic_action(first, SemanticAction::Activate));
        assert!(runtime.dispatch_semantic_action(second, SemanticAction::Activate));
        assert_eq!(&*trace.borrow(), &["second tap"]);
    }
}

#[test]
fn sliver_text_payloads_are_delivered_unchanged_to_current_handlers() {
    #[derive(Debug, PartialEq)]
    enum Call {
        Text(&'static str, String),
        Selection(&'static str, usize, usize),
    }
    let calls = Rc::new(RefCell::new(Vec::new()));
    let build = |marker: &'static str, active: bool, blocked: bool| {
        let calls = calls.clone();
        scene(move || {
            let semantics = Semantics::default()
                .container(true)
                .label("editable annotation")
                .block_user_actions(blocked);
            if active {
                let text = calls.clone();
                let selection = calls.clone();
                semantics
                    .on_set_text(move |value| text.borrow_mut().push(Call::Text(marker, value)))
                    .on_set_selection(move |value| {
                        selection.borrow_mut().push(Call::Selection(
                            marker,
                            value.base,
                            value.extent,
                        ));
                    })
            } else {
                // Declaring a payload kind alone must not create an unreachable action.
                semantics
                    .action(SemanticAction::SetText(String::new()))
                    .action(SemanticAction::SetSelection { base: 0, extent: 0 })
            }
        })
    };
    let mut runtime = Runtime::new(build("first", true, false)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = node(&runtime, "editable annotation").id;
    let revision = runtime.tree().semantics().revision();
    let mut projection = MobileAccessibilityProjection::new();
    assert!(projection.sync(runtime.tree().semantics()).is_some());
    let has_text_actions = |runtime: &Runtime, expected| {
        let node = node(runtime, "editable annotation");
        assert_eq!(node.id, id);
        assert_eq!(
            node.actions.contains(&SemanticActionKind::SetText),
            expected
        );
        assert_eq!(
            node.actions.contains(&SemanticActionKind::SetSelection),
            expected
        );
        assert!(!node.state.focusable);
    };
    has_text_actions(&runtime, true);
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::SetText("café\nrepeat".into())));
    assert!(
        runtime.dispatch_semantic_action(id, SemanticAction::SetSelection { base: 4, extent: 1 },)
    );

    runtime
        .schedule_update(root, build("second", true, false))
        .unwrap();
    frame(&mut runtime);
    has_text_actions(&runtime, true);
    assert_eq!(runtime.tree().semantics().revision(), revision);
    assert!(projection.sync(runtime.tree().semantics()).is_none());
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::SetText(String::new())));
    assert!(
        runtime.dispatch_semantic_action(id, SemanticAction::SetSelection { base: 1, extent: 5 },)
    );
    assert_eq!(
        &*calls.borrow(),
        &[
            Call::Text("first", "café\nrepeat".into()),
            Call::Selection("first", 4, 1),
            Call::Text("second", String::new()),
            Call::Selection("second", 1, 5),
        ],
    );

    for (active, blocked) in [(true, true), (false, false)] {
        runtime
            .schedule_update(root, build("ignored", active, blocked))
            .unwrap();
        frame(&mut runtime);
        has_text_actions(&runtime, false);
        assert!(!runtime.dispatch_semantic_action(id, SemanticAction::SetText("ignored".into())));
        assert!(
            !runtime
                .dispatch_semantic_action(id, SemanticAction::SetSelection { base: 0, extent: 0 },)
        );
        assert_eq!(calls.borrow().len(), 4);
    }
}
