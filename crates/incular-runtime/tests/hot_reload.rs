//! Reassembling re-runs the retained root builder of every window without a
//! signal write, which is what makes freshly patched code visible.
#![cfg(feature = "hot-reload")]

use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{WindowId, WindowOptions};
use incular_runtime::{Application, BuildContext, Signal};
use incular_widgets::{Text, Widget};
use std::{cell::Cell, rc::Rc, time::Instant};

fn frame(application: &mut Application, window: WindowId) {
    application
        .run_window_frame_at(
            window,
            Constraints::tight(Size::new(120.0, 80.0)),
            Instant::now(),
        )
        .expect("frame");
}

fn counting_root(builds: &Rc<Cell<u32>>) -> impl FnMut(&mut BuildContext) -> Widget + 'static {
    let builds = builds.clone();
    move |_| {
        builds.set(builds.get() + 1);
        Text::new("root").into()
    }
}

#[test]
fn reassemble_rebuilds_every_window_root_once() {
    let primary_builds = Rc::new(Cell::new(0));
    let secondary_builds = Rc::new(Cell::new(0));
    let mut application = Application::new(counting_root(&primary_builds)).expect("application");
    let primary = application.primary_window();
    let secondary = application
        .open_window_with(WindowOptions::default(), counting_root(&secondary_builds))
        .expect("secondary window")
        .id();
    frame(&mut application, primary);
    frame(&mut application, secondary);
    let (primary_before, secondary_before) = (primary_builds.get(), secondary_builds.get());

    // Without demand a frame does not rebuild.
    frame(&mut application, primary);
    assert_eq!(primary_builds.get(), primary_before);

    application.reassemble();
    assert!(application.frame_requested(primary));
    assert!(application.frame_requested(secondary));
    frame(&mut application, primary);
    frame(&mut application, secondary);
    assert_eq!(primary_builds.get(), primary_before + 1);
    assert_eq!(secondary_builds.get(), secondary_before + 1);

    frame(&mut application, primary);
    assert_eq!(primary_builds.get(), primary_before + 1);
}

#[test]
fn reassemble_keeps_signal_state_and_dependencies() {
    let count = Signal::new(3_u32);
    let seen = Rc::new(Cell::new(0_u32));
    let (root_count, root_seen) = (count.clone(), seen.clone());
    let mut application = Application::new(move |_| {
        root_seen.set(root_count.get());
        Text::new("root").into()
    })
    .expect("application");
    let window = application.primary_window();
    frame(&mut application, window);

    application.reassemble();
    frame(&mut application, window);
    assert_eq!(seen.get(), 3);

    // The rebuilt root is still subscribed to the signal it read.
    count.set(4);
    frame(&mut application, window);
    assert_eq!(seen.get(), 4);
}
