//! Retained contracts for CustomPaint painter layering and sizing, clip
//! delegates (`CustomClipper`), and SlideTransition hit-test policy.
//!
//! Each case mounts, lays out and paints through the public widget API.
//! Expectations cite Flutter 3.41.7 sources in `widgets/basic.dart`,
//! `rendering/custom_paint.dart`, `rendering/proxy_box.dart` and
//! `widgets/transitions.dart`.

use std::cell::Cell;
use std::rc::Rc;

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, DisplayList, PaintCommand};
use incular_widgets::internal::*;
use incular_widgets::*;

/// A painter filling its whole paint size with one color, counting paints.
struct Fill {
    color: Color,
    paints: Rc<Cell<u32>>,
    repaint: bool,
}

impl CustomPainter for Fill {
    fn paint(&self, size: Size) -> DisplayList {
        self.paints.set(self.paints.get() + 1);
        let mut canvas = Canvas::default();
        canvas.rect(Rect::from_origin_size(Offset::ZERO, size), self.color);
        canvas.finish()
    }

    fn should_repaint(&self, _old: &Self) -> bool {
        self.repaint
    }
}

fn fill(color: Color) -> Fill {
    Fill {
        color,
        paints: Rc::new(Cell::new(0)),
        repaint: true,
    }
}

/// Colors of the rects painted, in record order.
fn painted_colors(list: &DisplayList) -> Vec<Color> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

/// World-space rectangles of the clip layers, in record order.
fn clip_rects(list: &DisplayList) -> Vec<Rect> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => Some(*rect),
            _ => None,
        })
        .collect()
}

/// Clip rect of the first clip node in a tree mounted from a column.
fn mounted_clip(child: Widget) -> (WidgetTree, ElementId) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(Column::new([child]).into()).expect("mount");
    let clip = tree.children(root).expect("children")[0];
    (tree, clip)
}

/// Flutter `RenderCustomPaint.paint`: the painter paints before the child
/// and the foreground painter after it.
#[test]
fn custom_paint_orders_painter_child_and_foreground() {
    let child = Widget::box_(Size::new(50., 20.), Color::rgba(1, 1, 1, 255));
    let widget: Widget = CustomPaint::from_painter(Size::ZERO, fill(Color::rgba(10, 0, 0, 255)))
        .foreground_painter(fill(Color::rgba(0, 0, 10, 255)))
        .child(child)
        .into();
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    let colors = painted_colors(&tree.paint());
    assert_eq!(
        colors,
        vec![
            Color::rgba(10, 0, 0, 255),
            Color::rgba(1, 1, 1, 255),
            Color::rgba(0, 0, 10, 255),
        ],
        "background painter, child, then foreground painter"
    );
}

/// Flutter `RenderCustomPaint.performLayout`: with a child the size is the
/// child's, and the preferred size is ignored.
#[test]
fn custom_paint_with_child_takes_the_child_size() {
    let widget: Widget = CustomPaint::from_painter(Size::new(10., 10.), fill(Color::WHITE))
        .child(Widget::box_(Size::new(50., 20.), Color::WHITE))
        .into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    assert_eq!(
        tree.element_bounds(root).expect("bounds").size,
        Size::new(50., 20.)
    );
}

/// Flutter `RenderCustomPaint.performLayout`: `Size.infinite` constrains to
/// the biggest size, which here is the loose maximum.
#[test]
fn custom_paint_fill_to_constraints_takes_the_biggest_size() {
    let widget: Widget = CustomPaint::from_painter_filling(fill(Color::WHITE)).into();
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 120.)))
        .expect("layout");
    assert_eq!(
        tree.element_bounds(root).expect("bounds").size,
        Size::new(200., 120.)
    );
}

/// A painter paints at the laid-out size: a fixed-size painter stays fixed
/// while the fill painter follows the constraints.
#[test]
fn custom_paint_painter_sees_the_laid_out_size() {
    let widget: Widget = CustomPaint::from_painter_filling(fill(Color::WHITE)).into();
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(64., 32.)))
        .expect("layout");
    let list = tree.paint();
    let rects: Vec<Rect> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(
        rects,
        vec![Rect::from_origin_size(Offset::ZERO, Size::new(64., 32.))]
    );
}

/// Flutter `RenderCustomPaint.hitTestSelf` defers to the painter, so a
/// foreground painter never absorbs a hit meant for the child.
#[test]
fn custom_paint_foreground_does_not_absorb_child_hits() {
    let widget: Widget = CustomPaint::from_painter(Size::ZERO, fill(Color::WHITE))
        .foreground_painter(fill(Color::BLACK))
        .child(Widget::box_(Size::new(50., 20.), Color::rgba(1, 1, 1, 255)))
        .into();
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    let hit = tree
        .hit_test(Offset::new(10., 10.))
        .and_then(|render| tree.render_object_kind(render))
        .map(|kind| matches!(kind, RenderKind::Box { .. }));
    assert_eq!(hit, Some(true));
}

