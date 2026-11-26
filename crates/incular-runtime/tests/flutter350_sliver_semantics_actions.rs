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

#[test]
fn clipboard_and_expansion_callbacks_are_independent_and_withdrawn_when_blocked() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let build = |marker: &'static str, active: bool, blocked: bool| {
        let calls = calls.clone();
        scene(move || {
            let sem = Semantics::default()
                .container(true)
                .label("commands")
                .block_user_actions(blocked);
            if !active {
                return sem;
            }
            let log = |action| {
                let calls = calls.clone();
                move || calls.borrow_mut().push((marker, action))
            };
            sem.on_copy(log("copy"))
                .on_cut(log("cut"))
                .on_paste(log("paste"))
                .on_expand(log("expand"))
                .on_collapse(log("collapse"))
        })
    };
    let actions = [
        (SemanticAction::Copy, "copy"),
        (SemanticAction::Cut, "cut"),
        (SemanticAction::Paste, "paste"),
        (SemanticAction::Expand, "expand"),
        (SemanticAction::Collapse, "collapse"),
    ];
    let mut runtime = Runtime::new(build("first", true, false)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = node(&runtime, "commands").id;
    let revision = runtime.tree().semantics().revision();
    let mut projection = MobileAccessibilityProjection::new();
    let mobile_id = projection
        .sync(runtime.tree().semantics())
        .unwrap()
        .nodes
        .into_iter()
        .find(|native| native.semantic.id == id)
        .unwrap()
        .native_id;
    for marker in ["first", "second"] {
        runtime
            .schedule_update(root, build(marker, true, false))
            .unwrap();
        frame(&mut runtime);
        assert_eq!(node(&runtime, "commands").id, id);
        assert_eq!(runtime.tree().semantics().revision(), revision);
        assert!(projection.sync(runtime.tree().semantics()).is_none());
        for (action, _) in &actions {
            let mapped = projection
                .translate_action(mobile_id, action.clone())
                .unwrap();
            assert!(runtime.dispatch_semantic_action(mapped.node, mapped.action));
        }
        assert_eq!(
            calls.borrow_mut().drain(..).collect::<Vec<_>>(),
            actions
                .iter()
                .map(|(_, name)| (marker, *name))
                .collect::<Vec<_>>(),
        );
    }
    for (active, blocked) in [(true, true), (false, false)] {
        runtime
            .schedule_update(root, build("ignored", active, blocked))
            .unwrap();
        frame(&mut runtime);
        projection.sync(runtime.tree().semantics());
        assert_eq!(node(&runtime, "commands").id, id);
        for (action, _) in &actions {
            assert!(!node(&runtime, "commands").actions.contains(&action.kind()));
            assert!(
                projection
                    .translate_action(mobile_id, action.clone())
                    .is_none()
            );
            assert!(!runtime.dispatch_semantic_action(id, action.clone()));
        }
        assert!(calls.borrow().is_empty());
    }
}

