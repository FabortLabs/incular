//! A signal written while a builder runs does not close the application.
//! Like Flutter's `setState` during build, the write is reported; the value
//! changes at once and its dependents rebuild in the next frame.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::{Application, Signal};
use incular_widgets::{SizedBox, Widget};
use std::{cell::Cell, rc::Rc, time::Instant};

fn frame(app: &mut Application) {
    let id = app.primary_window();
    app.run_window_frame_at(
        id,
        Constraints::tight(Size::new(100., 100.)),
        Instant::now(),
    )
    .expect("frame");
}

#[test]
fn a_write_during_build_is_deferred_instead_of_panicking() {
    let route = Signal::new(0_u32);
    let tooltip = Signal::new(true);
    let seen = Rc::new(Cell::new(true));
    let mut app = Application::new({
        let (route, tooltip, seen) = (route.clone(), tooltip.clone(), seen.clone());
        move |_| {
            // A stray write: hide the tooltip when the route changes.
            if route.get() == 1 {
                tooltip.set(false);
            }
            seen.set(tooltip.get());
            Widget::from(SizedBox::new().width(10.).height(10.))
        }
    })
    .expect("application");
    frame(&mut app);
    assert!(seen.get());
    route.set(1);
    frame(&mut app);
    assert!(!tooltip.get_untracked(), "the write applies at once");
    frame(&mut app);
    assert!(!seen.get(), "dependents see the write by the next frame");
    let id = app.primary_window();
    assert!(
        app.run_window_frame_at(
            id,
            Constraints::tight(Size::new(100., 100.)),
            Instant::now()
        )
        .is_ok(),
        "the application keeps running"
    );
}
