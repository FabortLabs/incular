//! Targeted public behavior comparisons against Flutter 3.50.0
//! `packages/flutter/test/services/restoration_bucket_test.dart`.
//! Incular leases and persisted scopes are separate objects, unlike Flutter buckets.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use incular_core::{Color, RestorationKey, RestorationScope, Size};
use incular_platform::WindowOptions;
use incular_runtime::{
    Application, InMemoryRestorationStore, RestorationConfig, RestorationHandle,
};
use incular_widgets::Widget;
use serde_json::{Value, json};

fn key(id: &str) -> RestorationKey {
    RestorationKey::new(id).unwrap()
}

fn open(
    store: Arc<InMemoryRestorationStore>,
) -> (Application, RestorationScope, RestorationHandle) {
    let captured = Rc::new(RefCell::new(None));
    let slot = captured.clone();
    let application = Application::new_with_restoration(
        WindowOptions::default(),
        RestorationConfig::new("org.incular.flutter350.restoration-bucket", 1, store),
        move |cx| {
            *slot.borrow_mut() = Some((cx.restoration_scope().unwrap(), cx.restoration().unwrap()));
            Widget::box_(Size::new(1., 1.), Color::WHITE)
        },
    )
    .unwrap();
    let (root, handle) = captured.borrow_mut().take().unwrap();
    (application, root, handle)
}

// Value and persistence subset of upstream "root bucket values" (:13).
// Flutter's typed read returns null for both absent and null; Incular's raw
// JSON read exposes the distinction that Flutter checks in its serialized map.
#[test]
fn root_scope_preserves_explicit_null_but_not_absence_across_restart() {
    let store = Arc::new(InMemoryRestorationStore::new());
    let (mut application, root, handle) = open(store.clone());
    let stored_null = key("value4");
    let removed = key("value1");
    let initial_requests = handle.diagnostics().save_requests;

    assert_eq!(root.get_json(&stored_null), None);
    root.remove(&stored_null);
    assert_eq!(handle.diagnostics().save_requests, initial_requests);

    root.set_json(&removed, json!(10));
    assert_eq!(root.get_json(&removed), Some(json!(10)));
    root.set_json(&removed, json!(22));
    assert_eq!(root.get_json(&removed), Some(json!(22)));
    root.remove(&removed);
    assert_eq!(root.get_json(&removed), None);

    root.set_json(&stored_null, Value::Null);
    assert_eq!(root.get_json(&stored_null), Some(Value::Null));
    assert_eq!(handle.diagnostics().save_requests, initial_requests + 4);
    application.shutdown();

    let (_restarted, root, _) = open(store);
    assert_eq!(root.get_json(&removed), None);
    assert_eq!(root.get_json(&stored_null), Some(Value::Null));
}

// Observable handoff subset of upstream "claim child that is already claimed
// does not throw if given up" (:237). There is no frame or flush between the
// second claim attempt and dropping the first owner.
#[test]
fn second_scope_claim_can_be_made_before_previous_owner_releases_in_same_turn() {
    let (_application, root, handle) = open(Arc::new(InMemoryRestorationStore::new()));
    let first_scope = root.child_unchecked(key("child1"));
    let first = handle.claim_scope(&first_scope).unwrap();
    first_scope.set_json(&key("foo"), json!(22));
    assert_eq!(first_scope.get_json(&key("foo")), Some(json!(22)));

    let second_scope = root.child_unchecked(key("child1"));
    let second = handle.claim_scope(&second_scope);
    drop(first);
    let _second =
        second.expect("a same-turn handoff can resolve the duplicate before finalization");

    assert_eq!(second_scope.get_json(&key("foo")), None);
    second_scope.set_json(&key("bar"), json!(55));
    assert_eq!(second_scope.get_json(&key("bar")), Some(json!(55)));
}

// A Rust lease release is not the explicit bucket disposal operation in
// upstream :291. Establish the lease control separately; explicit removal in
// the next test is the freshness counterpart.
#[test]
fn releasing_a_lease_does_not_dispose_its_persistent_scope() {
    let (_application, root, handle) = open(Arc::new(InMemoryRestorationStore::new()));
    let first_scope = root.child_unchecked(key("child1"));
    let first = handle.claim_scope(&first_scope).unwrap();
    let foo = key("foo");
    first_scope.set_json(&foo, json!(22));
    assert_eq!(first_scope.get_json(&foo), Some(json!(22)));
    drop(first);

    let next_scope = root.child_unchecked(key("child1"));
    let _next = handle.claim_scope(&next_scope).unwrap();
    assert_eq!(next_scope.get_json(&foo), Some(json!(22)));
}

#[test]
fn explicit_scope_removal_before_reclaim_keeps_the_other_scope_intact() {
    let (_application, root, handle) = open(Arc::new(InMemoryRestorationStore::new()));
    let child = root.child_unchecked(key("child1"));
    let other = root.child_unchecked(key("child2"));
    let foo = key("foo");
    let claim = handle.claim_scope(&child).unwrap();
    child.set_json(&foo, json!(22));
    other.set_json(&foo, json!(33));

    drop(claim);
    assert_eq!(handle.remove_scope(&child), 1);
    let rebound = root.child_unchecked(key("child1"));
    let _reclaimed = handle.claim_scope(&rebound).unwrap();
    assert_eq!(rebound.get_json(&foo), None);
    assert_eq!(other.get_json(&foo), Some(json!(33)));
}
