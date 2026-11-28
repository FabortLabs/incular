use incular_accessibility::MobileAccessibilityProjection;
use incular_config::Constraints;
use incular_core::{Size, time::Instant};
use incular_runtime::{Application, Runtime, RuntimeWake};
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
fn initializing_application_with_existing_semantics_demand_publishes_on_first_frame() {
    let mut runtime =
        Runtime::new(Semantics::new(SizedBox::expand()).label("startup").into()).unwrap();
    runtime.set_platform_semantics_enabled(false);
    let startup_demand = runtime.ensure_semantics();
    let mut app = Application::from_runtime(runtime, |_| {});
    let primary = app.primary_window();
    let replay = Rc::new(RefCell::new(Vec::new()));
    let _binding = app
        .bind_semantics_tree_enabled(primary, {
            let replay = replay.clone();
            move |enabled| replay.borrow_mut().push(enabled)
        })
        .unwrap();
    assert_eq!(*replay.borrow(), [true]);
    assert_eq!(app.semantics_enabled(primary), Some(true));

    let constraints = Constraints::tight(Size::new(400., 300.));
    let mut projection = MobileAccessibilityProjection::new();
    app.run_window_frame_at(primary, constraints, Instant::now())
        .unwrap()
        .unwrap();
    let first = app
        .sync_semantics_projection(primary, &mut projection)
        .unwrap();
    let [node] = first.nodes.as_slice() else {
        panic!("only the mounted startup label should be published");
    };
    assert_eq!(node.semantic.label.as_deref(), Some("startup"));
    let native_id = node.native_id;
    assert!(!app.frame_requested(primary));

    let wake = Arc::new(WakeCount::default());
    app.set_wake_handler(wake.clone());
    drop(startup_demand);
    assert_eq!(*replay.borrow(), [true, false]);
    assert_eq!(app.semantics_enabled(primary), Some(false));
    assert!(app.frame_requested(primary));
    assert!(wake.0.load(Ordering::Relaxed) > 0);

    app.run_window_frame_at(primary, constraints, Instant::now())
        .unwrap()
        .unwrap();
    let cleared = app
        .sync_semantics_projection(primary, &mut projection)
        .unwrap();
    assert!(cleared.nodes.is_empty());
    assert!(cleared.removed.contains(&native_id));
}

#[test]
fn native_pre_frame_activation_replays_without_enabling_a_neighbor_window() {
    let mut app = app();
    let primary = app.primary_window();
    let second = app
        .open_window(
            incular_platform::WindowOptions::default(),
            Semantics::new(SizedBox::expand()).label("neighbor").into(),
        )
        .unwrap()
        .id();
    // Native hosts opt out until their respective adapters request a tree.
    app.set_semantics_enabled(primary, false);
    app.set_semantics_enabled(second, false);
    app.set_semantics_enabled(primary, true);
    let events = Rc::new(RefCell::new(Vec::new()));
    let _first_binding = app
        .bind_semantics_tree_enabled(primary, {
            let events = events.clone();
            move |enabled| events.borrow_mut().push(("primary", enabled))
        })
        .unwrap();
    let _second_binding = app
        .bind_semantics_tree_enabled(second, {
            let events = events.clone();
            move |enabled| events.borrow_mut().push(("second", enabled))
        })
        .unwrap();
    assert_eq!(*events.borrow(), [("primary", true), ("second", false)]);

    let constraints = Constraints::tight(Size::new(400., 300.));
    for (window, label) in [(primary, Some("target")), (second, None)] {
        app.run_window_frame_at(window, constraints, Instant::now())
            .unwrap()
            .unwrap();
        let mut projection = MobileAccessibilityProjection::new();
        let update = app.sync_semantics_projection(window, &mut projection);
        let labels: Vec<_> = update
            .iter()
            .flat_map(|update| &update.nodes)
            .filter_map(|node| node.semantic.label.as_deref())
            .collect();
        assert_eq!(labels, label.into_iter().collect::<Vec<_>>());
    }
    assert_eq!(*events.borrow(), [("primary", true), ("second", false)]);
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

#[cfg(debug_assertions)]
#[test]
fn semantics_handle_emits_one_creation_and_disposal_for_the_same_identity() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};

    let mut app = app();
    let owner = app.primary_window();
    app.set_semantics_enabled(owner, false);
    let events = Rc::new(RefCell::new(Vec::new()));
    let _subscription = MemoryAllocations::current().add_listener({
        let events = events.clone();
        move |event| {
            if event.class_name == "SemanticsHandle" {
                events.borrow_mut().push(event);
            }
        }
    });
    {
        let handle = app.ensure_semantics(owner).unwrap();
        assert_eq!(events.borrow().len(), 1);
        assert_eq!(events.borrow()[0].kind, ObjectEventKind::Created);
        assert!(app.semantics_enabled(owner).unwrap());
        drop(handle);
    }
    assert!(!app.semantics_enabled(owner).unwrap());
    let events = events.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].library, "semantics");
    assert_eq!(events[1].kind, ObjectEventKind::Disposed);
    assert_eq!(events[0].id, events[1].id);
}