#[test]
fn accesskit_expand_and_collapse_requests_reach_the_matching_callback() {
    use accesskit::{Action, ActionRequest, NodeId, TreeId};
    use incular_accessibility::AccessKitProjection;

    let calls = Rc::new(RefCell::new(Vec::new()));
    let build = |expand: bool| {
        let calls = calls.clone();
        scene(move || {
            let calls = calls.clone();
            let sem = Semantics::default().container(true).label("disclosure");
            if expand {
                sem.on_expand(move || calls.borrow_mut().push("expand"))
            } else {
                sem.on_collapse(move || calls.borrow_mut().push("collapse"))
            }
        })
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    frame(&mut runtime);
    let id = node(&runtime, "disclosure").id;
    let mut bridge = AccessKitProjection::new();
    let initial = bridge
        .sync(runtime.tree().semantics(), 1.0)
        .unwrap()
        .into_accesskit();
    let native = NodeId(bridge.native_node_id(id).unwrap());
    let emitted = initial.nodes.iter().find(|(id, _)| *id == native).unwrap();
    assert!(emitted.1.supports_action(Action::Expand));
    assert!(!emitted.1.supports_action(Action::Collapse));

    let request = |action| ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: native,
        data: None,
    };
    let expand = bridge.translate_action(&request(Action::Expand)).unwrap();
    assert!(
        bridge
            .translate_action(&request(Action::Collapse))
            .is_none()
    );
    assert!(runtime.dispatch_semantic_action(expand.node, expand.action.clone()));
    runtime
        .schedule_update(runtime.tree().root().unwrap(), build(false))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(node(&runtime, "disclosure").id, id);
    assert!(!runtime.dispatch_semantic_action(expand.node, expand.action));
    let update = bridge
        .sync(runtime.tree().semantics(), 1.0)
        .unwrap()
        .into_accesskit();
    let emitted = update.nodes.iter().find(|(id, _)| *id == native).unwrap();
    assert!(!emitted.1.supports_action(Action::Expand));
    assert!(emitted.1.supports_action(Action::Collapse));
    assert!(bridge.translate_action(&request(Action::Expand)).is_none());
    let collapse = bridge.translate_action(&request(Action::Collapse)).unwrap();
    assert!(runtime.dispatch_semantic_action(collapse.node, collapse.action));
    assert_eq!(&*calls.borrow(), &["expand", "collapse"]);
}

#[test]
fn cursor_callbacks_preserve_direction_and_selection_extension_without_an_editor() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let build = |marker: &'static str, active: bool| {
        let calls = calls.clone();
        scene(move || {
            let sem = Semantics::default()
                .container(true)
                .label("cursor controls");
            if !active {
                return sem;
            }
            let forward = calls.clone();
            let backward = calls.clone();
            sem.on_move_cursor_forward_by_character(move |extend| {
                forward.borrow_mut().push((marker, "forward", extend));
            })
            .on_move_cursor_backward_by_character(move |extend| {
                backward.borrow_mut().push((marker, "backward", extend));
            })
        })
    };
    let mut runtime = Runtime::new(build("first", true)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = node(&runtime, "cursor controls").id;
    let revision = runtime.tree().semantics().revision();
    let directions = |extend_selection| {
        [
            SemanticAction::MoveCursorForwardByCharacter { extend_selection },
            SemanticAction::MoveCursorBackwardByCharacter { extend_selection },
        ]
    };
    let mut mobile = MobileAccessibilityProjection::new();
    let native = mobile
        .sync(runtime.tree().semantics())
        .unwrap()
        .nodes
        .into_iter()
        .find(|native| native.semantic.id == id)
        .unwrap()
        .native_id;
    for marker in ["first", "second"] {
        runtime.schedule_update(root, build(marker, true)).unwrap();
        frame(&mut runtime);
        assert_eq!(node(&runtime, "cursor controls").id, id);
        assert_eq!(runtime.tree().semantics().revision(), revision);
        assert!(mobile.sync(runtime.tree().semantics()).is_none());
        for extend in [false, true] {
            for action in directions(extend) {
                let request = mobile.translate_action(native, action).unwrap();
                assert!(runtime.dispatch_semantic_action(request.node, request.action));
            }
        }
        assert_eq!(
            &*calls.borrow(),
            &[
                (marker, "forward", false),
                (marker, "backward", false),
                (marker, "forward", true),
                (marker, "backward", true),
            ],
        );
        calls.borrow_mut().clear();
    }
    runtime
        .schedule_update(root, build("removed", false))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(node(&runtime, "cursor controls").id, id);
    assert!(mobile.sync(runtime.tree().semantics()).is_some());
    for action in directions(true) {
        assert!(mobile.translate_action(native, action.clone()).is_none());
        assert!(!runtime.dispatch_semantic_action(id, action));
    }
    assert!(calls.borrow().is_empty());
}

