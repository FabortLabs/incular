use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Offset, PointerDeviceKind as Device, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::internal::{
    PointerData, PointerDataChange as Change, PointerDataSignal as Signal,
};
use incular_widgets::{Center, InteractiveViewer, Listener, SizedBox, TransformationController};

const DPR: f64 = 2.;
const VIEW: u64 = 4;
const CENTER: Offset = Offset::new(400., 300.);

fn physical(value: Offset) -> Offset {
    let scale = DPR as f32;
    Offset::new(value.x * scale, value.y * scale)
}

struct Scene {
    runtime: Runtime,
    controller: TransformationController,
    time: Instant,
}

fn viewer(controller: &TransformationController) -> InteractiveViewer {
    InteractiveViewer::new(SizedBox::square(200.))
        .boundary_margin(EdgeInsets::all(50.))
        .transformation_controller(controller.clone())
}

impl Scene {
    fn new(
        controller: TransformationController,
        viewer: impl Into<incular_widgets::Widget>,
    ) -> Self {
        let mut scene = Self {
            runtime: Runtime::new(Center::new(viewer).into()).unwrap(),
            controller,
            time: Instant::now(),
        };
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.time += Duration::from_millis(17);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.time)
            .unwrap();
    }

    fn send(&mut self, kind: Device, change: Change, signal: Signal) {
        self.runtime.handle_pointer_data_packet(
            VIEW,
            DPR,
            [PointerData {
                view_id: VIEW,
                device: 13,
                pointer: 7,
                physical_position: physical(CENTER),
                kind,
                signal,
                ..PointerData::new(change, self.time)
            }],
        );
        self.frame();
    }

    fn scroll(&mut self, kind: Device, vector: Offset) {
        self.send(kind, Change::Hover, Signal::Scroll(physical(vector)));
    }

    fn state(&self) -> (f32, Offset) {
        let value = self.controller.value();
        (
            value.to_kurbo().as_coeffs()[0] as f32,
            value.translation_offset(),
        )
    }
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn mouse_wheel_zooms_but_trackpad_scroll_pans_in_both_axes() {
    let controller = TransformationController::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    let child = Listener::new(viewer(&controller)).on_pointer_signal(move |event| {
        recorded.borrow_mut().push(event);
    });
    let mut scene = Scene::new(controller, child);
    scene.scroll(Device::Mouse, Offset::new(300., -200.));
    assert_eq!(scene.state(), (2.5, Offset::new(-150., -150.)));
    scene.scroll(Device::Trackpad, Offset::new(100., -25.));
    assert_eq!(scene.state(), (2.5, Offset::new(-250., -125.)));
    scene.scroll(Device::Trackpad, Offset::new(-10., 0.));
    near(scene.state().1.x, -240.);

    let trace = events.borrow();
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[1].kind, Device::Trackpad);
    assert_eq!(trace[1].device, 13);
    assert_eq!(trace[1].position, CENTER);
    assert_eq!(trace[1].local_position, Offset::new(100., 100.));
    assert_eq!(trace[1].scroll_delta, Offset::new(100., -25.));
}

#[test]
fn trackpad_discrete_scroll_can_opt_into_zoom_without_horizontal_drift() {
    let controller = TransformationController::new();
    let widget = viewer(&controller).trackpad_scroll_causes_scale(true);
    let mut scene = Scene::new(controller, widget);
    scene.scroll(Device::Trackpad, Offset::new(0., -138.));
    let (scale, translation) = scene.state();
    near(scale, 1.993_715_5);
    near(translation.x, -99.371_55);
    near(translation.y, -99.371_55);
    let previous = scene.state();
    scene.scroll(Device::Trackpad, Offset::new(-138., 0.));
    assert_eq!(scene.state(), previous);
}