#[test]
fn restarting_application_semantics_republishes_the_same_label_without_remounting() {
    use incular_config::TextDirection;

    let mut app = Application::new(|_| {
        Semantics::new(SizedBox::expand())
            .label("test1")
            .text_direction(TextDirection::Ltr)
            .into()
    })
    .unwrap();
    let window = app.primary_window();
    app.set_semantics_enabled(window, false);
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut projection = MobileAccessibilityProjection::new();
    for pass in 0..2 {
        let request = app.ensure_semantics(window).unwrap();
        assert!(app.frame_requested(window));
        app.run_window_frame_at(window, bounds, Instant::now())
            .unwrap();
        let published = app
            .sync_semantics_projection(window, &mut projection)
            .unwrap();
        let [only] = published.nodes.as_slice() else {
            panic!("one semantics label")
        };
        assert_eq!(only.semantic.label.as_deref(), Some("test1"));
        assert_eq!(only.semantic.state.text_direction, Some(TextDirection::Ltr));
        assert!(!app.frame_requested(window));

        drop(request);
        assert_eq!(app.semantics_enabled(window), Some(false));
        // Incular's native host queues the snapshot removal on a frame even
        // though Flutter's release does not itself schedule a render frame.
        assert!(app.frame_requested(window));
        app.run_window_frame_at(window, bounds, Instant::now())
            .unwrap();
        let cleared = app
            .sync_semantics_projection(window, &mut projection)
            .unwrap();
        assert!(cleared.nodes.is_empty());
        assert!(cleared.removed.contains(&only.native_id));
        assert!(!app.frame_requested(window), "pass {pass}");
    }
}