#[test]
fn accessibility_focus_notifications_survive_user_action_blocking_without_stealing_input_focus() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let build = |marker: &'static str, handlers: bool, blocked: bool| {
        let calls = calls.clone();
        scene(move || {
            let sem = Semantics::default()
                .container(true)
                .label("focus controls")
                .block_user_actions(blocked);
            if !handlers {
                return sem.action(SemanticAction::DidGainAccessibilityFocus);
            }
            let log = |name| {
                let calls = calls.clone();
                move || calls.borrow_mut().push((marker, name))
            };
            sem.on_focus(log("request"))
                .on_did_gain_accessibility_focus(log("gained"))
                .on_did_lose_accessibility_focus(log("lost"))
        })
    };
    let mut runtime = Runtime::new(build("initial", true, false)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = node(&runtime, "focus controls").id;
    let input_focus = runtime.focused_element();
    let original_revision = runtime.tree().semantics().revision();
    let mut mobile = MobileAccessibilityProjection::new();
    let native = mobile
        .sync(runtime.tree().semantics())
        .unwrap()
        .nodes
        .into_iter()
        .find(|native| native.semantic.id == id)
        .unwrap()
        .native_id;
    let lifecycle = [
        SemanticAction::DidGainAccessibilityFocus,
        SemanticAction::DidLoseAccessibilityFocus,
    ];

    for (marker, blocked) in [
        ("initial", false),
        ("replacement", false),
        ("blocked", true),
    ] {
        runtime
            .schedule_update(root, build(marker, true, blocked))
            .unwrap();
        frame(&mut runtime);
        let update = mobile.sync(runtime.tree().semantics());
        if blocked {
            assert!(update.is_some());
        } else {
            assert!(update.is_none());
            assert_eq!(runtime.tree().semantics().revision(), original_revision);
        }
        let projected = node(&runtime, "focus controls");
        assert_eq!(projected.id, id);
        assert!(!projected.state.focusable);
        assert_eq!(
            projected.actions.contains(&SemanticActionKind::Focus),
            !blocked
        );
        for action in &lifecycle {
            let request = mobile.translate_action(native, action.clone()).unwrap();
            assert!(runtime.dispatch_semantic_action(request.node, request.action));
        }
        assert_eq!(
            runtime.dispatch_semantic_action(id, SemanticAction::Focus),
            !blocked
        );
        let mut expected = vec![(marker, "gained"), (marker, "lost")];
        if !blocked {
            expected.push((marker, "request"));
        }
        assert_eq!(&*calls.borrow(), &expected);
        calls.borrow_mut().clear();
        assert_eq!(runtime.focused_element(), input_focus);
    }

    runtime
        .schedule_update(root, build("removed", false, true))
        .unwrap();
    frame(&mut runtime);
    mobile.sync(runtime.tree().semantics());
    assert_eq!(node(&runtime, "focus controls").id, id);
    for action in lifecycle.into_iter().chain([SemanticAction::Focus]) {
        assert!(mobile.translate_action(native, action.clone()).is_none());
        assert!(!runtime.dispatch_semantic_action(id, action));
    }
    assert!(calls.borrow().is_empty());
}

