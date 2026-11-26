//! The host accessibility lifecycle suspends and restarts sliver semantics.

use std::time::Duration;

use incular_accessibility::{MobileAccessibilityProjection, MobileAccessibilityUpdate};
use incular_config::{Constraints, TextDirection};
use incular_core::{Size, time::Instant};
use incular_runtime::Application;
use incular_semantics::SemanticRole;
use incular_widgets::{
    CustomScrollView, Directionality, Semantics, SizedBox, Sliver, SliverSemantics,
    SliverToBoxAdapter,
};

fn restart_scene() -> incular_widgets::Widget {
    let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverSemantics::new(
        || {
            Semantics::default()
                .label("test1")
                .text_direction(TextDirection::Ltr)
        },
        SliverToBoxAdapter::new(SizedBox::new().height(10.)),
    ))];
    Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
}

#[test]
fn semantics_restart_schedules_one_frame_and_recreates_the_sliver_label() {
    let mut app = Application::new(|_| restart_scene()).unwrap();
    let window = app.primary_window();
    let constraints = Constraints::tight(Size::new(800., 600.));
    let now = Instant::now();
    let mut projection = MobileAccessibilityProjection::new();

    let label_in = |update: MobileAccessibilityUpdate| {
        assert_eq!(update.nodes.len(), 3);
        assert_eq!(update.roots.len(), 1);
        let outer = update
            .nodes
            .iter()
            .find(|node| node.native_id == update.roots[0])
            .unwrap();
        assert_eq!(outer.children.len(), 1);
        assert!(!outer.semantic.state.has_implicit_scrolling);
        let label = update
            .nodes
            .iter()
            .find(|node| node.semantic.label.as_deref() == Some("test1"))
            .expect("one sliver label in the projected tree");
        assert_eq!(
            label.semantic.state.text_direction,
            Some(TextDirection::Ltr)
        );
        let scroll = update
            .nodes
            .iter()
            .find(|node| node.semantic.role == SemanticRole::ScrollView)
            .expect("scrolling semantic ancestor");
        assert!(scroll.semantic.state.has_implicit_scrolling);
        assert_eq!(outer.children, [scroll.native_id]);
        assert!(outer.semantic.tags().is_empty());
        assert!(scroll.semantic.tags().is_empty());
        // Viewport tags stay in the framework graph, not the native payload.
        assert!(label.semantic.tags().is_empty());
        assert!(label.children.is_empty());
        for node in &update.nodes {
            assert!(node.semantic.actions.is_empty());
            assert!(node.semantic.value.is_none() && node.semantic.description.is_none());
            assert!(!node.semantic.state.hidden && !node.semantic.state.focusable);
        }
        assert_eq!(scroll.children, [label.native_id]);
        label.native_id
    };
    app.run_window_frame_at(window, constraints, now)
        .unwrap()
        .unwrap();
    let old = label_in(
        app.sync_semantics_projection(window, &mut projection)
            .unwrap(),
    );
    app.set_semantics_enabled(window, false);
    app.run_window_frame_at(window, constraints, now + Duration::from_millis(16))
        .unwrap()
        .unwrap();
    assert!(!app.frame_requested(window));
    let cleared = app
        .sync_semantics_projection(window, &mut projection)
        .unwrap();
    assert!(cleared.nodes.is_empty());
    assert!(cleared.removed.contains(&old));
    assert_eq!(cleared.removed.len(), 3);

    app.set_semantics_enabled(window, true);
    assert!(app.frame_requested(window));
    app.run_window_frame_at(window, constraints, now + Duration::from_millis(32))
        .unwrap()
        .unwrap();
    let new = label_in(
        app.sync_semantics_projection(window, &mut projection)
            .unwrap(),
    );
    assert_ne!(old, new, "detached accessibility nodes are not reused");
    assert!(!app.frame_requested(window));
}

#[test]
fn sliver_framework_tag_is_rebuilt_after_semantics_restart() {
    let mut runtime = incular_runtime::Runtime::new(restart_scene()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut previous = None;
    for enabled in [true, false, true] {
        runtime.tree_mut().set_semantics_enabled(enabled);
        runtime.run_frame(bounds).unwrap();
        let tree = runtime.tree().semantics();
        if !enabled {
            assert!(tree.roots().is_empty());
            assert_eq!(tree.iter().count(), 0);
            continue;
        }
        assert_eq!(tree.iter().count(), 3);
        let outer = tree.node(tree.root().unwrap()).unwrap();
        let pane = tree.node(outer.children[0]).unwrap();
        let label = tree.node(pane.children[0]).unwrap();
        assert_eq!(label.label.as_deref(), Some("test1"));
        assert_eq!(
            label.tags(),
            &[incular_widgets::Viewport::use_two_pane_semantics_tag().clone()]
        );
        if let Some(previous) = previous {
            assert_ne!(label.id, previous);
        }
        previous = Some(label.id);
    }
}

fn sliver_scene(slivers: Vec<Box<dyn Sliver>>) -> incular_widgets::Widget {
    Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new([
            Box::new(incular_widgets::SliverMainAxisGroup::new(slivers)) as Box<dyn Sliver>
        ]),
    )
    .into()
}

