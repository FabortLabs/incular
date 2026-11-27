//! Headless restoration data and dirty scheduling comparisons against pinned
//! Flutter 3.50.0. Incular's application store is not Flutter's engine wire.

use std::{cell::RefCell, rc::Rc, sync::Arc, thread, time::Duration};

use incular_animation::{add_post_frame_callback, schedule_frame};
use incular_config::Constraints;
use incular_core::{Color, RestorationKey, RestorationScope, Size, time::Instant};
use incular_platform::{PhysicalSize, PlatformEvent, WindowEvent, WindowMetrics, WindowOptions};
use incular_runtime::{
    Application, FRAMEWORK_RESTORATION_FORMAT_VERSION, InMemoryRestorationStore, RestorationConfig,
    RestorationHandle,
};
use incular_widgets::{LayoutBuilder, Widget};
use serde_json::{Value, json};

const APP_ID: &str = "org.incular.flutter350.restoration-dirty-flush";

fn key(id: &str) -> RestorationKey {
    RestorationKey::new(id).unwrap()
}

struct Fixture {
    app: Application,
    root: RestorationScope,
    handle: RestorationHandle,
    store: Arc<InMemoryRestorationStore>,
}

impl Fixture {
    fn new() -> Self {
        let store = Arc::new(InMemoryRestorationStore::new());
        store.set_bytes(Some(
            serde_json::to_vec(&json!({
                "framework_format_version": FRAMEWORK_RESTORATION_FORMAT_VERSION,
                "application_schema_version": 1,
                "snapshot_generation": 0,
                "application_id": APP_ID,
                "state": {"application/child1/foo": 22},
                "windows": [],
            }))
            .unwrap(),
        ));
        Self::open(store)
    }

    fn open(store: Arc<InMemoryRestorationStore>) -> Self {
        let captured = Rc::new(RefCell::new(None));
        let slot = captured.clone();
        let app = Application::new_with_restoration(
            WindowOptions::default(),
            RestorationConfig::new(APP_ID, 1, store.clone()).with_debounce(Duration::from_secs(60)),
            move |ctx| {
                *slot.borrow_mut() =
                    Some((ctx.restoration_scope().unwrap(), ctx.restoration().unwrap()));
                Widget::box_(Size::new(1., 1.), Color::WHITE)
            },
        )
        .unwrap();
        let (root, handle) = captured.borrow_mut().take().unwrap();
        Self {
            app,
            root,
            handle,
            store,
        }
    }

    fn frame(&mut self) {
        let id = self.app.primary_window();
        self.app
            .run_window_frame_at(
                id,
                Constraints::tight(Size::new(100., 100.)),
                Instant::now(),
            )
            .unwrap()
            .expect("headless frame");
    }

    fn drain_save(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.app.process_runtime_work();
            let diagnostics = self.handle.diagnostics();
            assert_eq!(diagnostics.last_save_error, None);
            if diagnostics.dirty_generation == diagnostics.persisted_generation {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "explicit store save did not finish"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn saved_state(&self, path: &str) -> Option<Value> {
        self.document()["state"].get(path).cloned()
    }

    fn document(&self) -> Value {
        serde_json::from_slice(&self.store.bytes().unwrap()).unwrap()
    }

    fn saved_scope(&self, path: &str) -> bool {
        self.document()["claimed_scopes"]
            .as_array()
            .is_some_and(|scopes| scopes.iter().any(|scope| scope.as_str() == Some(path)))
    }
}

// services/restoration_bucket_test.dart:147 state and dirty subset.
#[test]
fn claiming_an_existing_child_loads_values_without_requesting_a_save() {
    let mut fixture = Fixture::new();
    let requests = fixture.handle.diagnostics().save_requests;
    let child = fixture.root.child_unchecked(key("child1"));
    let _lease = fixture.handle.claim_scope(&child).unwrap();
    assert_eq!(fixture.handle.diagnostics().save_requests, requests);
    assert_eq!(child.get_json(&key("foo")), Some(json!(22)));

    child.set_json(&key("bar"), json!(44));
    assert_eq!(fixture.handle.diagnostics().save_requests, requests + 1);
    fixture.handle.flush();
    fixture.drain_save();
    assert_eq!(
        fixture.saved_state("application/child1/bar"),
        Some(json!(44))
    );
}

// services/restoration_bucket_test.dart:172. The local snapshot persists
// the shape of an empty bucket without inventing an application value.
#[test]
fn claiming_a_new_child_requests_a_save_before_its_first_value_write() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.saved_state("application/child2/foo"), None);
    let requests = fixture.handle.diagnostics().save_requests;
    let child = fixture.root.child_unchecked(key("child2"));
    let lease = fixture.handle.claim_scope(&child).unwrap();
    assert_eq!(fixture.handle.diagnostics().save_requests, requests + 1);
    fixture.handle.flush();
    fixture.drain_save();
    assert!(fixture.saved_scope("application/child2"));
    assert_eq!(fixture.saved_state("application/child2/foo"), None);