#[test]
fn declared_semantic_input_focus_does_not_create_or_move_keyboard_focus() {
    let build = |focused: Option<bool>, focusable: Option<bool>| {
        scene(move || {
            let mut annotation = Semantics::default().container(true).label("annotation");
            if let Some(focused) = focused {
                annotation = annotation.focused(focused);
            }
            if let Some(focusable) = focusable {
                annotation = annotation.focusable(focusable);
            }
            annotation
        })
    };
    let mut runtime = Runtime::new(build(None, None)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let keyboard_focus = runtime.focused_element();
    let id = node(&runtime, "annotation").id;
    let mut mobile = MobileAccessibilityProjection::new();
    let mut old_flags = None;
    for (focused, focusable, expected) in [
        (None, None, (false, false)),
        (Some(false), None, (true, false)),
        (Some(true), None, (true, true)),
        (None, Some(true), (true, false)),
        (None, Some(false), (false, false)),
        (None, None, (false, false)),
    ] {
        runtime
            .schedule_update(root, build(focused, focusable))
            .unwrap();
        frame(&mut runtime);
        let annotation = node(&runtime, "annotation");
        assert_eq!(annotation.id, id);
        assert_eq!(
            (annotation.state.focusable, annotation.state.focused),
            expected
        );
        assert!(!annotation.actions.contains(&SemanticActionKind::Focus));
        assert!(!runtime.dispatch_semantic_action(id, SemanticAction::Focus));
        assert_eq!(runtime.focused_element(), keyboard_focus);
        if old_flags != Some(expected) {
            let update = mobile.sync(runtime.tree().semantics()).unwrap();
            let native = update
                .nodes
                .iter()
                .find(|entry| entry.semantic.id == id)
                .unwrap();
            assert_eq!(
                (
                    native.semantic.state.focusable,
                    native.semantic.state.focused
                ),
                expected
            );
        }
        old_flags = Some(expected);
    }
}

#[test]
fn independent_sliver_annotation_flags_survive_primary_role_changes() {
    use incular_semantics::{CheckedState, SemanticAnnotationFlag as Flag, SemanticRole};

    let build = |active| {
        scene(move || {
            let annotations = Semantics::default()
                .container(true)
                .explicit_child_nodes(true)
                .label("flags")
                .button(active)
                .slider(active)
                .keyboard_key(active)
                .link(active)
                .text_field(active)
                .in_mutually_exclusive_group(active)
                .header(active)
                .image(active)
                .scopes_route(active)
                .names_route(active)
                .expanded(active);
            if active {
                annotations.checked(CheckedState::Indeterminate)
            } else {
                annotations.toggled(true)
            }
        })
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let id = node(&runtime, "flags").id;
    let mut mobile = MobileAccessibilityProjection::new();
    let declared = [
        Flag::Button,
        Flag::Slider,
        Flag::KeyboardKey,
        Flag::Link,
        Flag::TextField,
        Flag::MutuallyExclusiveGroup,
        Flag::Header,
        Flag::Image,
        Flag::ScopesRoute,
        Flag::NamesRoute,
    ];

    for active in [true, false, true] {
        runtime.schedule_update(root, build(active)).unwrap();
        frame(&mut runtime);
        let retained = node(&runtime, "flags");
        assert_eq!(retained.id, id);
        assert_eq!(
            retained.role,
            if active {
                SemanticRole::Image
            } else {
                SemanticRole::GenericContainer
            },
        );
        for flag in declared {
            assert_eq!(
                retained.state.annotation_flags.get(flag),
                Some(active),
                "{flag:?}"
            );
        }
        assert_eq!(retained.state.expanded, Some(active));
        assert_eq!(
            retained.state.checked,
            active.then_some(CheckedState::Indeterminate)
        );
        assert_eq!(retained.state.toggled, (!active).then_some(true));
        let update = mobile.sync(runtime.tree().semantics()).unwrap();
        let native = update
            .nodes
            .iter()
            .find(|entry| entry.semantic.id == id)
            .unwrap();
        assert_eq!(
            native.semantic.state.annotation_flags,
            retained.state.annotation_flags
        );
        assert_eq!(native.semantic.state.expanded, retained.state.expanded);
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "explicit_child_nodes must be true if scopes_route is true")]
fn a_semantic_route_scope_requires_explicit_children() {
    let _ = Semantics::new(SizedBox::square(10.))
        .scopes_route(true)
        .explicit();
}

#[test]
fn merging_annotations_unions_independent_flags_without_reassigning_them() {
    use incular_semantics::SemanticAnnotationFlag as Flag;
    use incular_widgets::{Column, MergeSemantics};

    let a = Semantics::new(SizedBox::square(10.))
        .label("a")
        .button(true);
    let b = Semantics::new(SizedBox::square(10.))
        .label("b")
        .button(false)
        .keyboard_key(true);
    let mut runtime = Runtime::new(MergeSemantics::new(Column::new([a, b])).into()).unwrap();
    frame(&mut runtime);
    let tree = runtime.tree().semantics();
    let merged = tree
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some("a\nb"))
        .unwrap();
    assert_eq!(merged.state.annotation_flags.get(Flag::Button), Some(true));
    assert_eq!(
        merged.state.annotation_flags.get(Flag::KeyboardKey),
        Some(true)
    );
    assert_eq!(merged.state.annotation_flags.get(Flag::Image), None);
}
