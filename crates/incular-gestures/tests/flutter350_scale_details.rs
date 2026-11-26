//! Selected comparisons with Flutter 3.50.0's scale recognizer metadata.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_core::{Offset, PointerDeviceKind, PointerPhase, time::Instant};
use incular_gestures::{
    RawPointerEvent, ScaleEndDetails, ScaleGestureDetector, ScaleStartDetails, ScaleUpdateDetails,
};

#[derive(Clone, Copy, Debug)]
enum Event {
    Start(ScaleStartDetails),
    Update(ScaleUpdateDetails),
    End(ScaleEndDetails),
}

struct ScaleFixture {
    scale: ScaleGestureDetector,
    events: Rc<RefCell<Vec<Event>>>,
    base: Instant,
}

impl ScaleFixture {
    fn new() -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let scale = ScaleGestureDetector::with_callbacks(
            Some({
                let events = events.clone();
                Rc::new(move |details| events.borrow_mut().push(Event::Update(details)))
            }),
            Some({
                let events = events.clone();
                Rc::new(move |details| events.borrow_mut().push(Event::Start(details)))
            }),
            Some({
                let events = events.clone();
                Rc::new(move |details| events.borrow_mut().push(Event::End(details)))
            }),
        );
        Self {
            scale,
            events,
            base: Instant::now(),
        }
    }

    fn input(
        &mut self,
        pointer: u64,
        point: (f32, f32),
        phase: PointerPhase,
        ms: u64,
        kind: PointerDeviceKind,
    ) -> Vec<Event> {
        let _ = self.scale.handle_raw(RawPointerEvent::new(
            pointer,
            0,
            kind,
            u32::from(matches!(phase, PointerPhase::Down | PointerPhase::Move)),
            Offset::new(point.0, point.1),
            phase,
            self.base + Duration::from_millis(ms),
        ));
        self.events.borrow_mut().drain(..).collect()
    }

    fn touch(
        &mut self,
        pointer: u64,
        point: (f32, f32),
        phase: PointerPhase,
        ms: u64,
    ) -> Vec<Event> {
        self.input(pointer, point, phase, ms, PointerDeviceKind::Touch)
    }

    fn ms(&self, time: Option<Instant>) -> u128 {
        time.expect("source event stamp")
            .duration_since(self.base)
            .as_millis()
    }

    fn expect_update(
        &self,
        events: &[Event],
        start_ms: Option<u128>,
        update_ms: u128,
        geometry: [(f32, f32); 2],
        scales: [f32; 3],
        contacts: usize,
    ) {
        let [point, delta] = geometry;
        let offset = |(x, y)| Offset::new(x, y);
        let update = match (events, start_ms) {
            ([Event::Start(start), Event::Update(update)], Some(ms)) => {
                assert_eq!(self.ms(start.source_time), ms);
                assert_eq!(start.focal_point, offset(point));
                assert_eq!(start.pointer_count, contacts);
                assert_eq!(start.kind, Some(PointerDeviceKind::Touch));
                update
            }
            ([Event::Update(update)], None) => update,
            _ => panic!("unexpected event sequence: {events:?}"),
        };
        assert_eq!(self.ms(update.source_time), update_ms);
        assert_eq!(update.pointer_count, contacts);
        assert_eq!(update.focal_point, offset(point));
        let actual = update.focal_point_delta;
        assert!(
            (actual.x - delta.0).abs() < 0.0001 && (actual.y - delta.1).abs() < 0.0001,
            "expected delta {delta:?}, got {actual:?}"
        );
        for (actual, expected) in [update.scale, update.horizontal_scale, update.vertical_scale]
            .into_iter()
            .zip(scales)
        {
            assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
        }
    }
}

fn expect_end(events: &[Event], remaining: usize) {
    assert!(
        matches!(events, [Event::End(end)] if end.pointer_count == remaining),
        "expected end with {remaining} contacts, got {events:?}"
    );
}

#[test]
fn scale_reports_the_source_event_time_across_contact_reconfigurations() {
    use PointerPhase::{Down, Move, Up};
    let mut f = ScaleFixture::new();
    assert!(f.touch(1, (0., 0.), Down, 10).is_empty());
    let updates = f.touch(1, (20., 30.), Move, 20);
    f.expect_update(&updates, Some(10), 20, [(20., 30.), (20., 30.)], [1.; 3], 1);
    expect_end(&f.touch(2, (10., 20.), Down, 30), 2);
    let updates = f.touch(2, (0., 10.), Move, 40);
    f.expect_update(&updates, Some(40), 40, [(10., 20.), (-5., -5.)], [2.; 3], 2);
    let updates = f.touch(2, (15., 25.), Move, 50);
    f.expect_update(&updates, None, 50, [(17.5, 27.5), (7.5, 7.5)], [0.5; 3], 2);
    let updates = f.touch(2, (0., 20.), Move, 60);
    let radial = 5_f32.sqrt() / 2_f32.sqrt();
    f.expect_update(
        &updates,
        None,
        60,
        [(10., 25.), (-7.5, -2.5)],
        [radial, 2., 1.],
        2,
    );
    let updates = f.touch(2, (10., 10.), Move, 70);
    f.expect_update(
        &updates,
        None,
        70,
        [(15., 20.), (5., -5.)],
        [radial, 1., 2.],
        2,
    );
    let _ = f.touch(2, (15., 25.), Move, 0);
    expect_end(&f.touch(3, (25., 35.), Down, 80), 3);
    let updates = f.touch(3, (55., 65.), Move, 90);
    f.expect_update(&updates, Some(90), 90, [(30., 40.), (10., 10.)], [5.; 3], 3);

    let _ = f.touch(1, (25., 35.), Move, 100);
    let _ = f.touch(2, (20., 30.), Move, 110);
    let updates = f.touch(3, (15., 25.), Move, 120);
    f.expect_update(
        &updates,
        None,
        120,
        [(20., 30.), (-40. / 3., -40. / 3.)],
        [1.; 3],
        3,
    );
    expect_end(&f.touch(1, (25., 35.), Up, 0), 2);

    let updates = f.touch(3, (10., 20.), Move, 130);
    f.expect_update(
        &updates,
        Some(130),
        130,
        [(15., 25.), (-2.5, -2.5)],
        [2.; 3],
        2,
    );
    let updates = f.touch(3, (30., 40.), Move, 140);
    f.expect_update(&updates, None, 140, [(25., 35.), (10., 10.)], [2.; 3], 2);
    let updates = f.touch(3, (10., 20.), Move, 140);
    f.expect_update(&updates, None, 140, [(15., 25.), (-10., -10.)], [2.; 3], 2);
    expect_end(&f.touch(2, (20., 30.), Up, 0), 1);
    let updates = f.touch(3, (0., 0.), Move, 150);
    f.expect_update(
        &updates,
        Some(150),
        150,
        [(0., 0.), (-10., -20.)],
        [1.; 3],
        1,
    );
    expect_end(&f.touch(3, (0., 0.), Up, 0), 0);
}

