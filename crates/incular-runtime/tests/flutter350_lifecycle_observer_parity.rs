//! Host lifecycle observer comparisons with Flutter 3.50.0
//! widgets/app_lifecycle_listener_test.dart. This projects Flutter's
//! transitions onto states visible in Incular; it does not alias Hidden
//! or reversible Detached to Incular's terminal states.

use std::{cell::RefCell, rc::Rc};

use incular_core::{Color, Size};
use incular_platform::{PlatformEvent, PlatformLifecycle};
use incular_runtime::{ApplicationLifecycle, LifecycleTransition, Runtime};
use incular_widgets::Widget;

use ApplicationLifecycle::{Active, Inactive, Starting, Suspended};

type TransitionLog = Rc<RefCell<Vec<(ApplicationLifecycle, ApplicationLifecycle)>>>;

fn observed_runtime() -> (Runtime, TransitionLog) {
    let mut runtime = Runtime::new(Widget::box_(Size::new(1., 1.), Color::WHITE)).unwrap();
    let log = TransitionLog::default();
    runtime.observe_lifecycle({
        let log = log.clone();
        move |LifecycleTransition { previous, current }| log.borrow_mut().push((previous, current))
    });
    (runtime, log)
}

fn send(runtime: &mut Runtime, event: PlatformLifecycle) {
    let _ = runtime.handle_platform_event(PlatformEvent::Lifecycle(event));
}

// Upstream :91. Detached is the Flutter fixture's initial, not-yet-active
// baseline; Incular exposes Starting initially. The intermediate active
// notification is tested, not a promise of reversible detachment.
#[test]
fn an_initial_inactive_event_notifies_activation_before_inactivity() {
    let (mut runtime, log) = observed_runtime();
    assert_eq!(runtime.lifecycle(), Starting);
    assert!(log.borrow().is_empty());

    send(&mut runtime, PlatformLifecycle::Inactive);
    assert_eq!(runtime.lifecycle(), Inactive);
    send(&mut runtime, PlatformLifecycle::Resumed);
    assert_eq!(runtime.lifecycle(), Active);
    assert_eq!(
        log.take(),
        [(Starting, Active), (Active, Inactive), (Inactive, Active)]
    );
}

// Upstream :124, :156–159 downward jump. Flutter also emits Hidden, which
// cannot be compared without a public Rust state.
#[test]
fn a_direct_suspend_from_active_reports_the_supported_inactive_edge() {
    let (mut runtime, log) = observed_runtime();
    send(&mut runtime, PlatformLifecycle::Resumed);
    assert_eq!(log.take(), [(Starting, Active)]);

    send(&mut runtime, PlatformLifecycle::Suspended);
    assert_eq!(runtime.lifecycle(), Suspended);
    assert_eq!(log.take(), [(Active, Inactive), (Inactive, Suspended)]);
}

// Authored control: Incular delivers the projected edges when an adapter
// emits each one explicitly, and does not double-notify an identical state.
#[test]
fn explicit_host_edges_are_delivered_in_order_and_duplicates_are_ignored() {
    let (mut runtime, log) = observed_runtime();
    for state in [
        PlatformLifecycle::Resumed,
        PlatformLifecycle::Active,
        PlatformLifecycle::Inactive,
        PlatformLifecycle::Inactive,
        PlatformLifecycle::Suspended,
        PlatformLifecycle::Inactive,
        PlatformLifecycle::Resumed,
    ] {
        send(&mut runtime, state);
    }
    assert_eq!(
        log.take(),
        [
            (Starting, Active),
            (Active, Inactive),
            (Inactive, Suspended),
            (Suspended, Inactive),
            (Inactive, Active),
        ]
    );
}

// Upstream :124 projected on the shared states. The runtime has no Hidden
// or restart callback, but it does expose the intermediate Inactive state.
#[test]
fn a_direct_resume_from_suspend_notifies_inactive_before_active() {
    let (mut runtime, log) = observed_runtime();
    for event in [
        PlatformLifecycle::Active,
        PlatformLifecycle::Inactive,
        PlatformLifecycle::Suspended,
    ] {
        send(&mut runtime, event);
    }
    log.take();

    send(&mut runtime, PlatformLifecycle::Resumed);
    assert_eq!(runtime.lifecycle(), Active);
    assert_eq!(log.take(), [(Suspended, Inactive), (Inactive, Active)]);
}

#[test]
fn a_direct_suspend_at_start_notifies_each_supported_edge() {
    let (mut runtime, log) = observed_runtime();
    send(&mut runtime, PlatformLifecycle::Suspended);
    assert_eq!(runtime.lifecycle(), Suspended);
    assert_eq!(
        log.take(),
        [
            (Starting, Active),
            (Active, Inactive),
            (Inactive, Suspended)
        ]
    );
}

#[test]
fn a_direct_local_transition_remains_literal() {
    let (mut runtime, log) = observed_runtime();
    assert!(runtime.transition_lifecycle(Suspended));
    assert!(!runtime.transition_lifecycle(Suspended));
    assert_eq!(log.take(), [(Starting, Suspended)]);
}
