use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Alignment, Center, ColoredBox, SizedBox, TapRegion, TapRegionSurface, Widget,
};

#[test]
fn captured_tap_region_releases_keep_down_membership_and_report_the_up_position() {
    let reports = Rc::new(RefCell::new(Vec::new()));
    let log_inside = reports.clone();
    let log_outside = reports.clone();
    let target = TapRegion::new(Widget::box_(Size::new(80., 80.), Color::WHITE))
        .on_tap_up_inside(move |event| log_inside.borrow_mut().push(("inside", event.position)))
        .on_tap_up_outside(move |event| log_outside.borrow_mut().push(("outside", event.position)));
    let root = Center::new(
        SizedBox::new()
            .width(200.)
            .height(100.)
            .child(TapRegionSurface::new(ColoredBox::new(
                Color::rgba(238, 238, 238, 255),
                Align::new(Alignment::CENTER_LEFT, target),
            ))),
    );
    let mut runtime = Runtime::new(root.into()).unwrap();
    let time = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), time)
        .unwrap();

    let inside = Offset::new(340., 300.);
    let outside_region = Offset::new(460., 300.);
    let outside_surface = Offset::new(530., 300.);
    let mut next_ms = 10;
    let mut drag = |id, from, to, release| {
        for (phase, position) in [
            (PointerPhase::Down, from),
            (PointerPhase::Move, to),
            (release, to),
        ] {
            runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: id,
                    device: 1,
                    kind: PointerDeviceKind::Touch,
                    phase,
                    position,
                    buttons: if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
                        PRIMARY_POINTER_BUTTON
                    } else {
                        0
                    },
                    button: (phase == PointerPhase::Down || phase == PointerPhase::Up)
                        .then_some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::default(),
                },
                time + Duration::from_millis(next_ms),
            );
            next_ms += 10;
        }
    };

    drag(1, inside, outside_region, PointerPhase::Up);
    assert_eq!(reports.take(), [("inside", outside_region)]);
    drag(2, outside_region, inside, PointerPhase::Up);
    assert_eq!(reports.take(), [("outside", inside)]);
    drag(3, inside, outside_surface, PointerPhase::Up);
    assert_eq!(reports.take(), [("inside", outside_surface)]);
    drag(4, outside_surface, inside, PointerPhase::Up);
    assert!(reports.take().is_empty());
    drag(5, inside, outside_region, PointerPhase::Cancel);
    assert!(reports.take().is_empty());
    drag(5, outside_region, inside, PointerPhase::Up);
    assert_eq!(reports.take(), [("outside", inside)]);
}