#[test]
fn singular_fitted_sliver_drops_its_scroll_subtree_but_not_the_viewport() {
    use incular_core::{Offset, Rect};
    use incular_runtime::Runtime;
    use incular_widgets::{Column, FittedBox, SingleChildScrollView, Widget};

    let scene = |height| {
        let scroll = SingleChildScrollView::new(Column::new(Vec::<Widget>::new()));
        let inner = SizedBox::new().width(266.).height(55.).child(scroll);
        let box_ = SizedBox::new()
            .width(500.)
            .height(height)
            .child(FittedBox::new(inner));
        sliver_scene(vec![Box::new(SliverToBoxAdapter::new(box_))])
    };
    let viewport = Size::new(800., 600.);
    let bounds = Constraints::tight(viewport);
    let mut rt = Runtime::new(scene(0.)).unwrap();
    rt.run_frame(bounds).unwrap();
    let root = rt.tree().root().unwrap();
    let sem_root = rt.tree().semantics().root().unwrap();
    for height in [0., 55., 0.] {
        rt.schedule_update(root, scene(height)).unwrap();
        rt.run_frame(bounds).unwrap();
        let tree = rt.tree().semantics();
        assert_eq!(tree.roots(), [sem_root]);
        let viewport_node = tree.node(sem_root).unwrap();
        let world_bounds = Rect::from_origin_size(Offset::ZERO, viewport);
        assert_eq!(viewport_node.bounds, world_bounds);
        if height > 0. {
            assert_eq!(viewport_node.role, SemanticRole::GenericContainer);
            assert_eq!(viewport_node.children.len(), 1);
            let pane = tree.node(viewport_node.children[0]).unwrap();
            assert_eq!(pane.role, SemanticRole::ScrollView);
            assert!(pane.state.has_implicit_scrolling);
            assert_eq!(pane.bounds, world_bounds);
            assert_eq!(pane.children.len(), 1);
            assert_eq!(tree.iter().count(), 3);
        } else {
            assert_eq!(viewport_node.role, SemanticRole::ScrollView);
            assert!(viewport_node.state.has_implicit_scrolling);
            assert!(viewport_node.children.is_empty());
            assert_eq!(tree.iter().count(), 1);
        }
    }
}

#[test]
fn transformed_sliver_text_is_sorted_and_dropped_or_hidden_by_its_actual_bounds() {
    use std::f32::consts::FRAC_PI_2;

    use incular_runtime::Runtime;
    use incular_widgets::{Row, Transform, Widget};
    let label = |index| -> Widget {
        // Flutter's Ahem test font gives these seven characters 98 px.
        Semantics::new(SizedBox::new().width(98.).height(14.))
            .label(format!("Label {index}"))
            .role(SemanticRole::Text)
            .into()
    };
    let scene = |translation| {
        let row = Transform::rotation(FRAC_PI_2, Row::new([label(3), label(4), label(5)]));
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(SliverToBoxAdapter::new(label(1))),
            Box::new(SliverToBoxAdapter::new(label(2))),
            Box::new(SliverToBoxAdapter::new(Transform::translation(
                incular_core::Offset::new(0., translation),
                row,
            ))),
        ];
        sliver_scene(slivers)
    };
    let mut runtime = Runtime::new(scene(0.)).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    let mut projection = MobileAccessibilityProjection::new();
    for i in 0..2 {
        let tree = runtime.tree().semantics();
        assert_eq!(tree.iter().count(), 6);
        assert_eq!(tree.roots().len(), 1);
        let viewport = tree.node(tree.root().unwrap()).unwrap();
        assert_eq!(viewport.children.len(), 1);
        assert!(!viewport.state.has_implicit_scrolling);
        let pane = tree.node(viewport.children[0]).unwrap();
        assert!(pane.state.has_implicit_scrolling);
        assert_eq!(pane.bounds, viewport.bounds);
        let children = pane
            .children
            .iter()
            .map(|id| {
                let node = tree.node(*id).unwrap();
                assert!(node.children.is_empty());
                assert!(!node.state.has_implicit_scrolling);
                (
                    node.label.as_deref().unwrap(),
                    node.state.hidden,
                    node.state.text_direction,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            children,
            [
                ("Label 1", false, Some(TextDirection::Ltr)),
                ("Label 2", false, Some(TextDirection::Ltr)),
                ("Label 4", true, Some(TextDirection::Ltr)),
                ("Label 5", true, Some(TextDirection::Ltr)),
            ],
            "frame {i}"
        );
        for (_, node) in tree.iter() {
            assert!(node.actions.is_empty());
            assert!(node.value.is_none() && node.description.is_none());
        }
        if i == 0 {
            assert_eq!(projection.sync(tree).unwrap().nodes.len(), 6);
        } else {
            assert!(projection.sync(tree).is_none());
        }
        let revision = tree.revision();
        runtime.run_frame(bounds).unwrap();
        assert_eq!(runtime.tree().semantics().revision(), revision);
    }
    let root = runtime.tree().root().unwrap();
    let label_id = |runtime: &Runtime, text: &str| {
        runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(text))
            .map(|(id, node)| (id, node.state.hidden))
    };
    let retained_four = label_id(&runtime, "Label 4").unwrap();
    let retained_five = label_id(&runtime, "Label 5").unwrap();
    for (pixels, incoming) in [(30., true), (0., false)] {
        runtime.schedule_update(root, scene(pixels)).unwrap();
        runtime.run_frame(bounds).unwrap();
        assert_eq!(
            label_id(&runtime, "Label 3").map(|(_, hidden)| hidden),
            incoming.then_some(true)
        );
        assert_eq!(label_id(&runtime, "Label 4"), Some(retained_four));
        assert_eq!(label_id(&runtime, "Label 5"), Some(retained_five));
    }
}

