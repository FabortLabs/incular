//! Public hit tests adapted from Flutter 3.50.0 widgets/clip_test.dart.

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Rect, Size};
use incular_gestures::PointerEvent;
use incular_rendering::Path;
use incular_widgets::{
    Align, Alignment, ClipOval, ClipPath, ClipRect, ClipShape, CustomClipper, GestureDetector,
    Opacity, SizedBox, Widget, internal::WidgetTree,
};
use std::{
    cell::RefCell,
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

fn check_oval(transparent: bool) {
    let log = Log::default();
    let oval = Widget::from(ClipOval::new(tapping(Size::new(800., 600.), &log)));
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
    check_oval(false);
}

#[test]
fn transparent_oval_still_limits_hit_testing_to_its_shape() {
    check_oval(true);
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
    let scene = ClipPath::new(rect_path(), tapping(Size::new(800., 600.), &log))
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
