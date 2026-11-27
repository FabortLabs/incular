use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Axis, Constraints, EdgeInsets};
use incular_core::{
    DragStartBehavior, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, GestureDetector, ListView, ScrollController, SizedBox, Text,
    TextDirection, Widget, internal::Key,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
type Hits = Rc<RefCell<Vec<u64>>>;

fn list(
    hits: &Hits,
    controller: &ScrollController,
    axis: Axis,
    extent: f32,
    padding: EdgeInsets,
    reverse: bool,
    drag_from_down: bool,
) -> ListView {
    let behavior = if drag_from_down {
        DragStartBehavior::Down
    } else {
        DragStartBehavior::Start
    };
    let children = (0..6).map(|i| {
        let hits = Rc::clone(hits);
        GestureDetector::new(Widget::from(Text::new(i.to_string())).with_key(Key::from(i)))
            .drag_start_behavior(behavior)
            .on_tap(move || hits.borrow_mut().push(i))
    });
    ListView::new(children)
        .scroll_direction(axis)
        .item_extent(extent)
        .padding(padding)
        .reverse(reverse)
        .drag_start_behavior(behavior)
        .controller(controller.clone())
}

struct Host {
    runtime: Runtime,
    origin: Instant,
}

impl Host {
    fn new(child: impl Into<Widget>) -> Self {
        let root = Directionality::new(TextDirection::Ltr, child);
        let mut host = Self {
            runtime: Runtime::new(root.into()).unwrap(),
            origin: Instant::now(),
        };
        host.frame(0);
        host
    }

    fn frame(&mut self, millis: u64) {
        self.runtime
            .run_frame_at(
                Constraints::tight(VIEW),
                self.origin + Duration::from_millis(millis),
            )
            .unwrap();
    }

    fn event(&mut self, pointer: u64, phase: PointerPhase, position: Offset, millis: u64) {
        self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer,
                device: 1,
                kind: PointerDeviceKind::Touch,
                position,
                phase,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
            },
            self.origin + Duration::from_millis(millis),
        );
    }

    fn tap(&mut self, sequence: u64, point: Offset, time: u64) {
        self.event(sequence, PointerPhase::Down, point, time);
        self.event(sequence, PointerPhase::Up, point, time + 10);
    }

    fn bounds(&self, row: u64) -> Option<Rect> {
        let element = self.runtime.tree().element_with_key(&Key::from(row))?;
        self.runtime.tree().element_bounds(element)
    }

    fn center(&self, row: u64) -> Offset {
        let bounds = self.bounds(row).unwrap();
        bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.)
    }
}

fn after_drag(axis: Axis, row: u64) {
    let hits = Hits::default();
    let scroll = ScrollController::new();
    let list = list(&hits, &scroll, axis, 290., EdgeInsets::ZERO, false, true);
    let box_ = match axis {
        Axis::Horizontal => SizedBox::new().height(50.),
        Axis::Vertical => SizedBox::new().width(50.),
    };
    let mut host = Host::new(Center::new(box_.child(list)));
    let from = host.center(row);
    host.event(1, PointerPhase::Down, from, 100);
    host.event(1, PointerPhase::Move, from + axis.offset(-20., 0.), 100);
    let end = from + axis.offset(-280., 0.);
    host.event(1, PointerPhase::Move, end, 100);
    host.event(1, PointerPhase::Up, end, 100);
    host.frame(1100);

    assert_eq!(scroll.offset(), 280.);
    let visible: Vec<_> = (0..6)
        .filter(|&i| {
            host.bounds(i)
                .is_some_and(|bounds| bounds.intersects(Rect::from_origin_size(Offset::ZERO, VIEW)))
        })
        .collect();
    assert_eq!(visible, [0, 1, 2, 3]);
    for i in visible {
        let rect = host.bounds(i).unwrap();
        assert_eq!(axis.main_extent(rect.size), 290.);
        assert_eq!(
            if axis.is_horizontal() {
                rect.origin.x
            } else {
                rect.origin.y
            },
            i as f32 * 290. - 280.
        );
    }
    assert!(hits.borrow().is_empty());
    host.tap(2, host.center(row), 1200);
    assert_eq!(*hits.borrow(), [row]);
    if axis == Axis::Vertical {
        host.tap(3, host.center(3), 1300);
        assert_eq!(
            *hits.borrow(),
            [row],
            "offscreen center cannot receive a pointer"
        );
    }
}

#[test]
fn hit_testing_tracks_a_horizontal_scroll() {
    after_drag(Axis::Horizontal, 2);
}

#[test]
fn hit_testing_tracks_a_vertical_scroll_but_excludes_the_offscreen_center() {
    after_drag(Axis::Vertical, 1);
}

fn padding(reverse: bool) {
    let hits = Hits::default();
    let controller = ScrollController::new();
    let mut host = Host::new(list(
        &hits,
        &controller,
        Axis::Vertical,
        290.,
        EdgeInsets::from_ltrb(5., 20., 15., 10.),
        reverse,
        false,
    ));
    let (outside, inside, cross_y) = if reverse {
        (591., 589., 200.)
    } else {
        (19., 21., 400.)
    };
    let cases: [(Offset, &[u64]); 6] = [
        (Offset::new(200., outside), &[]),
        (Offset::new(200., inside), &[0]),
        (Offset::new(4., cross_y), &[0]),
        (Offset::new(6., cross_y), &[0, 1]),
        (Offset::new(786., cross_y), &[0, 1]),
        (Offset::new(784., cross_y), &[0, 1, 1]),
    ];
    for (i, (point, expected)) in cases.into_iter().enumerate() {
        host.tap(i as u64 + 1, point, i as u64 * 50 + 100);
        assert_eq!(
            hits.borrow().as_slice(),
            expected,
            "at {point:?}, reversed: {reverse}"
        );
    }
}

#[test]
fn normal_padding_is_not_part_of_the_item_hit_box() {
    padding(false);
}

#[test]
fn reverse_padding_uses_the_bottom_anchor_and_the_same_physical_cross_insets() {
    padding(true);
}

#[test]
fn the_immediate_tap_after_a_clamped_fling_still_reaches_the_first_item() {
    let hits = Hits::default();
    let scroll = ScrollController::new();
    let mut host = Host::new(list(
        &hits,
        &scroll,
        Axis::Vertical,
        200.,
        EdgeInsets::ZERO,
        false,
        false,
    ));
    let start = host.center(0);
    host.event(1, PointerPhase::Down, start, 100);
    for step in 1..=8 {
        host.event(
            1,
            PointerPhase::Move,
            start + Offset::new(0., step as f32 * 50.),
            100 + step * 50,
        );
    }
    host.event(1, PointerPhase::Up, start + Offset::new(0., 400.), 500);
    assert_eq!(scroll.offset(), 0.);
    assert!(hits.borrow().is_empty());
    host.tap(2, Offset::new(200., 100.), 500);
    assert_eq!(*hits.borrow(), [0]);
}