#[test]
fn roleless_wrapper_outside_the_cache_does_not_hide_a_child_translated_back_inside() {
    use incular_config::EdgeInsets;
    use incular_core::Offset;
    use incular_runtime::Runtime;
    use incular_widgets::{Padding, Transform};

    let annotation = Semantics::new(SizedBox::new().height(20.)).label("compensated");
    let restore = Transform::translation(Offset::new(0., 500.), annotation);
    let outer = Transform::translation(
        Offset::new(0., -500.),
        Padding::new(EdgeInsets::all(2.), restore),
    );
    let mut runtime =
        Runtime::new(sliver_scene(vec![Box::new(SliverToBoxAdapter::new(outer))])).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let tree = runtime.tree().semantics();
    let node = tree
        .iter()
        .map(|(_, n)| n)
        .find(|n| n.label.as_deref() == Some("compensated"))
        .expect("compensating transform returns the actual label to the viewport");
    assert!(!node.state.hidden);
    assert_eq!(node.bounds.origin, Offset::new(2., 2.));
}

#[test]
fn a_globally_keyed_sliver_annotation_reattaches_beneath_a_new_semantic_parent() {
    use incular_widgets::{GlobalKey, internal::Key};

    let key = GlobalKey::new();
    let build = |inside_middle| {
        let child = SliverSemantics::new(
            move || {
                Semantics::default()
                    .container(true)
                    .label(if inside_middle { "test2b" } else { "test2a" })
            },
            SliverToBoxAdapter::new(SizedBox::new().height(10.)),
        )
        .key(key);
        let child = if inside_middle {
            SliverSemantics::new(
                || Semantics::default().container(true).label("middle"),
                child,
            )
        } else {
            child
        };
        let slivers: [Box<dyn Sliver>; 1] = [Box::new(SliverSemantics::new(
            || Semantics::default().label("test1"),
            child,
        ))];
        Directionality::new(TextDirection::Ltr, CustomScrollView::new(slivers)).into()
    };

    let mut runtime = incular_runtime::Runtime::new(build(false)).unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(constraints).unwrap();
    let root = runtime.tree().root().unwrap();
    let element = runtime.tree().element_with_key(&Key::Global(key)).unwrap();
    let retained_id = runtime.tree().semantic_node_for_element(element).unwrap();
    for nested in [false, true, false] {
        runtime.schedule_update(root, build(nested)).unwrap();
        runtime.run_frame(constraints).unwrap();
        let tree = runtime.tree().semantics();
        let outer = tree.node(tree.root().unwrap()).unwrap();
        let pane = tree.node(outer.children[0]).unwrap();
        assert!(pane.state.has_implicit_scrolling);
        assert_eq!(pane.children.len(), 1);
        let annotation = tree.node(pane.children[0]).unwrap();
        assert_eq!(annotation.label.as_deref(), Some("test1"));
        assert!(annotation.is_tagged(incular_widgets::Viewport::use_two_pane_semantics_tag()));
        assert_eq!(annotation.children.len(), 1);
        let target = if nested {
            let middle = tree.node(annotation.children[0]).unwrap();
            assert_eq!(middle.label.as_deref(), Some("middle"));
            assert_eq!(middle.children.len(), 1);
            middle.children[0]
        } else {
            annotation.children[0]
        };
        let label = tree.node(target).unwrap();
        assert_eq!(
            label.label.as_deref(),
            Some(if nested { "test2b" } else { "test2a" })
        );
        assert_eq!(label.id, retained_id);
        assert_eq!(
            runtime.tree().element_with_key(&Key::Global(key)),
            Some(element)
        );
        assert!(label.children.is_empty());
        assert_eq!(tree.iter().count(), if nested { 5 } else { 4 });
        for (_, node) in tree.iter() {
            assert!(node.actions.is_empty());
            assert!(node.value.is_none() && node.description.is_none());
        }
        runtime.tree().verify_invariants().unwrap();
    }
}
