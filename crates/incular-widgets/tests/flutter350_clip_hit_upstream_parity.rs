//! Public hit tests adapted from Flutter 3.50.0 widgets/clip_test.dart.

use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, PointerPhase, Rect, Size};
use incular_gestures::PointerEvent;
use incular_rendering::{CornerRadii, PaintCommand, Path};
use incular_widgets::{
    Align, Alignment, BorderRadiusDirectional, ClipOval, ClipPath, ClipRRect, ClipReclip, ClipRect,
    ClipShape, CustomClipper, Directionality, GestureDetector, Opacity, Placeholder, Radius,
    SizedBox, Widget,
    internal::{Key, RenderKind, WidgetTree},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

type Log = Rc<RefCell<Vec<&'static str>>>;

fn tapping(size: Size, log: &Log) -> Widget {
    let log = log.clone();
    GestureDetector::new(Widget::box_(size, Color::WHITE))
        .on_tap(move || log.borrow_mut().push("tap"))
        .into()
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
}

fn tap(tree: &mut WidgetTree, position: Offset, start: Instant, index: u64) {
    for (phase, offset) in [(PointerPhase::Down, 0), (PointerPhase::Up, 10)] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position,
            phase,
            time: start + Duration::from_millis(100 * index + offset),
        });
    }
}

fn directional_radii() -> BorderRadiusDirectional {
    BorderRadiusDirectional::horizontal(Radius::circular(15.), Radius::circular(30.))
}

fn rounded_clip_state(tree: &WidgetTree, key: &Key) -> (CornerRadii, Option<TextDirection>) {
    let render = tree.render_id(tree.element_with_key(key).unwrap()).unwrap();
    match tree.render_object_kind(render) {
        Some(RenderKind::ClipRRect {
            radius,
            text_direction,
            ..
        }) => (radius.to_corner_radii(), *text_direction),
        other => panic!("expected rounded clip, got {other:?}"),
    }
}

/// clip_test.dart:826. Check all four physical corners in both directions.
#[test]
fn childless_rounded_clip_resolves_start_and_end_radii() {
    let key = Key::from("directional rounded clip");
    let clip = Widget::from(
        ClipRRect::builder()
            .directional_radius(directional_radii())
            .build(),
    )
    .with_key(key.clone());
    let mut tree = WidgetTree::new();
    let scene = |direction| Directionality::new(direction, clip.clone()).into();
    let root = tree.mount(scene(TextDirection::Rtl)).unwrap();
    for (direction, (left, right)) in [
        (TextDirection::Rtl, (30., 15.)),
        (TextDirection::Ltr, (15., 30.)),
    ] {
        tree.update(root, scene(direction)).unwrap();
        layout(&mut tree);
        assert_eq!(
            rounded_clip_state(&tree, &key),
            (
                CornerRadii {
                    top_left: left,
                    top_right: right,
                    bottom_left: left,
                    bottom_right: right,
                },
                Some(direction)
            )
        );
    }
}

/// clip_test.dart:862. A tap changes the state in the same mounted subtree.
#[test]
fn rounded_clip_direction_changes_on_the_same_render_object_after_tap() {
    let key = Key::from("stateful directional clip");
    let direction = Rc::new(Cell::new(TextDirection::Ltr));
    let revision = Rc::new(Cell::new(0_u64));
    let scene = Widget::stateful_builder(revision.clone(), {
        let key = key.clone();
        move |_| {
            let touch = GestureDetector::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
                .on_tap({
                    let direction = direction.clone();
                    let revision = revision.clone();
                    move || {
                        direction.set(TextDirection::Rtl);
                        revision.set(revision.get() + 1);
                    }
                });
            let clip =
                Widget::from(ClipRRect::new(0., touch).directional_radius(directional_radii()))
                    .with_key(key.clone());
            Directionality::new(direction.get(), SizedBox::square(100.).child(clip)).into()
        }
    });
    let mut tree = WidgetTree::new();
    tree.mount(scene).unwrap();
    layout(&mut tree);
    let render = tree
        .render_id(tree.element_with_key(&key).unwrap())
        .unwrap();
    let (_, before) = rounded_clip_state(&tree, &key);
    assert_eq!(before, Some(TextDirection::Ltr));
    tap(&mut tree, Offset::ZERO, Instant::now(), 0);
    layout(&mut tree);
    assert_eq!(
        tree.render_id(tree.element_with_key(&key).unwrap()),
        Some(render)
    );
    let (corners, after) = rounded_clip_state(&tree, &key);
    assert_eq!(after, Some(TextDirection::Rtl));
    assert_eq!((corners.top_left, corners.top_right), (30., 15.));
    assert!(tree.paint().commands().iter().any(|op| matches!(
        op, PaintCommand::PushClipRRect { rrect, .. } if rrect.radii == corners
    )));
}