#[test]
fn global_semantics_handle_covers_existing_and_future_window_roots() {
    let mut app = app();
    let primary = app.primary_window();
    let open = |app: &mut Application, label: &'static str| {
        let id = app
            .open_window(
                incular_platform::WindowOptions::default(),
                Semantics::new(SizedBox::expand()).label(label).into(),
            )
            .unwrap()
            .id();
        app.set_semantics_enabled(id, false);
        id
    };
    app.set_semantics_enabled(primary, false);
    let second = open(&mut app, "second");
    let bounds = Constraints::tight(Size::new(400., 300.));
    for id in [primary, second] {
        assert_eq!(app.semantics_enabled(id), Some(false));
        app.run_window_frame_at(id, bounds, Instant::now()).unwrap();
        assert!(!app.frame_requested(id));
    }

    let events = Rc::new(RefCell::new(Vec::new()));
    let _listener = app
        .on_semantics_enabled_changed(primary, {
            let events = events.clone();
            move |enabled| events.borrow_mut().push(enabled)
        })
        .unwrap();
    let first = app.ensure_global_semantics();
    let last = app.ensure_global_semantics();
    let late = open(&mut app, "late");
    let mut projections = [
        (primary, "target", MobileAccessibilityProjection::new()),
        (second, "second", MobileAccessibilityProjection::new()),
        (late, "late", MobileAccessibilityProjection::new()),
    ];
    drop(first);
    assert_eq!(*events.borrow(), [true]);
    for (id, label, projection) in &mut projections {
        assert_eq!(app.semantics_enabled(*id), Some(true));
        assert!(app.frame_requested(*id));
        app.run_window_frame_at(*id, bounds, Instant::now())
            .unwrap();
        let update = app.sync_semantics_projection(*id, projection).unwrap();
        let [node] = update.nodes.as_slice() else {
            panic!("exactly one label on each independently presented view");
        };
        assert_eq!(node.semantic.label.as_deref(), Some(*label));
        assert!(!app.frame_requested(*id));
    }

    // Retiring a view and reusing its slot while the handle is live must
    // attach the new generation, not keep the old projection in the scope.
    assert!(app.close_window(second));
    let replacement = open(&mut app, "replacement");
    assert_eq!(replacement.index(), second.index());
    assert_ne!(replacement.generation(), second.generation());
    assert_eq!(app.semantics_enabled(second), None);
    assert_eq!(app.semantics_enabled(replacement), Some(true));

    let local = app.ensure_semantics(late).unwrap();
    app.process_runtime_work();
    let wake = Arc::new(WakeCount::default());
    app.set_wake_handler(wake.clone());
    let before_release = wake.0.load(Ordering::Relaxed);
    drop(last);
    assert!(wake.0.load(Ordering::Relaxed) > before_release);
    assert_eq!(*events.borrow(), [true, false]);
    assert_eq!(app.semantics_enabled(replacement), Some(false));
    assert_eq!(app.semantics_enabled(primary), Some(false));
    assert!(app.frame_requested(primary));
    assert_eq!(app.semantics_enabled(late), Some(true));
    assert!(!app.frame_requested(late));
    app.run_window_frame_at(primary, bounds, Instant::now())
        .unwrap();
    let removed = app
        .sync_semantics_projection(primary, &mut projections[0].2)
        .unwrap();
    assert!(removed.nodes.is_empty());
    assert_eq!(removed.removed.len(), 1);
    drop(local);
    assert_eq!(app.semantics_enabled(late), Some(false));
    assert!(app.frame_requested(late));
}

#[test]
fn a_release_listener_can_restore_global_demand_without_stale_notifications() {
    let mut app = app();
    let first = app.primary_window();
    let second = app
        .open_window(
            incular_platform::WindowOptions::default(),
            SizedBox::expand().into(),
        )
        .unwrap()
        .id();
    app.set_semantics_enabled(first, false);
    app.set_semantics_enabled(second, false);
    let app = Rc::new(RefCell::new(app));
    let rebound = Rc::new(RefCell::new(None));
    let restore_on_release = app
        .borrow_mut()
        .on_semantics_enabled_changed(first, {
            let app = Rc::downgrade(&app);
            let rebound = rebound.clone();
            move |enabled| {
                if !enabled {
                    let app = app.upgrade().unwrap();
                    assert_eq!(app.borrow().semantics_enabled(second), Some(false));
                    *rebound.borrow_mut() = Some(app.borrow_mut().ensure_global_semantics());
                }
            }
        })
        .unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let subscriptions = [first, second].map(|id| {
        app.borrow_mut()
            .on_semantics_enabled_changed(id, {
                let events = events.clone();
                move |enabled| events.borrow_mut().push((id, enabled))
            })
            .unwrap()
    });
    let original = app.borrow_mut().ensure_global_semantics();
    assert_eq!(*events.borrow(), [(first, true), (second, true)]);
    events.borrow_mut().clear();
    drop(original);
    assert!(events.borrow().is_empty());
    assert_eq!(app.borrow().semantics_enabled(first), Some(true));
    assert_eq!(app.borrow().semantics_enabled(second), Some(true));

    drop(restore_on_release);
    drop(rebound.borrow_mut().take().unwrap());
    assert_eq!(*events.borrow(), [(first, false), (second, false)]);
    drop(subscriptions);
}
