use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_rendering::{ClipCoverage, DisplayList, LayerKind, PaintCommand};
use incular_widgets::{
    BorderSide, BoxFit, CircleBorder, CustomPaint, CustomPainter, FittedBox, PhysicalModel,
    PhysicalShape, ShapeBorder, SizedBox, StadiumBorder, Widget, internal::WidgetTree,
};
use std::{cell::Cell, rc::Rc};

struct CountPaint(Rc<Cell<usize>>);
impl CustomPainter for CountPaint {
    fn paint(&self, _: Size) -> DisplayList {
        self.0.set(self.0.get() + 1);
        DisplayList::default()
    }
    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

fn count_layers(tree: &WidgetTree, matches: impl Fn(&LayerKind) -> bool) -> usize {
    let layers = tree.compositor_layers();
    let mut queue: Vec<_> = layers.root().into_iter().collect();
    let mut count = 0;
    while let Some(id) = queue.pop() {
        let (kind, children) = layers.layer(id).unwrap();
        if matches(kind) {
            count += 1;
        }
        queue.extend_from_slice(children);
    }
    count
}

#[test]
fn fitted_box_does_not_call_its_painter_when_either_size_is_empty() {
    let unit = Size::new(1., 1.);
    let view = Constraints::loose(Size::new(800., 600.));
    for (preferred, constraints, paints) in [
        (unit, view, 1),
        (Size::ZERO, view, 0),
        (unit, Constraints::tight(Size::ZERO), 0),
    ] {
        let calls = Rc::new(Cell::new(0));
        let child = CustomPaint::from_painter(preferred, CountPaint(calls.clone()));
        let mut tree = WidgetTree::new();
        tree.mount(FittedBox::new(child).into()).unwrap();
        tree.layout(constraints).unwrap();
        let _ = tree.paint();
        assert_eq!(calls.get(), paints);
        if paints == 0 {
            assert!(tree.hit_test(Offset::ZERO).is_none());
            assert_eq!(
                count_layers(&tree, |kind| matches!(kind,
                    LayerKind::Transform { transform, .. } if *transform == Transform::scale(0.)
                )),
                1
            );
        }
    }
}

#[test]
fn fitted_box_clip_matches_all_modes_for_a_200_square_within_a_100_square() {
    let clip_scene = |clip| -> Widget {
        let fit =
            FittedBox::new(Widget::box_(Size::new(200., 200.), Color::WHITE)).fit(BoxFit::None);
        if let Some(clip) = clip {
            fit.clip_behavior(clip).into()
        } else {
            fit.into()
        }
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(clip_scene(None)).unwrap();
    let bounds = Size::new(100., 100.);
    for (clip, kind) in [
        (None, None),
        (Some(Clip::None), None),
        (Some(Clip::HardEdge), Some(ClipCoverage::HardEdge)),
        (Some(Clip::AntiAlias), Some(ClipCoverage::AntiAlias)),
        (
            Some(Clip::AntiAliasWithSaveLayer),
            Some(ClipCoverage::AntiAliasWithSaveLayer),
        ),
    ] {
        tree.update(root, clip_scene(clip)).unwrap();
        tree.layout(Constraints::loose(bounds)).unwrap();
        let display = tree.paint();
        let actual: Vec<_> = display
            .commands()
            .iter()
            .filter_map(|cmd| match cmd {
                PaintCommand::PushClip { rect } => Some((*rect, ClipCoverage::HardEdge)),
                PaintCommand::PushClipRRect {
                    rrect, coverage, ..
                } => Some((rrect.rect, *coverage)),
                _ => None,
            })
            .collect();
        assert_eq!(
            actual,
            kind.map(|kind| (Rect::from_origin_size(Offset::ZERO, bounds), kind))
                .into_iter()
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn physical_model_emits_one_shadow_only_for_positive_elevation() {
    let scene = |elevation| {
        PhysicalModel::new(Color::rgba(255, 0, 255, 255), SizedBox::square(10.))
            .elevation(elevation)
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(0.)).unwrap();
    let render = tree.render_id(root).unwrap();
    for (elevation, count) in [(0., 0), (1., 1), (0., 0)] {
        tree.update(root, scene(elevation)).unwrap();
        tree.layout(Constraints::loose(Size::new(800., 600.)))
            .unwrap();
        assert_eq!(tree.render_id(root), Some(render));
        let display = tree.paint();
        assert_eq!(
            display
                .commands()
                .iter()
                .filter(|cmd| matches!(cmd, PaintCommand::PushDropShadow { .. }))
                .count(),
            count
        );
    }
}

#[test]
fn changing_physical_shape_repaints_but_equivalent_path_does_not() {
    let size = Size::new(200., 100.);
    let rect = Rect::from_origin_size(Offset::ZERO, size);
    let scene = |shape: &dyn ShapeBorder| {
        PhysicalShape::new(
            shape.outer_path(rect),
            Color::rgba(255, 0, 255, 255),
            SizedBox::new().width(size.width).height(size.height),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(&CircleBorder::new(BorderSide::NONE)))
        .unwrap();
    let layout = Constraints::loose(size);
    tree.layout(layout).unwrap();
    let first = tree.paint();
    let paints = tree.diagnostics().paints;

    tree.update(root, scene(&CircleBorder::new(BorderSide::NONE)))
        .unwrap();
    tree.layout(layout).unwrap();
    assert_eq!(first, tree.paint());
    assert_eq!(tree.diagnostics().paints, paints, "equal clipper geometry");

    tree.update(root, scene(&StadiumBorder::new(BorderSide::NONE)))
        .unwrap();
    tree.layout(layout).unwrap();
    assert_ne!(first, tree.paint());
    assert!(tree.diagnostics().paints > paints, "new shape must repaint");
}

fn retained_layers_matching(
    tree: &WidgetTree,
    matches: impl Fn(&LayerKind) -> bool,
) -> Vec<incular_rendering::LayerId> {
    let layers = tree.compositor_layers();
    let mut queue: Vec<_> = layers.root().into_iter().collect();
    let mut selected = Vec::new();
    while let Some(id) = queue.pop() {
        let (kind, children) = layers.layer(id).unwrap();
        if matches(kind) {
            selected.push(id);
        }
        queue.extend_from_slice(children);
    }
    selected
}

fn repaint_preserving_effect(scene: Widget, matches: impl Fn(&LayerKind) -> bool) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene).unwrap();
    tree.layout(Constraints::tight(Size::new(10., 10.)))
        .unwrap();
    let _ = tree.paint();
    let before = retained_layers_matching(&tree, &matches);
    assert_eq!(before.len(), 1);
    let paints = tree.diagnostics().paints;
    tree.mark_paint(root).unwrap();
    let _ = tree.paint();
    assert!(tree.diagnostics().paints > paints);
    assert_eq!(retained_layers_matching(&tree, &matches), before);
}

#[test]
fn fitted_box_retains_its_clip_or_transform_layer_after_a_real_repaint() {
    use incular_widgets::RepaintBoundary;

    for clip in [true, false, true] {
        let child = RepaintBoundary::new(
            SizedBox::new()
                .width(if clip { 100. } else { 1. })
                .height(if clip { 200. } else { 1. }),
        );
        let fit = FittedBox::new(child);
        let scene = if clip {
            fit.fit(BoxFit::Cover).clip_behavior(Clip::HardEdge)
        } else {
            fit.fit(BoxFit::Fill)
        };
        repaint_preserving_effect(scene.into(), |kind| {
            if clip {
                matches!(kind, LayerKind::ClipRect { .. })
            } else {
                matches!(kind, LayerKind::Transform {transform, ..} if *transform == Transform::scale(10.))
            }
        });
    }
}

#[test]
fn static_and_controller_opacity_layers_keep_their_ids_after_repaint() {
    use incular_widgets::{FadeTransition, Opacity, OpacityController, RepaintBoundary};
    let controller = OpacityController::new();
    assert!(controller.set_opacity(0.5));
    for scene in [
        Widget::from(Opacity::new(
            0.5,
            RepaintBoundary::new(SizedBox::square(1.)),
        )),
        FadeTransition::new(controller, SizedBox::square(1.)).into(),
    ] {
        repaint_preserving_effect(
            scene,
            |kind| matches!(kind, LayerKind::Opacity { alpha } if (*alpha-0.5).abs() <= 1./255.),
        );
    }
}

#[test]
fn fitted_child_becoming_visible_receives_a_previously_deferred_paint() {
    let calls = Rc::new(Cell::new(0));
    let child = CustomPaint::from_painter(Size::new(1., 1.), CountPaint(calls.clone()));
    let mut tree = WidgetTree::new();
    tree.mount(FittedBox::new(child).into()).unwrap();
    for (bounds, count) in [
        (Size::ZERO, 0),
        (Size::new(10., 10.), 1),
        (Size::ZERO, 1),
        (Size::new(10., 10.), 1),
    ] {
        tree.layout(Constraints::tight(bounds)).unwrap();
        let _ = tree.paint();
        assert_eq!(calls.get(), count);
    }
}

#[test]
fn offstage_and_both_box_opacity_paths_report_when_a_child_is_painted() {
    use incular_widgets::{FadeTransition, Offstage, Opacity, OpacityController};
    let child = || SizedBox::new().width(20.);
    let mut tree = WidgetTree::new();
    let scene = |visible: bool| Offstage::new(child()).offstage(!visible).into();
    let root = tree.mount(scene(true)).unwrap();
    let can_paint = |tree: &WidgetTree, root| {
        let [child] = tree.children(root).unwrap() else {
            panic!("one proxy child")
        };
        tree.paints_child(
            tree.render_id(root).unwrap(),
            tree.render_id(*child).unwrap(),
        )
    };
    assert!(can_paint(&tree, root));
    tree.update(root, scene(false)).unwrap();
    assert!(!can_paint(&tree, root));

    let root = tree.mount(Opacity::new(1., child()).into()).unwrap();
    assert!(can_paint(&tree, root));
    tree.update(root, Opacity::new(0., child()).into()).unwrap();
    assert!(!can_paint(&tree, root));

    let controller = OpacityController::new();
    let root = tree
        .mount(FadeTransition::new(controller.clone(), child()).into())
        .unwrap();
    assert!(can_paint(&tree, root));
    assert!(controller.set_opacity(0.));
    assert!(!can_paint(&tree, root));
}

#[test]
fn sliver_fade_uses_the_same_live_paints_child_gate() {
    use incular_widgets::{
        CustomScrollView, OpacityController, Sliver, SliverFadeTransition, SliverToBoxAdapter,
        internal::RenderKind,
    };
    let controller = OpacityController::new();
    let sliver: Box<dyn Sliver> = Box::new(SliverFadeTransition::new(
        controller.clone(),
        SliverToBoxAdapter::new(SizedBox::new().width(20.).height(20.)),
    ));
    let mut tree = WidgetTree::new();
    let root = tree.mount(CustomScrollView::new([sliver]).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let mut queue = vec![root];
    let (parent, child) = loop {
        let element = queue.pop().expect("rendered sliver opacity");
        let render = tree.render_id(element).unwrap();
        let children = tree.children(element).unwrap();
        if matches!(
            tree.render_object_kind(render),
            Some(RenderKind::Opacity { .. })
        ) {
            let [child] = children else {
                panic!("one opacity child")
            };
            break (render, tree.render_id(*child).unwrap());
        }
        queue.extend_from_slice(children);
    };
    assert!(tree.paints_child(parent, child));
    assert!(controller.set_opacity(0.));
    assert!(!tree.paints_child(parent, child));
}

#[test]
fn empty_physical_layers_do_not_composite_a_shadow_at_any_elevation() {
    let magenta = Color::rgba(255, 0, 255, 255);
    for physical_shape in [false, true] {
        let scene = |elevation, side| {
            if physical_shape {
                PhysicalShape::new(
                    CircleBorder::new(BorderSide::NONE)
                        .outer_path(Rect::from_origin_size(Offset::ZERO, Size::new(side, side))),
                    magenta,
                    SizedBox::square(side),
                )
                .elevation(elevation)
                .into()
            } else {
                PhysicalModel::new(magenta, SizedBox::square(side))
                    .elevation(elevation)
                    .into()
            }
        };
        let mut tree = WidgetTree::new();
        let root = tree.mount(scene(0., 0.)).unwrap();
        let render = tree.render_id(root).unwrap();
        let layout = Constraints::loose(Size::new(800., 600.));
        for (elevation, side, paints_shadow) in [
            (0., 0., false),
            (1., 0., false),
            (1., 10., true),
            (1., 0., false),
            (0., 0., false),
        ] {
            tree.update(root, scene(elevation, side)).unwrap();
            tree.layout(layout).unwrap();
            assert_eq!(tree.render_id(root), Some(render));
            assert_eq!(
                tree.element_bounds(root).unwrap().size,
                Size::new(side, side)
            );
            let list = tree.paint();
            assert_eq!(
                list.commands()
                    .iter()
                    .any(|cmd| matches!(cmd, PaintCommand::PushDropShadow { .. })),
                paints_shadow,
                "shape={physical_shape} elevation={elevation} side={side}",
            );
        }
    }
}