fn check_oval(transparent: bool, clip: Clip) {
    let log = Log::default();
    let oval =
        Widget::from(ClipOval::new(tapping(Size::new(800., 600.), &log)).clip_behavior(clip));
    let scene = if transparent {
        Opacity::new(0., oval).into()
    } else {
        oval
    };
    let mut tree = WidgetTree::new();
    tree.mount(scene).unwrap();
    layout(&mut tree);
    assert!(log.borrow().is_empty());
    let start = Instant::now();
    tap(&mut tree, Offset::new(10., 10.), start, 0);
    let corner = log.borrow().clone();
    log.borrow_mut().clear();
    tap(&mut tree, Offset::new(400., 300.), start, 1);
    assert_eq!(log.borrow().as_slice(), &["tap"], "center contact control");
    assert!(
        corner.is_empty(),
        "corner contact outside the oval: {corner:?}"
    );
}

#[test]
fn oval_hit_test_excludes_fully_clipped_corners() {
    check_oval(false, Clip::HardEdge);
}

#[test]
fn transparent_oval_still_limits_hit_testing_to_its_shape() {
    check_oval(true, Clip::HardEdge);
}

#[test]
fn oval_paint_clip_none_still_uses_elliptical_hit_shape() {
    check_oval(false, Clip::None);
}

struct PathClipper(Log);

impl CustomClipper for PathClipper {
    fn get_clip(&self, _: Size) -> ClipShape {
        self.0.borrow_mut().push("getClip");
        ClipShape::Path(rect_path())
    }
    fn should_reclip(&self, _: &Self) -> bool {
        false
    }
}

fn rect_path() -> Arc<Path> {
    let mut path = Path::builder();
    path.move_to(Offset::new(50., 50.))
        .line_to(Offset::new(150., 50.))
        .line_to(Offset::new(150., 150.))
        .line_to(Offset::new(50., 150.))
        .close();
    Arc::new(path.build())
}

#[test]
fn custom_clip_path_excludes_outside_contact_and_reuses_cached_geometry() {
    let log = Log::default();
    let scene = ClipPath::builder()
        .child(tapping(Size::new(800., 600.), &log))
        .build()
        .custom_clipper(PathClipper(log.clone()));
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).unwrap();
    layout(&mut tree);
    let initial = log.borrow().clone();
    let start = Instant::now();
    tap(&mut tree, Offset::new(10., 10.), start, 0);
    let outside = log.borrow().clone();
    log.borrow_mut().clear();
    tap(&mut tree, Offset::new(100., 100.), start, 1);
    assert_eq!(
        (initial, outside, log.borrow().clone()),
        (vec!["getClip"], vec!["getClip"], vec!["tap"])
    );
}

struct RectClipper {
    name: &'static str,
    rect: Rect,
    log: Log,
}

#[derive(Clone)]
struct NotifiedRectClipper {
    geometry: Rc<Cell<Rect>>,
    changed: ClipReclip,
}

impl CustomClipper for NotifiedRectClipper {
    fn get_clip(&self, _: Size) -> ClipShape {
        ClipShape::Rect(self.geometry.get())
    }
    fn should_reclip(&self, old: &Self) -> bool {
        !Rc::ptr_eq(&self.geometry, &old.geometry)
    }
    fn reclip(&self) -> Option<ClipReclip> {
        Some(self.changed.clone())
    }
}

fn observable_rect(left: f32, width: f32) -> NotifiedRectClipper {
    NotifiedRectClipper {
        geometry: Rc::new(Cell::new(Rect::from_origin_size(
            Offset::new(left, 50.),
            Size::new(width, 100.),
        ))),
        changed: ClipReclip::default(),
    }
}

fn expect_current_rect(tree: &mut WidgetTree, expected: Rect) {
    let paint = tree.paint();
    assert!(
        paint
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::PushClip { rect } if *rect == expected)),
        "expected clip {expected:?}, got {:?}",
        paint.commands()
    );
}

/// clip_test.dart:788. Notification invalidates the clip without replacing
/// the descriptor; the next paint uses the widened geometry.
#[test]
fn custom_clipper_reclips_when_notified_without_child_layout() {
    let clipper = observable_rect(50., 100.);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            ClipRect::new(Placeholder::new())
                .custom_clipper(clipper.clone())
                .into(),
        )
        .unwrap();
    let render = tree.render_id(root).unwrap();
    layout(&mut tree);
    expect_current_rect(&mut tree, clipper.geometry.get());
    assert!(!tree.has_dirty_dependencies());
    let before = tree.diagnostics();
    let next = Rect::from_origin_size(Offset::new(50., 50.), Size::new(150., 100.));
    clipper.geometry.set(next);
    clipper.changed.notify();
    assert!(tree.has_dirty_dependencies());
    layout(&mut tree);
    expect_current_rect(&mut tree, next);
    assert!(!tree.has_dirty_dependencies());
    assert_eq!(tree.render_id(root), Some(render));
    assert_eq!(tree.diagnostics().layouts, before.layouts);
    assert_eq!(tree.diagnostics().paints, before.paints);
}