    let store = fixture.store.clone();
    drop(lease);
    fixture.app.shutdown();
    drop(fixture);

    let mut fixture = Fixture::open(store);
    let child = fixture.root.child_unchecked(key("child2"));
    let requests = fixture.handle.diagnostics().save_requests;
    let _lease = fixture.handle.claim_scope(&child).unwrap();
    assert_eq!(fixture.handle.diagnostics().save_requests, requests);
    child.set_json(&key("foo"), json!(55));
    assert_eq!(child.get_json(&key("foo")), Some(json!(55)));
    fixture.handle.flush();
    fixture.drain_save();
    assert_eq!(
        fixture.saved_state("application/child2/foo"),
        Some(json!(55))
    );
}

#[test]
fn removing_empty_bucket_metadata_keeps_neighboring_scope_paths() {
    let mut fixture = Fixture::new();
    let parent = fixture.root.child_unchecked(key("new"));
    let nested = parent.child_unchecked(key("empty"));
    let sibling = fixture.root.child_unchecked(key("new-sibling"));
    let _nested = fixture.handle.claim_scope(&nested).unwrap();
    let _sibling = fixture.handle.claim_scope(&sibling).unwrap();
    let requests = fixture.handle.diagnostics().save_requests;
    let _parent = fixture.handle.claim_scope(&parent).unwrap();
    assert_eq!(fixture.handle.diagnostics().save_requests, requests);
    fixture.handle.flush();
    fixture.drain_save();
    assert!(fixture.saved_scope("application/new/empty"));
    assert!(fixture.saved_scope("application/new-sibling"));
    assert_eq!(fixture.handle.remove_scope(&parent), 0);
    assert_eq!(fixture.handle.diagnostics().save_requests, requests + 1);
    fixture.handle.flush();
    fixture.drain_save();
    assert!(!fixture.saved_scope("application/new"));
    assert!(!fixture.saved_scope("application/new/empty"));
    assert!(fixture.saved_scope("application/new-sibling"));
}

// services/restoration_test.dart:238. Timing comparison uses save starts
// rather than completed writes, because the Incular store is asynchronous.
#[test]
fn explicit_flush_defers_until_scheduled_frame_but_starts_immediately_when_idle() {
    let mut fixture = Fixture::new();
    fixture.frame();
    let id = fixture.app.primary_window();
    let starts = fixture.handle.diagnostics().saves_started;

    schedule_frame();
    assert!(fixture.app.frame_requested(id));
    fixture.root.set_json(&key("foo"), json!(1));
    assert!(fixture.app.flush_restoration());
    let starts_before_frame = fixture.handle.diagnostics().saves_started;
    fixture.frame();
    fixture.drain_save();
    assert_eq!(fixture.handle.diagnostics().saves_started, starts + 1);
    assert_eq!(fixture.saved_state("application/foo"), Some(json!(1)));

    assert!(!fixture.app.frame_requested(id));
    fixture.root.set_json(&key("foo"), json!(2));
    assert!(fixture.app.flush_restoration());
    assert_eq!(fixture.handle.diagnostics().saves_started, starts + 2);
    fixture.drain_save();
    assert_eq!(fixture.saved_state("application/foo"), Some(json!(2)));
    assert_eq!(
        starts_before_frame, starts,
        "flush started before the frame"
    );
}

