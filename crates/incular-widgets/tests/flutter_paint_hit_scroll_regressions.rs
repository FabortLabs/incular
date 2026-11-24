//! Neutral Flutter 3.50.0 painting, hit-test, and scroll regressions.
//! See B02/B03 and C26/C27 in the pinned parity audit.

use incular_config::{Axis, Constraints};
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_rendering::{PaintCommand, Path};
use incular_scroll::{ScrollController, SliverConstraints};
use incular_widgets::{
    Border, BorderSide, BorderStyle, BoxDecoration, ClipPath, ClipShape, Container, CustomClipper,
    Directionality, GestureDetector, SingleChildScrollView, SizedBox, Sliver,
    SliverFixedExtentList, TextDirection, Widget,
    internal::{Key, WidgetTree},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

fn paints_border(style: Option<BorderStyle>) -> bool {
    let mut tree = WidgetTree::new();
    let tint = Color::rgba(217, 11, 79, 255);
    let size = Size::new(40.0, 40.0);
    let mut decoration = BoxDecoration::new();
    if let Some(style) = style {
        decoration = decoration.border(Border::all(BorderSide::new(tint, 2.0, style)));
    }
    tree.mount(
        Container::new()
            .width(40.0)
            .height(40.0)
            .decoration(decoration)
            .child(Widget::box_(Size::new(10.0, 10.0), Color::WHITE))
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(size)).unwrap();

    tree.paint().commands().iter().any(|cmd| {
        matches!(cmd, PaintCommand::Border { border, .. }
        if border.width == 2.0 && border.color == tint)
    })
}

#[test]
fn none_style_container_border_does_not_paint() {
    assert!(!paints_border(None), "no-border control");
    assert!(
        paints_border(Some(BorderStyle::Solid)),
        "solid-border control"
    );
    assert!(
        !paints_border(Some(BorderStyle::None)),
        "invisible border painted"
    );
}

fn built_indices(extent: f32, cache_start: f32, cache_end: f32) -> Vec<usize> {
    let built = Rc::new(RefCell::new(Vec::new()));
    let observed = built.clone();
    let sliver = SliverFixedExtentList::new(10, extent, move |index| {
        observed.borrow_mut().push(index);
        Widget::box_(Size::new(40.0, extent), Color::WHITE)
    });
    let mut render = sliver.create_render_sliver(&ScrollController::new(), Axis::Vertical, false);
    let cache_extent = cache_end - cache_start;
    let _ = render.perform_layout(SliverConstraints::new(
        Axis::Vertical,
        false,
        cache_start,
        0.0,
        0.0,
        cache_extent,
        40.0,
        cache_extent,
        cache_extent,
        0.0,
    ));
    let indices = built.borrow().clone();
    indices
}

#[test]
fn fixed_extent_cache_excludes_row_at_fractional_end() {
    assert_eq!(
        built_indices(20.0, 0.0, 60.0),
        [0, 1, 2],
        "integer edge control"
    );
    assert_eq!(
        built_indices(20.3, 0.0, 60.9),
        [0, 1, 2],
        "row at cache end"
    );
    assert_eq!(
        built_indices(20.3, 0.0, 60.899),
        [0, 1, 2],
        "just before the end"
    );
    assert_eq!(
        built_indices(20.3, 0.0, 60.901),
        [0, 1, 2, 3],
        "actual overlap still builds a row"
    );
}

#[test]
fn fixed_extent_cache_excludes_preceding_row_at_fractional_start() {
    assert_eq!(
        built_indices(1.1, 7.7, 8.05),
        [7],
        "preceding row touches but does not overlap"
    );
    assert_eq!(
        built_indices(1.1, 7.699, 8.05),
        [6, 7],
        "genuine overlap still builds preceding row"
    );
    assert_eq!(built_indices(1.1, 7.701, 8.05), [7]);
}
fn triangle(size: Size) -> Arc<Path> {
    let mut path = Path::builder();
    path.move_to(Offset::ZERO)
        .line_to(Offset::new(size.width, 0.0))
        .line_to(Offset::new(0.0, size.height))
        .close();
    Arc::new(path.build())
}

struct Triangle;

impl CustomClipper for Triangle {
    fn get_clip(&self, size: Size) -> ClipShape {
        ClipShape::Path(triangle(size))
    }
}

#[test]
fn clip_path_rejects_hits_inside_layout_bounds_but_outside_clip() {
    let size = Size::new(100.0, 100.0);
    let taps = Rc::new(Cell::new(0));
    let counter = taps.clone();
    let child = GestureDetector::new(Widget::box_(size, Color::WHITE))
        .on_tap(move || counter.set(counter.get() + 1));
    let mut tree = WidgetTree::new();
    tree.mount(
        ClipPath::new(triangle(size), child)
            .custom_clipper(Triangle)
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    let _ = tree.paint();

    let origin = Instant::now();
    let mut tap_at = |point, at| {
        for (phase, elapsed) in [(PointerPhase::Down, at), (PointerPhase::Up, at + 10)] {
            tree.dispatch_gesture(PointerEvent {
                pointer: 1,
                position: point,
                phase,
                time: origin + Duration::from_millis(elapsed),
            });
        }
    };

    tap_at(Offset::new(20.0, 20.0), 0);
    assert_eq!(taps.get(), 1, "inside-path control");
    tap_at(Offset::new(80.0, 80.0), 100);
    assert_eq!(taps.get(), 1, "outside-path tap reached GestureDetector");
}

fn horizontal_content_origin(direction: TextDirection, reverse: bool) -> f32 {
    let key = Key::from(36051_u64);
    let content = Widget::from(SizedBox::new().width(200.0).height(60.0)).with_key(key.clone());
    let mut tree = WidgetTree::new();
    let scroll = SingleChildScrollView::new(content)
        .scroll_direction(Axis::Horizontal)
        .reverse(reverse);
    tree.mount(Directionality::new(direction, scroll).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(100.0, 60.0)))
        .unwrap();
    let _ = tree.paint();
    let child = tree.element_with_key(&key).unwrap();
    tree.element_bounds(child).unwrap().origin.x
}

#[test]
fn horizontal_scroll_offset_zero_respects_rtl_and_reverse() {
    assert_eq!(
        [false, true].map(|reverse| horizontal_content_origin(TextDirection::Ltr, reverse)),
        [0.0, -100.0],
        "LTR control",
    );
    assert_eq!(
        [false, true].map(|reverse| horizontal_content_origin(TextDirection::Rtl, reverse)),
        [-100.0, 0.0],
        "RTL changes the horizontal axis before applying reverse",
    );
}

#[test]
fn horizontal_scroll_reuses_its_offset_when_directionality_changes() {
    let key = Key::from(36052_u64);
    let controller = ScrollController::new();
    let scene = |direction| -> Widget {
        let child = Widget::from(SizedBox::new().width(200.).height(60.)).with_key(key.clone());
        Directionality::new(
            direction,
            SingleChildScrollView::new(child)
                .controller(controller.clone())
                .scroll_direction(Axis::Horizontal),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Ltr)).unwrap();
    let constraints = Constraints::tight(Size::new(100., 60.));
    tree.layout(constraints).unwrap();
    let child = tree.element_with_key(&key).unwrap();
    controller.jump_to(25.);
    tree.layout(constraints).unwrap();
    let _ = tree.paint();
    assert_eq!(tree.element_bounds(child).unwrap().origin.x, -25.);
    for (direction, x) in [(TextDirection::Rtl, -75.), (TextDirection::Ltr, -25.)] {
        tree.update(root, scene(direction)).unwrap();
        tree.layout(constraints).unwrap();
        let _ = tree.paint();
        assert_eq!(controller.offset(), 25.);
        assert_eq!(tree.element_with_key(&key), Some(child));
        assert_eq!(tree.element_bounds(child).unwrap().origin.x, x);
    }
}
