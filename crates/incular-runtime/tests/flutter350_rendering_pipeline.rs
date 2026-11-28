//! Focused mounted counterparts of the Flutter rendering binding tests.
use incular_config::Constraints;
use incular_core::{Size, time::Instant};
use incular_platform::WindowOptions;
use incular_rendering::DisplayList;
use incular_runtime::{Application, RuntimeWake};
use incular_widgets::{
    Center, CustomPaint, CustomPainter, CustomSingleChildLayout, LayoutRelayout,
    SingleChildLayoutDelegate, SizedBox,
};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

struct Delegate {
    size: Rc<Cell<Size>>,
    relayout: LayoutRelayout,
}

impl SingleChildLayoutDelegate for Delegate {
    fn get_size(&self, _: Constraints) -> Size {
        self.size.get()
    }

    fn get_constraints_for_child(&self, _: Constraints) -> Constraints {
        Constraints::tight(self.size.get())
    }

    fn relayout(&self) -> Option<LayoutRelayout> {
        Some(self.relayout.clone())
    }

    fn should_relayout(&self, _: &Self) -> bool {
        false
    }
}

struct Recorder(Rc<Cell<Size>>);

impl CustomPainter for Recorder {
    fn paint(&self, size: Size) -> DisplayList {
        self.0.set(size);
        DisplayList::default()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

#[derive(Default)]
struct Wakes(AtomicUsize);

impl RuntimeWake for Wakes {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn a_mounted_child_view_layout_invalidation_wakes_the_host_once() {
    let mut app = Application::new(|_| SizedBox::expand().into()).unwrap();
    let primary = app.primary_window();
    let size = Rc::new(Cell::new(Size::new(40., 50.)));
    let painted = Rc::new(Cell::new(Size::ZERO));
    let builds = Rc::new(Cell::new(0));
    let relayout = LayoutRelayout::default();
    let window = app
        .open_window_with(WindowOptions::default(), {
            let (size, painted, builds, relayout) = (
                size.clone(),
                painted.clone(),
                builds.clone(),
                relayout.clone(),
            );
            move |_| {
                builds.set(builds.get() + 1);
                Center::new(CustomSingleChildLayout::new(
                    Delegate {
                        size: size.clone(),
                        relayout: relayout.clone(),
                    },
                    CustomPaint::from_painter(Size::ZERO, Recorder(painted.clone())),
                ))
                .into()
            }
        })
        .unwrap()
        .id();
    let constraints = Constraints::tight(Size::new(400., 300.));
    for id in [primary, window] {
        app.run_window_frame_at(id, constraints, Instant::now())
            .unwrap();
        assert!(!app.frame_requested(id));
    }
    assert_eq!(painted.get(), Size::new(40., 50.));
    app.process_runtime_work();
    let wakes = Arc::new(Wakes::default());
    app.set_wake_handler(wakes.clone());
    let initial_wakes = wakes.0.load(Ordering::Relaxed);

    size.set(Size::new(80., 90.));
    relayout.notify();
    assert!(app.frame_requested(window));
    assert!(!app.frame_requested(primary));
    assert_eq!(wakes.0.load(Ordering::Relaxed), initial_wakes + 1);
    size.set(Size::new(100., 120.));
    relayout.notify();
    assert_eq!(wakes.0.load(Ordering::Relaxed), initial_wakes + 1);

    app.process_runtime_work();
    app.run_window_frame_at(window, constraints, Instant::now())
        .unwrap();
    assert_eq!(painted.get(), Size::new(100., 120.));
    assert_eq!(builds.get(), 1, "a render-only dependency must not rebuild");
    assert!(!app.frame_requested(window));

    size.set(Size::new(120., 140.));
    relayout.notify();
    assert_eq!(wakes.0.load(Ordering::Relaxed), initial_wakes + 2);
    app.process_runtime_work();
    app.run_window_frame_at(window, constraints, Instant::now())
        .unwrap();
    assert_eq!(painted.get(), Size::new(120., 140.));
    assert_eq!(builds.get(), 1);
    assert!(!app.frame_requested(primary));

    assert!(app.close_window(window));
    app.process_runtime_work();
    let wakes_after_close = wakes.0.load(Ordering::Relaxed);
    relayout.notify();
    assert_eq!(wakes.0.load(Ordering::Relaxed), wakes_after_close);
    assert!(!app.frame_requested(primary));
}
