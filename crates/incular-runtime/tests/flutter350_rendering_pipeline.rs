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

#[test]
fn changed_host_metrics_force_a_frame_even_without_environment_subscribers() {
    use incular_accessibility::MobileAccessibilityProjection;
    use incular_platform::{PhysicalSize, PlatformEvent, WindowEvent, WindowId, WindowMetrics};
    use incular_widgets::Semantics;

    let mut app = Application::new(|_| {
        Semantics::new(SizedBox::expand())
            .label("window root")
            .into()
    })
    .unwrap();
    let live = app.primary_window();
    let neighbor = app
        .open_window(WindowOptions::default(), SizedBox::expand().into())
        .unwrap()
        .id();
    let missing = WindowId::from_parts(999, 1);
    let first = WindowMetrics::new(PhysicalSize::new(200, 150), 1.);
    let second = WindowMetrics::new(PhysicalSize::new(800, 600), 2.);
    let metrics = |id, value| WindowEvent::platform(id, PlatformEvent::Metrics(value));
    app.handle_window_event(metrics(live, first));
    for window in [live, neighbor] {
        app.run_window_frame_at(
            window,
            Constraints::tight(first.logical_size()),
            Instant::now(),
        )
        .unwrap();
        assert!(!app.frame_requested(window));
    }
    let mut projection = MobileAccessibilityProjection::new();
    let view = app
        .sync_semantics_projection(live, &mut projection)
        .unwrap();
    assert_eq!(view.nodes[0].semantic.bounds.size, Size::new(200., 150.));

    app.handle_window_event(metrics(missing, second));
    app.handle_window_event(metrics(live, first));
    assert!(!app.frame_requested(live));
    assert!(!app.frame_requested(neighbor));
    assert!(!app.frame_requested(missing));

    app.handle_window_event(metrics(live, second));
    assert!(app.frame_requested(live));
    assert!(!app.frame_requested(neighbor));
    app.run_window_frame_at(
        live,
        Constraints::tight(second.logical_size()),
        Instant::now(),
    )
    .unwrap();
    let resized = app
        .sync_semantics_projection(live, &mut projection)
        .unwrap();
    assert_eq!(resized.nodes[0].semantic.bounds.size, Size::new(400., 300.));
    assert!(!app.frame_requested(live));

    // Logical dimensions stay unchanged, but device pixels still change.
    app.handle_window_event(metrics(
        live,
        WindowMetrics::new(PhysicalSize::new(400, 300), 1.),
    ));
    assert!(app.frame_requested(live));
    assert!(!app.frame_requested(neighbor));
}

#[test]
fn semantics_diagnostics_explain_missing_trees_once_and_report_collected_labels() {
    use incular_widgets::Semantics;

    let mut app = Application::new(|_| {
        Semantics::new(SizedBox::expand())
            .label("primary label")
            .into()
    })
    .unwrap();
    let primary = app.primary_window();
    let secondary = app
        .open_window(
            WindowOptions::default(),
            Semantics::new(SizedBox::expand())
                .label("secondary label")
                .into(),
        )
        .unwrap()
        .id();
    let bounds = Constraints::tight(Size::new(400., 300.));
    let missing = "Semantics not generated";
    let help = "assistive technology such as a screen reader";
    for window in [primary, secondary] {
        app.set_semantics_enabled(window, false);
        app.run_window_frame_at(window, bounds, Instant::now())
            .unwrap();
    }

    let dump = app.debug_dump_semantics();
    assert!(dump.starts_with(missing), "{dump}");
    assert_eq!(dump.matches(missing).count(), 2);
    assert_eq!(dump.matches(help).count(), 1);
    assert!(dump.contains("application semantics handle"));
    assert!(!dump.contains("primary label"));
    assert!(!app.frame_requested(primary));
    assert!(!app.frame_requested(secondary));

    let request = app.ensure_semantics(primary).unwrap();
    app.run_window_frame_at(primary, bounds, Instant::now())
        .unwrap();
    let partial = app.debug_dump_semantics();
    assert_eq!(partial.matches(missing).count(), 1);
    assert_eq!(partial.matches(help).count(), 1);
    assert!(partial.contains("primary label"));
    assert!(!partial.contains("secondary label"));

    app.set_semantics_enabled(secondary, true);
    app.run_window_frame_at(secondary, bounds, Instant::now())
        .unwrap();
    let collected = app.debug_dump_semantics();
    assert!(collected.contains("primary label") && collected.contains("secondary label"));
    assert!(!collected.contains(missing));
    assert!(!collected.contains(help));

    drop(request);
    app.run_window_frame_at(primary, bounds, Instant::now())
        .unwrap();
    let released = app.debug_dump_semantics();
    assert_eq!(released.matches(missing).count(), 1);
    assert_eq!(released.matches(help).count(), 1);
    assert!(app.close_window(secondary));
    assert!(app.close_window(primary));
    assert_eq!(
        app.debug_dump_semantics(),
        "No render tree root is registered."
    );
}