#[test]
fn pending_flush_captures_restoration_changes_made_by_the_frame() {
    let mut fixture = Fixture::new();
    fixture.frame();
    let starts = fixture.handle.diagnostics().saves_started;
    fixture.root.set_json(&key("foo"), json!(1));
    let scope = fixture.root.clone();
    let handle = fixture.handle.clone();
    add_post_frame_callback(move |_| {
        assert_eq!(handle.diagnostics().saves_started, starts);
        scope.set_json(&key("foo"), json!(11));
    });
    // A desktop/mobile host may already have requested a redraw without
    // an explicit scheduler callback asking for a frame.
    fixture
        .app
        .note_frame_requested(fixture.app.primary_window());
    assert!(fixture.app.flush_restoration());
    fixture.app.process_runtime_work();
    assert_eq!(fixture.handle.diagnostics().saves_started, starts);
    assert_eq!(fixture.saved_state("application/foo"), None);
    fixture.frame();
    fixture.drain_save();
    assert_eq!(fixture.handle.diagnostics().saves_started, starts + 1);
    assert_eq!(fixture.saved_state("application/foo"), Some(json!(11)));
}

#[test]
fn unrelated_redraw_cannot_flush_a_pending_secondary_window() {
    let mut fixture = Fixture::new();
    fixture.frame();
    let scope = fixture.root.clone();
    let secondary = fixture
        .app
        .open_window(
            WindowOptions::default(),
            LayoutBuilder::new(move |_, _| {
                scope.set_json(&key("foo"), json!(11));
                Widget::box_(Size::new(1., 1.), Color::WHITE)
            })
            .into(),
        )
        .unwrap()
        .id();
    let starts = fixture.handle.diagnostics().saves_started;
    fixture.root.set_json(&key("foo"), json!(1));
    fixture.app.note_frame_requested(secondary);
    assert!(fixture.app.flush_restoration());

    fixture
        .app
        .note_frame_requested(fixture.app.primary_window());
    fixture.frame();
    assert_eq!(fixture.root.get_json(&key("foo")), Some(json!(1)));
    assert_eq!(fixture.handle.diagnostics().saves_started, starts);

    fixture
        .app
        .note_frame_requested(fixture.app.primary_window());
    fixture
        .app
        .run_window_frame_at(
            secondary,
            Constraints::tight(Size::new(100., 100.)),
            Instant::now(),
        )
        .unwrap()
        .expect("secondary frame");
    fixture.drain_save();
    assert_eq!(fixture.handle.diagnostics().saves_started, starts + 1);
    assert_eq!(fixture.saved_state("application/foo"), Some(json!(11)));
}

#[test]
fn cancelling_the_awaited_redraw_releases_the_deferred_save() {
    for reason in ["closed", "hidden", "zero-size"] {
        let mut fixture = Fixture::new();
        fixture.frame();
        let secondary = fixture
            .app
            .open_window(
                WindowOptions::default(),
                Widget::box_(Size::new(1., 1.), Color::WHITE),
            )
            .unwrap();
        let starts = fixture.handle.diagnostics().saves_started;
        fixture.root.set_json(&key("foo"), json!(3));
        fixture.app.note_frame_requested(secondary.id());
        assert!(fixture.app.flush_restoration());
        assert_eq!(fixture.handle.diagnostics().saves_started, starts);

        match reason {
            "closed" => assert!(fixture.app.close_window(secondary.id())),
            "hidden" => secondary.set_visible(false).unwrap(),
            "zero-size" => fixture.app.handle_window_event(WindowEvent::platform(
                secondary.id(),
                PlatformEvent::Metrics(WindowMetrics::new(PhysicalSize::new(0, 0), 1.)),
            )),
            _ => unreachable!(),
        }
        fixture.drain_save();
        assert_eq!(
            fixture.handle.diagnostics().saves_started,
            starts + 1,
            "{reason}"
        );
        assert_eq!(
            fixture.saved_state("application/foo"),
            Some(json!(3)),
            "{reason}"
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn the_shared_restoration_manager_reports_one_owned_lifetime() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};

    let events = Rc::new(RefCell::new(Vec::new()));
    let trace = events.clone();
    let _observer = MemoryAllocations::current().add_listener(move |event| {
        if event.class_name == "RestorationManager" {
            trace.borrow_mut().push(event);
        }
    });
    let fixture = Fixture::new();
    assert_eq!(events.borrow().len(), 1);
    let retained = fixture.handle.clone();
    drop(fixture);
    assert_eq!(
        events.borrow().len(),
        1,
        "a cloned handle holds the same manager"
    );
    drop(retained);

    let events = events.borrow();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, ObjectEventKind::Created);
    assert_eq!(events[1].kind, ObjectEventKind::Disposed);
    assert_eq!(events[0].id, events[1].id);
    assert!(events.iter().all(|event| event.library == "services"));
}
