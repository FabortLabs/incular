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
