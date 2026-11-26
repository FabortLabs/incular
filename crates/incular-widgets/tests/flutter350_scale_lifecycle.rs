use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_config::Constraints;
use incular_core::{Color, DragStartBehavior, Offset, PointerPhase, Size};
use incular_gestures::{
    PointerDeviceKind, PointerEvent, PointerPanZoomEvent, PointerPanZoomPhase, ScaleStartDetails,
    ScaleUpdateDetails,
};
use incular_widgets::{GestureDetector, Widget, internal::WidgetTree};

#[derive(Default)]
struct Trace {
    starts: Vec<ScaleStartDetails>,
    updates: Vec<ScaleUpdateDetails>,
    ends: Vec<usize>,
    taps: usize,
}

struct Scale {
    tree: WidgetTree,
    trace: Rc<RefCell<Trace>>,
    now: Instant,
}

impl Scale {
    fn new(competing_tap: bool) -> Self {
        let trace = Rc::new(RefCell::new(Trace::default()));
        macro_rules! log {
            ($field:ident) => {{
                let trace = trace.clone();
                move |details| trace.borrow_mut().$field.push(details)
            }};
        }
        let scale = GestureDetector::new(Widget::box_(Size::new(100., 100.), Color::BLACK))
            .drag_start_behavior(DragStartBehavior::Down)
            .on_scale_start(log!(starts))
            .on_scale_update(log!(updates))
            .on_scale_end({
                let trace = trace.clone();
                move |details| trace.borrow_mut().ends.push(details.pointer_count)
            });
        let widget: Widget = if competing_tap {
            GestureDetector::new(scale)
                .on_tap({
                    let trace = trace.clone();
                    move || trace.borrow_mut().taps += 1
                })
                .into()
        } else {
            scale.into()
        };
        let mut tree = WidgetTree::new();
        tree.mount(widget).unwrap();
        tree.layout(Constraints::tight(Size::new(100., 100.)))
            .unwrap();
        Self {
            tree,
            trace,
            now: Instant::now(),
        }
    }

    fn route(
        &mut self,
        pointer: u64,
        kind: PointerDeviceKind,
        position: Offset,
        phase: PointerPhase,
    ) {
        self.tree.dispatch_pointer_gesture_in_window(
            0,
            1,
            kind,
            PointerEvent {
                pointer,
                position,
                phase,
                time: self.now,
            },
        );
    }

    fn touch(&mut self, pointer: u64, x: f32, y: f32, phase: PointerPhase) {
        self.route(pointer, PointerDeviceKind::Touch, Offset::new(x, y), phase);
    }

    fn trackpad(&mut self, position: Offset, phase: PointerPanZoomPhase) {
        self.tree.dispatch_pointer_pan_zoom_in_window(
            0,
            PointerPanZoomEvent {
                device: 1,
                pointer: 6,
                position,
                phase,
                time: self.now,
                synthesized: false,
            },
        );
    }

    fn callbacks(&self, starts: usize, moves: usize, ends: usize) {
        let trace = self.trace.borrow();
        assert_eq!(
            (
                trace.starts.len(),
                trace.updates.len(),
                trace.ends.len(),
                trace.taps
            ),
            (starts, moves, ends, 0)
        );
    }

    fn details(
        &self,
        focal: (f32, f32),
        delta: (f32, f32),
        factors: (f32, f32, f32),
        count: usize,
    ) {
        let trace = self.trace.borrow();
        let d = trace.updates.last().unwrap();
        let close = |actual: f32, expected: f32| {
            assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}")
        };
        close(d.focal_point.x, focal.0);
        close(d.focal_point.y, focal.1);
        close(d.focal_point_delta.x, delta.0);
        close(d.focal_point_delta.y, delta.1);
        close(d.scale, factors.0);
        close(d.horizontal_scale, factors.1);
        close(d.vertical_scale, factors.2);
        assert_eq!(d.pointer_count, count);
    }
}