#[test]
fn custom_clipper_replaces_its_notification_source_and_updates_hit_geometry() {
    let previous = observable_rect(50., 100.);
    let current = observable_rect(200., 100.);
    let log = Log::default();
    let scene = |clipper| {
        ClipRect::new(tapping(Size::new(800., 600.), &log))
            .custom_clipper(clipper)
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(previous.clone())).unwrap();
    layout(&mut tree);
    tree.update(root, scene(current.clone())).unwrap();
    layout(&mut tree);
    previous.changed.notify();
    assert!(!tree.has_dirty_dependencies());
    let start = Instant::now();
    tap(&mut tree, Offset::new(100., 100.), start, 0);
    assert!(log.borrow().is_empty());
    current.geometry.set(Rect::from_origin_size(
        Offset::new(50., 50.),
        Size::new(100., 100.),
    ));
    current.changed.notify();
    layout(&mut tree);
    tap(&mut tree, Offset::new(100., 100.), start, 1);
    assert_eq!(log.borrow().as_slice(), &["tap"]);
}

impl CustomClipper for RectClipper {
    fn get_clip(&self, _: Size) -> ClipShape {
        self.log.borrow_mut().push(self.name);
        ClipShape::Rect(self.rect)
    }
    fn should_reclip(&self, old: &Self) -> bool {
        self.name != old.name || self.rect != old.rect
    }
}

#[test]
fn custom_rect_clip_recomputes_only_on_size_and_delegate_changes_and_limits_hit_testing() {
    let log = Log::default();
    let scene = |size, name, origin| {
        let size = Size::new(size, size);
        let clip = ClipRect::new(tapping(size, &log)).custom_clipper(RectClipper {
            name,
            rect: Rect::from_origin_size(Offset::new(origin, origin), Size::new(10., 10.)),
            log: log.clone(),
        });
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new()
                .width(size.width)
                .height(size.height)
                .child(clip),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let mut snapshots = Vec::new();
    let mut capture = || snapshots.push(log.borrow().clone());
    let root = tree.mount(scene(100., "a", 5.)).unwrap();
    layout(&mut tree);
    capture();
    let start = Instant::now();
    tap(&mut tree, Offset::new(10., 10.), start, 0);
    capture();
    tap(&mut tree, Offset::new(100., 100.), start, 1);
    capture();
    for (size, name, origin) in [
        (100., "a", 5.),
        (200., "a", 5.),
        (200., "a", 5.),
        (200., "b", 5.),
        (200., "c", 25.),
    ] {
        tree.update(root, scene(size, name, origin)).unwrap();
        layout(&mut tree);
        capture();
    }
    tap(&mut tree, Offset::new(30., 30.), start, 2);
    capture();
    tap(&mut tree, Offset::new(100., 100.), start, 3);
    capture();

    assert_eq!(
        snapshots,
        [
            vec!["a"],
            vec!["a", "tap"],
            vec!["a", "tap"],
            vec!["a", "tap"],
            vec!["a", "tap", "a"],
            vec!["a", "tap", "a"],
            vec!["a", "tap", "a", "b"],
            vec!["a", "tap", "a", "b", "c"],
            vec!["a", "tap", "a", "b", "c", "tap"],
            vec!["a", "tap", "a", "b", "c", "tap"],
        ]
    );
}

#[test]
fn custom_rect_hit_clip_persists_across_paint_policy_changes_and_excludes_trailing_edges() {
    let log = Log::default();
    let area = Rect::from_origin_size(Offset::new(10., 10.), Size::new(30., 30.));
    let scene = |clip| -> Widget {
        ClipRect::new(tapping(Size::new(800., 600.), &log))
            .clip_behavior(clip)
            .custom_clipper(RectClipper {
                name: "clip",
                rect: area,
                log: log.clone(),
            })
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(Clip::None)).unwrap();
    layout(&mut tree);
    assert!(log.borrow().is_empty(), "no paint clip is resolved yet");
    let start = Instant::now();
    for (i, point) in [(9., 20.), (10., 10.), (39., 39.), (40., 39.), (39., 40.)]
        .into_iter()
        .enumerate()
    {
        tap(&mut tree, Offset::new(point.0, point.1), start, i as u64);
    }
    assert_eq!(log.borrow().as_slice(), ["clip", "tap", "tap"]);

    tree.update(root, scene(Clip::HardEdge)).unwrap();
    layout(&mut tree);
    assert!(tree.paint().commands().iter().any(|command| matches!(
        command, PaintCommand::PushClip { rect } if *rect == area
    )));
    tree.update(root, scene(Clip::None)).unwrap();
    layout(&mut tree);
    assert!(
        !tree
            .paint()
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::PushClip { .. }))
    );
    tap(&mut tree, Offset::new(20., 20.), start, 5);
    assert_eq!(log.borrow().as_slice(), ["clip", "tap", "tap", "tap"]);
}