/// Flutter `RenderCustomPaint.painter` setter: a replacement painter whose
/// `shouldRepaint` is false keeps the recorded picture, and the new painter
/// paints only when a repaint is needed.
#[test]
fn custom_paint_skips_repaint_when_should_repaint_is_false() {
    let paints = Rc::new(Cell::new(0));
    let painter = |repaint: bool| Fill {
        color: Color::WHITE,
        paints: paints.clone(),
        repaint,
    };
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(CustomPaint::from_painter(Size::new(20., 20.), painter(true)).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    let _ = tree.paint();
    assert_eq!(paints.get(), 1);

    tree.update(
        root,
        CustomPaint::from_painter(Size::new(20., 20.), painter(false)).into(),
    )
    .expect("update");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    let _ = tree.paint();
    assert_eq!(paints.get(), 1, "shouldRepaint false keeps the picture");

    tree.update(
        root,
        CustomPaint::from_painter(Size::new(20., 20.), painter(true)).into(),
    )
    .expect("update");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    let _ = tree.paint();
    assert_eq!(paints.get(), 2, "shouldRepaint true repaints");
}

/// A clipper derives the clip from the laid-out size, and its `should_reclip`
/// decides whether a replacement delegate rebuilds the clip.
struct Width {
    width: f32,
    reclip: bool,
}

impl CustomClipper for Width {
    fn get_clip(&self, size: Size) -> ClipShape {
        ClipShape::Rect(Rect::from_origin_size(
            Offset::ZERO,
            Size::new(self.width.min(size.width), size.height),
        ))
    }

    fn should_reclip(&self, _old: &Self) -> bool {
        self.reclip
    }
}

/// Flutter `RenderClipRect` with a `CustomClipper`: the clip rectangle comes
/// from the clipper and the laid-out child size.
#[test]
fn clip_rect_custom_clipper_derives_the_clip_from_the_size() {
    let (mut tree, _clip) = mounted_clip(
        ClipRect::new(Widget::box_(Size::new(40., 40.), Color::WHITE))
            .custom_clipper(Width {
                width: 20.,
                reclip: true,
            })
            .into(),
    );
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    // The 40x40 child centers the clip at (80,0); the clipper halves the width.
    assert_eq!(
        clip_rects(&tree.paint()),
        vec![Rect::from_origin_size(
            Offset::new(80., 0.),
            Size::new(20., 40.)
        )]
    );
}

/// Flutter `RenderCustomClip` keeps its clip when `shouldReclip` is false, and
/// rebuilds it when the delegate asks for a reclip.
#[test]
fn clip_rect_custom_clipper_reclips_only_when_asked() {
    let child = || Widget::box_(Size::new(40., 40.), Color::WHITE);
    let (mut tree, clip) = mounted_clip(
        ClipRect::new(child())
            .custom_clipper(Width {
                width: 20.,
                reclip: true,
            })
            .into(),
    );
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    let width_of = |tree: &mut WidgetTree| clip_rects(&tree.paint())[0].size.width;
    assert_eq!(width_of(&mut tree), 20.);

    // A replacement that does not reclip keeps the existing clip.
    tree.update(
        clip,
        ClipRect::new(child())
            .custom_clipper(Width {
                width: 10.,
                reclip: false,
            })
            .into(),
    )
    .expect("update");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    assert_eq!(
        width_of(&mut tree),
        20.,
        "shouldReclip false keeps the clip"
    );

    // A replacement that reclips rebuilds it from the size.
    tree.update(
        clip,
        ClipRect::new(child())
            .custom_clipper(Width {
                width: 30.,
                reclip: true,
            })
            .into(),
    )
    .expect("update");
    tree.layout(Constraints::tight(Size::new(200., 200.)))
        .expect("layout");
    assert_eq!(
        width_of(&mut tree),
        30.,
        "shouldReclip true rebuilds the clip"
    );
}

/// `SlideTransition(transformHitTests: false)` keeps hits at the child's
/// unshifted layout position; the default follows the slide.
#[test]
fn slide_transition_hit_tests_follow_the_policy() {
    use incular_animation::{DrivenAnimation, LinearTween};
    use incular_widgets::SlideTransition;

    let controller = AnimationController::new(std::time::Duration::from_millis(100));
    controller.set_value(1.0);
    let position = |controller: &AnimationController| {
        DrivenAnimation::new(
            controller.clone(),
            LinearTween {
                begin: Offset::ZERO,
                end: Offset::new(0.4, 0.),
            },
        )
    };
    let boxed = || Widget::box_(Size::new(50., 20.), Color::WHITE);
    // A 0.4 fraction of the 50 px child slides it 20 px to the right.
    let hits_box_at = |widget: Widget, point: Offset| {
        let mut tree = WidgetTree::new();
        tree.mount(widget).expect("mount");
        tree.layout(Constraints::loose(Size::new(200., 200.)))
            .expect("layout");
        tree.hit_test(point)
            .and_then(|render| tree.render_object_kind(render))
            .is_some_and(|kind| matches!(kind, RenderKind::Box { .. }))
    };

    let untransformed =
        SlideTransition::from_animation_with_options(position(&controller), None, false, boxed());
    assert!(
        hits_box_at(untransformed, Offset::new(5., 10.)),
        "transformHitTests false hits the unshifted child"
    );

    let transformed =
        SlideTransition::from_animation_with_options(position(&controller), None, true, boxed());
    assert!(
        !hits_box_at(transformed, Offset::new(5., 10.)),
        "transformHitTests true follows the slide"
    );
}
