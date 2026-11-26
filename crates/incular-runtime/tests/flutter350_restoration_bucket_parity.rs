//! Targeted public behavior comparisons against Flutter 3.50.0
//! `packages/flutter/test/services/restoration_bucket_test.dart`.
//! Incular leases and persisted scopes are separate objects, unlike Flutter buckets.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use incular_core::{Color, RestorationKey, RestorationScope, Size};
use incular_platform::WindowOptions;
use incular_runtime::{
    Application, InMemoryRestorationStore, RestorationConfig, RestorationHandle,
    RestorationHandoffError,
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
    let store = Arc::new(InMemoryRestorationStore::new());
    let (mut application, root, handle) = open(store.clone());
    let first_scope = root.child_unchecked(key("child1"));
    first_scope.set_json(&key("foo"), json!(22));
    let first = handle.claim_scope(&first_scope).unwrap();
    let sibling = root.child_unchecked(key("child10"));
    sibling.set_json(&key("foo"), json!(99));
    assert_eq!(first_scope.get_json(&key("foo")), Some(json!(22)));

    let before = handle.diagnostics().save_requests;
    let mut second = handle.prepare_scope_handoff(&first_scope).unwrap();
    assert_eq!(second.scope().get_json(&key("foo")), None);
    second.scope().set_json(&key("bar"), json!(55));
    assert_eq!(first_scope.get_json(&key("bar")), None);
    assert_eq!(handle.diagnostics().save_requests, before);
    assert_eq!(
        second.commit().err(),
        Some(RestorationHandoffError::PreviousOwnerStillActive)
    );
    assert_eq!(first_scope.get_json(&key("foo")), Some(json!(22)));
    drop(first);
    let _claim = second
        .commit()
        .expect("the previous owner released the path");
    assert_eq!(second.scope().get_json(&key("foo")), None);
    assert_eq!(second.scope().get_json(&key("bar")), Some(json!(55)));
    assert_eq!(handle.diagnostics().save_requests, before + 1);
    assert_eq!(second.commit().err(), Some(RestorationHandoffError::Closed));
    assert!(handle.claim_scope(&first_scope).is_err());
    application.shutdown();

    let (_application, root, _) = open(store);
    let restored = root.child_unchecked(key("child1"));
    assert_eq!(restored.get_json(&key("foo")), None);
    assert_eq!(restored.get_json(&key("bar")), Some(json!(55)));
    assert_eq!(
        root.child_unchecked(key("child10")).get_json(&key("foo")),
        Some(json!(99))
    );
}

#[test]
fn repeated_pending_claims_do_not_publish_even_after_the_first_owner_releases() {
    let (_application, root, handle) = open(Arc::new(InMemoryRestorationStore::new()));
    let scope = root.child_unchecked(key("child1"));
    let first = handle.claim_scope(&scope).unwrap();
    scope.set_json(&key("committed"), json!(1));
    let mut second = handle.prepare_scope_handoff(&scope).unwrap();
    second.scope().set_json(&key("second"), json!(2));
    drop(first);
    assert!(
        handle.claim_scope(&scope).is_err(),
        "the pending owner reserves this path"
    );
    let mut third = handle.prepare_scope_handoff(&scope).unwrap();
    third.scope().set_json(&key("third"), json!(3));
    let before = handle.diagnostics().save_requests;
    for pending in [&mut second, &mut third] {
        assert_eq!(
            pending.commit().err(),
            Some(RestorationHandoffError::MultipleReplacementOwners)
        );
    }
    assert_eq!(scope.get_json(&key("committed")), Some(json!(1)));
    assert_eq!(scope.get_json(&key("second")), None);
    assert_eq!(scope.get_json(&key("third")), None);
    assert_eq!(handle.diagnostics().save_requests, before);

    let abandoned = third.scope().clone();
    drop(third);
    assert_eq!(abandoned.get_json(&key("third")), None);
    let _claim = second.commit().expect("one replacement remains");
    assert_eq!(scope.get_json(&key("committed")), None);
    assert_eq!(scope.get_json(&key("second")), Some(json!(2)));
}

#[test]
fn handoff_respects_live_descendants_and_cannot_restore_after_reset() {
    let (_application, root, handle) = open(Arc::new(InMemoryRestorationStore::new()));
    let scope = root.child_unchecked(key("child1"));
    let original = handle.claim_scope(&scope).unwrap();
    let descendant = scope.child_unchecked(key("nested"));
    let nested = handle.claim_scope(&descendant).unwrap();
    descendant.set_json(&key("value"), json!(9));
    let mut replacement = handle.prepare_scope_handoff(&scope).unwrap();
    replacement.scope().set_json(&key("value"), json!(10));
    drop(original);
    assert_eq!(
        replacement.commit().err(),
        Some(RestorationHandoffError::ActiveDescendant)
    );
    assert_eq!(descendant.get_json(&key("value")), Some(json!(9)));
    handle.reset();
    drop(nested);
    assert_eq!(
        replacement.commit().err(),
        Some(RestorationHandoffError::Closed)
    );
    assert_eq!(scope.get_json(&key("value")), None);
    assert_eq!(descendant.get_json(&key("value")), None);
    assert!(handle.claim_scope(&scope).is_ok());
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

    let before = handle.diagnostics().save_requests;
    assert_eq!(claim.dispose(), 1);
    assert_eq!(handle.diagnostics().save_requests, before + 1);
    let rebound = root.child_unchecked(key("child1"));
    let _reclaimed = handle.claim_scope(&rebound).unwrap();
    assert_eq!(rebound.get_json(&foo), None);
    assert_eq!(other.get_json(&foo), Some(json!(33)));
}

#[test]
fn disposing_a_scope_erases_nested_values_and_empty_bucket_metadata() {
    use incular_runtime::RestorationStore;

    let store = Arc::new(InMemoryRestorationStore::new());
    let (mut application, root, handle) = open(store.clone());
    let named = |name: &str| root.child_unchecked(key(name));
    let parent = named("child1");
    let nested = parent.child_unchecked(key("child1OfChild1"));
    let nested_empty = parent.child_unchecked(key("child2OfChild1"));
    let other = named("child2");
    let parent_claim = handle.claim_scope(&parent).unwrap();
    let descendant = handle.claim_scope(&nested).unwrap();
    let empty = handle.claim_scope(&nested_empty).unwrap();
    let other_claim = handle.claim_scope(&other).unwrap();
    parent.set_json(&key("foo"), json!(22));
    nested.set_json(&key("foo"), json!(44));
    other.set_json(&key("bar"), json!(55));
    let before = handle.diagnostics().save_requests;
    let scopes_before = handle.diagnostics().restoration_scopes;
    drop((descendant, empty)); // independent native leases
    assert_eq!(parent_claim.dispose(), 2);
    assert_eq!(handle.diagnostics().save_requests, before + 1);
    assert_eq!(handle.diagnostics().restoration_scopes, scopes_before - 3);
    assert_eq!(nested.get_json(&key("foo")), None);
    assert_eq!(other.get_json(&key("bar")), Some(json!(55)));
    assert_eq!(other_claim.dispose(), 1);
    assert_eq!(handle.diagnostics().save_requests, before + 2);
    application.shutdown();
    let raw: Value = serde_json::from_slice(&store.load().unwrap().unwrap()).unwrap();
    assert_eq!(raw["state"], json!({}));
    assert!(raw.get("claimed_scopes").is_none());
}
