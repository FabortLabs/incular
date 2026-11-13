//! Shared scenario hosts that need dev-dependencies the harness library may
//! not use (see the allow-list in `specs/architecture.json`).

use std::cell::RefCell;
use std::rc::Rc;

use incular_navigation::Navigator;
use incular_parity_harness::{FrameHarness, surface_constraints};
use incular_runtime::RouteOutlet;
use incular_widgets::{SizedBox, Widget};

type OutletSlot = Rc<RefCell<Option<Rc<RefCell<RouteOutlet>>>>>;

/// Mounts `navigator` the way a production window hosts it: the scene is
/// `build(phase, outlet)`, where `outlet` is the navigator's route outlet
/// widget, and every recorded frame (frame 0 included) is presented through
/// `RouteOutlet::present_frame_at`. The Flutter twin mounts a `Navigator` in
/// the same place.
///
/// Creating and attaching the outlet takes one unrecorded frame at time
/// zero; frame 0, also at time zero, is then presented and recorded.
pub fn mount_navigator(
    h: &mut FrameHarness,
    navigator: &Navigator,
    build: impl Fn(u32, Widget) -> Widget + 'static,
) {
    let slot: OutletSlot = Rc::default();
    let driver_slot = Rc::clone(&slot);
    h.set_frame_driver(move |runtime, constraints, now| {
        let outlet = driver_slot.borrow().clone().expect("outlet attached");
        RouteOutlet::present_frame_at(&outlet, runtime, constraints, now).expect("present frame");
    });
    let scene_slot = Rc::clone(&slot);
    let navigator = navigator.clone();
    h.mount_with_setup(
        move |phase| {
            let outlet = scene_slot
                .borrow()
                .as_ref()
                .map_or_else(|| SizedBox::shrink().into(), |o| o.borrow().widget());
            build(phase, outlet)
        },
        move |runtime, now| {
            let parent = runtime.spawner().scope();
            let outlet = Rc::new(RefCell::new(RouteOutlet::new(&navigator, &parent)));
            *slot.borrow_mut() = Some(Rc::clone(&outlet));
            runtime
                .run_frame_at(surface_constraints(), now)
                .expect("mount outlet");
            RouteOutlet::attach(&outlet, runtime).expect("outlet mounted");
        },
    );
}