#[test]
fn scale_reports_the_start_device_across_six_successive_gestures() {
    let mut h = Scale::new(false);
    for (index, kind) in [
        PointerDeviceKind::Touch,
        PointerDeviceKind::Mouse,
        PointerDeviceKind::Stylus,
        PointerDeviceKind::InvertedStylus,
        PointerDeviceKind::Unknown,
    ]
    .into_iter()
    .enumerate()
    {
        let pointer = (index + 1) as u64;
        let origin = if index == 0 {
            Offset::ZERO
        } else {
            Offset::new(10., 20.)
        };
        h.route(pointer, kind, origin, PointerPhase::Down);
        h.callbacks(index + 1, index, index);
        h.route(pointer, kind, Offset::new(20., 30.), PointerPhase::Move);
        assert_eq!(h.trace.borrow().starts[index].kind, Some(kind));
        h.route(pointer, kind, Offset::new(20., 30.), PointerPhase::Up);
        h.callbacks(index + 1, index + 1, index + 1);
    }
    h.trackpad(Offset::new(10., 20.), PointerPanZoomPhase::Start);
    h.callbacks(6, 5, 5);
    h.trackpad(
        Offset::new(20., 30.),
        PointerPanZoomPhase::Update {
            pan: Offset::ZERO,
            scale: 1.,
            rotation: 0.,
        },
    );
    assert_eq!(
        h.trace.borrow().starts[5].kind,
        Some(PointerDeviceKind::Trackpad)
    );
    h.trackpad(Offset::new(20., 30.), PointerPanZoomPhase::End);
    h.callbacks(6, 6, 6);
}

#[test]
fn scale_checkpoints_survive_adding_removing_and_crossing_three_contacts() {
    let mut h = Scale::new(true);
    h.touch(1, 0., 0., PointerPhase::Down);
    h.callbacks(0, 0, 0);
    h.touch(1, 20., 30., PointerPhase::Move);
    h.callbacks(1, 1, 0);
    h.details((20., 30.), (20., 30.), (1., 1., 1.), 1);

    h.touch(2, 10., 20., PointerPhase::Down);
    h.callbacks(1, 1, 1);
    h.touch(2, 0., 10., PointerPhase::Move);
    h.callbacks(2, 2, 1);
    h.details((10., 20.), (-5., -5.), (2., 2., 2.), 2);
    h.touch(2, 15., 25., PointerPhase::Move);
    h.details((17.5, 27.5), (7.5, 7.5), (0.5, 0.5, 0.5), 2);
    h.touch(2, 0., 20., PointerPhase::Move);
    h.details((10., 25.), (-7.5, -2.5), (2.5_f32.sqrt(), 2., 1.), 2);
    h.touch(2, 10., 10., PointerPhase::Move);
    h.details((15., 20.), (5., -5.), (2.5_f32.sqrt(), 1., 2.), 2);
    h.touch(2, 15., 25., PointerPhase::Move);

    h.touch(3, 25., 35., PointerPhase::Down);
    h.callbacks(2, 6, 2);
    h.touch(3, 55., 65., PointerPhase::Move);
    h.callbacks(3, 7, 2);
    h.details((30., 40.), (10., 10.), (5., 5., 5.), 3);
    h.touch(1, 25., 35., PointerPhase::Move);
    h.touch(2, 20., 30., PointerPhase::Move);
    h.touch(3, 15., 25., PointerPhase::Move);
    h.callbacks(3, 10, 2);
    h.details((20., 30.), (-40. / 3., -40. / 3.), (1., 1., 1.), 3);

    h.touch(1, 25., 35., PointerPhase::Up);
    h.callbacks(3, 10, 3);
    h.touch(3, 10., 20., PointerPhase::Move);
    h.callbacks(4, 11, 3);
    h.details((15., 25.), (-2.5, -2.5), (2., 2., 2.), 2);
    h.touch(3, 30., 40., PointerPhase::Move);
    h.details((25., 35.), (10., 10.), (2., 2., 2.), 2);
    h.touch(3, 10., 20., PointerPhase::Move);
    h.details((15., 25.), (-10., -10.), (2., 2., 2.), 2);

    h.touch(2, 20., 30., PointerPhase::Up);
    h.callbacks(4, 13, 4);
    h.touch(3, 0., 0., PointerPhase::Move);
    h.callbacks(5, 14, 4);
    h.details((0., 0.), (-10., -20.), (1., 1., 1.), 1);
    h.touch(3, 0., 0., PointerPhase::Up);
    h.callbacks(5, 14, 5);
    assert_eq!(h.trace.borrow().ends, [2, 3, 2, 1, 0]);
}
