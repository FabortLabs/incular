use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Offset, Rect, Size, Transform, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Container, InteractiveViewer, SizedBox, TransformationController, Widget,
    internal::{PointerData, PointerDataChange as Change},
};

fn frame(runtime: &mut Runtime, viewport: Size, now: Instant) {
    runtime
        .run_frame_at(Constraints::tight(viewport), now)
        .unwrap();
}

#[test]
fn tight_viewport_cannot_pan_before_or_after_resizing() {
    let controller = TransformationController::new();
    let widget = Center::new(
        InteractiveViewer::new(Container::new()).transformation_controller(controller.clone()),
    );
    let mut runtime = Runtime::new(widget.into()).unwrap();
    let mut time = Instant::now();
    for viewport in [Size::new(800., 600.), Size::new(100., 100.)] {
        frame(&mut runtime, viewport, time);
        assert_eq!(controller.value(), Transform::IDENTITY);
        for (change, at) in [
            (Change::Down, Offset::new(20., 20.)),
            (Change::Move, Offset::ZERO),
            (Change::Up, Offset::ZERO),
        ] {
            time += Duration::from_millis(110);
            runtime.handle_pointer_data_packet(
                1,
                1.,
                [PointerData {
                    view_id: 1,
                    pointer: 1,
                    device: 8,
                    physical_position: at,
                    ..PointerData::new(change, time)
                }],
            );
            frame(&mut runtime, viewport, time);
        }
        assert_eq!(controller.value(), Transform::IDENTITY);
    }
}

#[test]
fn only_active_controller_requests_builder_rebuild_and_resize_uses_new_viewport() {
    let old = TransformationController::new();
    let current = TransformationController::new();
    let builds = Rc::new(RefCell::new(Vec::<Rect>::new()));
    let widget = |controller: &TransformationController| -> Widget {
        InteractiveViewer::builder({
            let logger = builds.clone();
            move |_, viewport| {
                logger.borrow_mut().push(viewport.bounds());
                SizedBox::square(300.).into()
            }
        })
        .transformation_controller(controller.clone())
        .into()
    };
    let mut runtime = Runtime::new(widget(&old)).unwrap();
    let mut now = Instant::now();
    let a = Size::new(200., 100.);
    let b = Size::new(100., 60.);
    frame(&mut runtime, a, now);
    assert_eq!(builds.borrow().last().unwrap().size, a);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, widget(&current)).unwrap();
    now += Duration::from_millis(16);
    frame(&mut runtime, a, now);
    let count = builds.borrow().len();

    old.set_value(Transform::translation(Offset::new(-15., 0.)));
    assert!(
        !runtime.frame_requested(),
        "the old controller still wants a frame"
    );
    assert_eq!(builds.borrow().len(), count);

    current.set_value(Transform::translation(Offset::new(-20., -5.)));
    assert!(runtime.frame_requested());
    now += Duration::from_millis(16);
    frame(&mut runtime, b, now);
    let measured = *builds.borrow().last().unwrap();
    assert_eq!(measured.origin, Offset::new(20., 5.));
    assert_eq!(measured.size, b);
    assert!(builds.borrow().len() > count);
}