#[test]
fn scale_start_reports_the_physical_contact_type() {
    use PointerDeviceKind::{InvertedStylus, Mouse, Stylus, Touch, Unknown};
    use PointerPhase::{Down, Move, Up};
    let mut f = ScaleFixture::new();
    for (index, kind) in [Touch, Mouse, Stylus, InvertedStylus, Unknown]
        .into_iter()
        .enumerate()
    {
        let pointer = index as u64 + 1;
        let from = (0., 0.);
        let to = (20., 40.);
        assert!(f.input(pointer, from, Down, 5 * pointer, kind).is_empty());
        let events = f.input(pointer, to, Move, 5 * pointer + 1, kind);
        assert!(
            matches!(
                events.as_slice(),
                [Event::Start(start), Event::Update(_)] if start.kind == Some(kind)
            ),
            "{events:?}"
        );
        expect_end(&f.input(pointer, to, Up, 5 * pointer + 2, kind), 0);
    }
}

#[test]
fn a_touch_only_scale_ignores_mouse_contacts_before_and_during_touch() {
    use PointerDeviceKind::{Mouse, Touch};
    use PointerPhase::{Down, Move, Up};
    let mut f = ScaleFixture::new();
    f.scale = f.scale.with_supported_devices([Touch]);

    assert!(f.input(2, (0., 0.), Down, 2, Mouse).is_empty());
    assert!(f.input(2, (20., 30.), Move, 3, Mouse).is_empty());
    assert!(f.input(2, (20., 30.), Up, 4, Mouse).is_empty());
    assert_eq!(f.scale.pointer_count(), 0);

    assert!(f.touch(1, (0., 0.), Down, 5).is_empty());
    // A lone arena member accepts on close; it must not deliver the
    // synthetic result of accept() as an input update.
    let _ = f.scale.accept().expect("arena accepts one scale member");
    let start = f.events.borrow_mut().pop().unwrap();
    assert!(matches!(
        start,
        Event::Start(d) if d.focal_point == Offset::ZERO && d.kind == Some(Touch)
    ));
    assert!(f.events.borrow().is_empty());
    let update = f.touch(1, (20., 30.), Move, 6);
    f.expect_update(&update, None, 6, [(20., 30.), (20., 30.)], [1.; 3], 1);

    assert!(f.input(2, (10., 20.), Down, 7, Mouse).is_empty());
    assert!(f.input(2, (0., 10.), Move, 8, Mouse).is_empty());
    assert!(f.input(2, (0., 10.), Up, 9, Mouse).is_empty());
    assert!(f.scale.is_started());
    assert_eq!(f.scale.pointer_count(), 1);
    expect_end(&f.touch(1, (20., 30.), Up, 10), 0);
}

#[test]
fn scale_pointer_count_follows_finger_addition_and_removal() {
    use PointerPhase::{Down, Move, Up};
    let mut f = ScaleFixture::new();
    assert!(f.touch(1, (0., 0.), Down, 1).is_empty());
    assert_eq!(f.scale.pointer_count(), 1);
    f.scale.accept().expect("only arena member");
    let start = f.events.borrow_mut().pop().unwrap();
    assert!(matches!(start, Event::Start(d) if d.pointer_count == 1));
    let update = f.touch(1, (20., 30.), Move, 2);
    assert!(matches!(update.as_slice(), [Event::Update(d)] if d.pointer_count == 1));
    expect_end(&f.touch(2, (10., 20.), Down, 3), 2);
    let update = f.touch(2, (0., 10.), Move, 4);
    f.expect_update(&update, Some(4), 4, [(10., 20.), (-5., -5.)], [2.; 3], 2);
    expect_end(&f.touch(1, (20., 30.), Up, 5), 1);
    let update = f.touch(2, (0., 10.), Move, 6);
    f.expect_update(&update, Some(6), 6, [(0., 10.), (0., 0.)], [1.; 3], 1);
    expect_end(&f.touch(2, (0., 10.), Up, 7), 0);
}
