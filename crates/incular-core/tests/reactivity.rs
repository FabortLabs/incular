use incular_core::{
    BuildContext,
    reactivity::{DependencySource, ReadOnlyGuard, Subscription, assert_mutation_allowed},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn dropping_a_pending_observer_during_notification_skips_it() {
    let source = DependencySource::default();
    let pending = Rc::new(RefCell::new(None::<Subscription>));
    let _first = source.subscribe((), {
        let pending = pending.clone();
        move || {
            pending.borrow_mut().take();
        }
    });
    let calls = Rc::new(Cell::new(0));
    *pending.borrow_mut() = Some(source.subscribe((), {
        let calls = calls.clone();
        move || calls.set(calls.get() + 1)
    }));
    source.notify();
    assert_eq!(calls.get(), 0);
    assert_eq!(source.subscriber_count(), 1);
}

#[test]
fn context_replaces_branches_and_last_owner_releases_subscriptions() {
    let first = DependencySource::default();
    let second = DependencySource::default();
    let context = BuildContext::new();
    context.build(|_| {
        first.track();
        first.track();
    });
    assert_eq!(first.subscriber_count(), 1);
    context.build(|_| {
        second.track();
    });
    assert_eq!(first.subscriber_count(), 0);
    assert_eq!(second.subscriber_count(), 1);
    let clone = context.clone();
    drop(context);
    second.notify();
    assert!(clone.is_dirty());
    drop(clone);
    assert_eq!(second.subscriber_count(), 0);
}

#[test]
fn pure_scope_restores_mutation_permission_after_unwind() {
    let panic = std::panic::catch_unwind(|| {
        let _guard = ReadOnlyGuard::enter();
        assert_mutation_allowed();
    });
    assert!(panic.is_err());
    assert_mutation_allowed();
}

#[test]
fn rebuilding_during_dispatch_cancels_all_previous_source_edges() {
    use incular_core::{BuildContext, reactivity::DependencySource};
    let first = DependencySource::default();
    let second = DependencySource::default();
    let context = BuildContext::new();
    let _earlier = first.subscribe((), {
        let context = context.clone();
        let second = second.clone();
        move || {
            context.run(|| {
                second.track();
            })
        }
    });
    context.run(|| {
        first.track();
        second.track();
    });
    first.notify();
    assert!(!context.is_dirty());
    assert_eq!(context.dependency_count(), 1);
    second.notify();
    assert!(context.take_dirty());
    context.clear_dependencies();
    assert_eq!(second.subscriber_count(), 0);
}

#[test]
fn inherited_chain_stops_at_first_rejection_but_retains_providers_already_seen() {
    #[derive(Clone, Copy)]
    struct Visible(bool);
    let outer = BuildContext::new().provide(Visible(true));
    let inner = outer.provide(Visible(false));
    let reader = inner.for_related_consumer(incular_core::ConsumerId::new());
    let read = || reader.build(|context| context.all_retained_inherited::<Visible>(|v| v.0));

    assert!(!read());
    assert_eq!(reader.dependency_count(), 1);
    outer.insert(Visible(false));
    assert!(!reader.is_dirty(), "never observed the outer provider");
    outer.insert(Visible(true));
    inner.insert(Visible(true));
    assert!(reader.take_dirty());
    assert!(read());
    assert_eq!(reader.dependency_count(), 2);
    inner.insert(Visible(false));
    assert!(reader.take_dirty());
    assert!(!read());
    assert_eq!(reader.dependency_count(), 2);
    outer.insert(Visible(false));
    assert!(reader.take_dirty(), "prior ancestor dependency survives");
    reader.clear_dependencies();
    assert_eq!(reader.dependency_count(), 0);
    outer.insert(Visible(true));
    inner.insert(Visible(true));
    assert!(!reader.is_dirty(), "rebind or detach drops all old edges");
}

#[test]
fn inherited_projection_ignores_gate_handoffs_and_clears_all_subscriptions() {
    struct Enabled(bool);
    let outer = BuildContext::new().provide(Enabled(true));
    let inner = outer.provide(Enabled(false));
    let reader = inner.for_related_consumer(incular_core::ConsumerId::new());
    let read = || reader.build(|ctx| ctx.all_inherited_filtered::<Enabled, _>(|g| g.0));

    assert!(!read());
    assert_eq!(reader.dependency_count(), 2);
    outer.insert(Enabled(false));
    inner.insert(Enabled(true));
    assert!(!reader.is_dirty(), "the combined state never changed");
    outer.insert(Enabled(true));
    assert!(reader.take_dirty());
    assert!(read());
    inner.insert(Enabled(false));
    assert!(reader.take_dirty());
    reader.clear_dependencies();
    assert_eq!(reader.dependency_count(), 0);
    inner.insert(Enabled(true));
    assert!(!reader.is_dirty());
}