#[test]
fn trackpad_pan_zoom_stream_uses_absolute_scroll_to_scale_gain() {
    let controller = TransformationController::new();
    let widget = viewer(&controller).trackpad_scroll_causes_scale(true);
    let mut scene = Scene::new(controller, widget);
    scene.send(Device::Trackpad, Change::PanZoomStart, Signal::None);
    assert_eq!(scene.state().0, 1.);
    for (pan, delta) in [
        (Offset::new(0., -81.), Offset::new(0., -81.)),
        (Offset::new(81., -81.), Offset::new(81., 0.)),
    ] {
        scene.send(
            Device::Trackpad,
            Change::PanZoomUpdate {
                physical_pan: physical(pan),
                physical_pan_delta: physical(delta),
                scale: 1.,
                rotation: 0.,
            },
            Signal::None,
        );
        near(scene.state().0, 1.499_302_5);
    }
    scene.send(Device::Trackpad, Change::PanZoomEnd, Signal::None);
}

#[test]
fn callbacks_include_global_local_and_zero_velocity_when_wheel_or_pan_disabled() {
    let controller = TransformationController::new();
    let start = Rc::new(RefCell::new(Vec::new()));
    let update = Rc::new(RefCell::new(Vec::new()));
    let end = Rc::new(RefCell::new(Vec::new()));
    let child = viewer(&controller)
        .scale_enabled(false)
        .pan_enabled(false)
        .on_interaction_start({
            let out = start.clone();
            move |event| out.borrow_mut().push(event)
        })
        .on_interaction_update({
            let out = update.clone();
            move |event| out.borrow_mut().push(event)
        })
        .on_interaction_end({
            let out = end.clone();
            move |event| out.borrow_mut().push(event)
        });
    let mut scene = Scene::new(controller, child);
    scene.scroll(Device::Mouse, Offset::new(0., -20.));
    scene.scroll(Device::Trackpad, Offset::new(20., -10.));
    assert_eq!(scene.state(), (1., Offset::ZERO));
    assert_eq!(start.borrow().len(), 2);
    let updates = update.borrow();
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0].focal_point, CENTER);
    assert_eq!(updates[0].local_focal_point, Offset::new(100., 100.));
    near(updates[0].scale, 1.105_171);
    assert_eq!(updates[1].scale, 1.);
    assert_eq!(updates[1].local_focal_point, Offset::new(80., 110.));
    assert_eq!(updates[1].focal_point_delta, Offset::new(-20., 10.));
    assert_eq!(
        end.borrow()
            .iter()
            .map(|event| event.velocity.pixels_per_second)
            .collect::<Vec<_>>(),
        [Offset::ZERO; 2]
    );
}

#[test]
fn mouse_zoom_reports_start_scale_and_unchanged_pointer_scene_location() {
    let controller = TransformationController::new();
    let starts = Rc::new(RefCell::new(Vec::new()));
    let updates = Rc::new(RefCell::new(Vec::new()));
    let ends = Rc::new(RefCell::new(Vec::new()));
    let child = viewer(&controller)
        .on_interaction_start({
            let out = starts.clone();
            move |event| out.borrow_mut().push(event)
        })
        .on_interaction_update({
            let out = updates.clone();
            move |event| out.borrow_mut().push(event)
        })
        .on_interaction_end({
            let out = ends.clone();
            move |event| out.borrow_mut().push(event)
        });
    let mut scene = Scene::new(controller, child);
    scene.scroll(Device::Mouse, Offset::new(0., -20.));
    let [event] = *updates.borrow().as_slice() else {
        panic!("expected one update")
    };
    assert_eq!(starts.borrow().len(), 1);
    assert!(event.scale > 1.);
    near(scene.state().0, event.scale);
    assert_eq!(event.focal_point, CENTER);
    assert_eq!(event.local_focal_point, Offset::new(100., 100.));
    let transformed = scene.controller.to_scene(event.local_focal_point);
    near(transformed.x, 100.);
    near(transformed.y, 100.);
    assert_eq!(ends.borrow().len(), 1);
    assert_eq!(ends.borrow()[0].velocity.pixels_per_second, Offset::ZERO);
}
