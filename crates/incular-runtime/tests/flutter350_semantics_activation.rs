use incular_accessibility::MobileAccessibilityProjection;
use incular_config::Constraints;
use incular_core::{Size, time::Instant};
use incular_runtime::{Application, RuntimeWake};
use incular_widgets::{Semantics, SizedBox};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn app() -> Application {
    Application::new(|_| {
        Semantics::new(SizedBox::new().width(60.).height(50.))
            .label("target")
            .into()
    })
    .unwrap()
}

#[test]
fn two_handles_keep_semantics_enabled_across_a_platform_toggle() {
    let mut app = app();
    let window = app.primary_window();
    app.set_semantics_enabled(window, false);
    let events = Rc::new(RefCell::new(Vec::new()));
    let _listener = app
        .on_semantics_enabled_changed(window, {
            let events = events.clone();
            move |enabled| events.borrow_mut().push(enabled)
        })
        .unwrap();
    assert_eq!(app.semantics_enabled(window), Some(false));

    let first = app.ensure_semantics(window).unwrap();
    assert_eq!(events.borrow().as_slice(), [true]);
    assert_eq!(app.semantics_enabled(window), Some(true));
    events.borrow_mut().clear();

    let second = app.ensure_semantics(window).unwrap();
    assert!(events.borrow().is_empty());
    app.set_semantics_enabled(window, true);
    app.set_semantics_enabled(window, false);
    assert!(events.borrow().is_empty());
    drop(first);
    assert_eq!(app.semantics_enabled(window), Some(true));
    assert!(events.borrow().is_empty());
    drop(second);
    assert_eq!(app.semantics_enabled(window), Some(false));
    assert_eq!(events.borrow().as_slice(), [false]);
}

#[test]
fn a_native_accessibility_request_survives_a_temporary_application_handle() {
    let mut app = app();
    let window = app.primary_window();
    app.set_semantics_enabled(window, false);
    let events = Rc::new(RefCell::new(Vec::new()));
    let listener = app
        .on_semantics_enabled_changed(window, {
            let events = events.clone();
            move |enabled| events.borrow_mut().push(enabled)
        })
        .unwrap();
    assert_eq!(app.semantics_enabled(window), Some(false));
    app.set_semantics_enabled(window, true);
    assert_eq!(app.semantics_enabled(window), Some(true));
    assert_eq!(events.borrow().as_slice(), [true]);
    events.borrow_mut().clear();

    drop(app.ensure_semantics(window).unwrap());
    assert_eq!(app.semantics_enabled(window), Some(true));
    assert!(events.borrow().is_empty());
    app.set_semantics_enabled(window, false);
    assert_eq!(app.semantics_enabled(window), Some(false));
    assert_eq!(events.borrow().as_slice(), [false]);
    drop(listener);
    app.set_semantics_enabled(window, true);
    assert_eq!(events.borrow().as_slice(), [false]);
}

#[derive(Default)]
struct WakeCount(AtomicUsize);

impl RuntimeWake for WakeCount {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn an_application_handle_publishes_the_tree_and_last_drop_wakes_to_release_it() {
    let mut app = app();
    let window = app.primary_window();
    app.set_semantics_enabled(window, false);
    let wake = Arc::new(WakeCount::default());
    app.set_wake_handler(wake.clone());
    let constraints = Constraints::tight(Size::new(400., 300.));
    let mut projection = MobileAccessibilityProjection::new();
    app.run_window_frame_at(window, constraints, Instant::now())
        .unwrap()
        .unwrap();
    if let Some(update) = app.sync_semantics_projection(window, &mut projection) {
        assert!(update.nodes.is_empty());
    }
    let handle = app.ensure_semantics(window).unwrap();
    assert!(app.frame_requested(window));
    app.run_window_frame_at(window, constraints, Instant::now())
        .unwrap()
        .unwrap();
    let updated = app
        .sync_semantics_projection(window, &mut projection)
        .unwrap();
    let native = updated
        .nodes
        .iter()
        .find(|node| node.semantic.label.as_deref() == Some("target"))
        .unwrap()
        .native_id;
    assert!(!app.frame_requested(window));

    let before = wake.0.load(Ordering::Relaxed);
    drop(handle);
    assert_eq!(app.semantics_enabled(window), Some(false));
    assert!(wake.0.load(Ordering::Relaxed) > before);
    assert!(app.frame_requested(window));
    app.run_window_frame_at(window, constraints, Instant::now())
        .unwrap()
        .unwrap();
    let cleared = app
        .sync_semantics_projection(window, &mut projection)
        .unwrap();
    assert!(cleared.nodes.is_empty());
    assert!(cleared.removed.contains(&native));
    assert!(!app.frame_requested(window));
}

#[test]
fn handles_and_listeners_are_local_to_the_original_window_generation() {
    let mut app = app();
    let primary = app.primary_window();
    let second = app
        .open_window(
            incular_platform::WindowOptions::default(),
            SizedBox::shrink().into(),
        )
        .unwrap()
        .id();
    app.set_semantics_enabled(primary, false);
    app.set_semantics_enabled(second, false);
    let second_handle = app.ensure_semantics(second).unwrap();
    assert_eq!(app.semantics_enabled(primary), Some(false));
    assert_eq!(app.semantics_enabled(second), Some(true));
    assert!(app.close_window(second));
    assert!(app.ensure_semantics(second).is_none());

    let replacement = app
        .open_window(
            incular_platform::WindowOptions::default(),
            SizedBox::shrink().into(),
        )
        .unwrap()
        .id();
    assert_eq!(replacement.index(), second.index());
    assert_ne!(replacement.generation(), second.generation());
    app.set_semantics_enabled(replacement, false);
    let new_handle = app.ensure_semantics(replacement).unwrap();
    drop(second_handle);
    assert_eq!(app.semantics_enabled(replacement), Some(true));
    drop(new_handle);
    assert_eq!(app.semantics_enabled(replacement), Some(false));
}
