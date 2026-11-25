//! Headless restoration data and dirty scheduling comparisons against pinned
//! Flutter 3.50.0. Incular's application store is not Flutter's engine wire.

use std::{cell::RefCell, rc::Rc, sync::Arc, thread, time::Duration};

use incular_animation::{add_post_frame_callback, schedule_frame};
use incular_config::Constraints;
use incular_core::{Color, RestorationKey, RestorationScope, Size, time::Instant};
use incular_platform::WindowOptions;
use incular_runtime::{
    Application, FRAMEWORK_RESTORATION_FORMAT_VERSION, InMemoryRestorationStore, RestorationConfig,
    RestorationHandle,
};
use incular_widgets::Widget;
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
        let document: Value = serde_json::from_slice(&self.store.bytes().unwrap()).unwrap();
        document["state"].get(path).cloned()
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

// services/restoration_bucket_test.dart:172 value persistence plus whether
// merely claiming a new empty owner marks the state dirty. Rust's flat store
// has no stand-alone serializable empty child object.
#[test]
fn claiming_a_new_child_requests_a_save_before_its_first_value_write() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.saved_state("application/child2/foo"), None);
    let requests = fixture.handle.diagnostics().save_requests;
    let child = fixture.root.child_unchecked(key("child2"));
    let _lease = fixture.handle.claim_scope(&child).unwrap();
    let requests_after_claim = fixture.handle.diagnostics().save_requests;

    child.set_json(&key("foo"), json!(55));
    assert_eq!(child.get_json(&key("foo")), Some(json!(55)));
    fixture.handle.flush();
    fixture.drain_save();
    assert_eq!(
        fixture.saved_state("application/child2/foo"),
        Some(json!(55))
    );
    assert!(
        requests_after_claim > requests,
        "claiming the empty child did not mark restoration state dirty"
    );
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
