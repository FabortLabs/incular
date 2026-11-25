//! Owned activity tokens: brackets identified by generation, not by a
//! boolean saying whether `Start` was emitted.
//!
//! Only the still-current token may close the bracket. Takeovers bump
//! the generation without a duplicate `Start`; stale tokens finish
//! silently without touching a newer activity.

use std::{cell::RefCell, rc::Rc};

use incular_scroll::{
    ActivityOrigin, OwnedDrive, ScrollController, ScrollNotification, ScrollNotificationType,
};

fn event_log(
    controller: &ScrollController,
) -> (
    Rc<RefCell<Vec<ScrollNotificationType>>>,
    incular_scroll::ScrollNotificationSubscription,
) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let logged = log.clone();
    let subscription = controller.add_listener(move |notification: ScrollNotification| {
        logged.borrow_mut().push(notification.kind);
        false
    });
    (log, subscription)
}

#[test]
fn takeover_emits_no_duplicate_start() {
    // An open bracket taken over by a new token continues silently: one
    // `Start` total, and the taking token closes with one `End`.
    let controller = ScrollController::new();
    controller
        .update_extents(500., 100.)
        .expect("free controller publishes");
    let (log, _guard) = event_log(&controller);
    assert!(controller.begin_activity());
    let activity = controller.start_owned_activity(ActivityOrigin::Scrollbar);
    assert_eq!(activity.origin(), ActivityOrigin::Scrollbar);
    assert!(activity.is_current());
    assert!(controller.jump_to(10.));
    assert!(activity.finish());
    assert_eq!(
        log.borrow().as_slice(),
        &[
            ScrollNotificationType::Start,
            ScrollNotificationType::Update,
            ScrollNotificationType::End,
        ]
    );
}

#[test]
fn stale_finish_after_takeover_stays_silent() {
    // The old token survives physically but owns nothing: after an
    // external close and a fresh bracket, its finish emits nothing and
    // the newer activity stays open.
    use incular_scroll::ScrollNotificationType::{End, Start};
    let controller = ScrollController::new();
    let (log, _guard) = event_log(&controller);
    let old = controller.start_owned_activity(ActivityOrigin::Scrollbar);
    assert!(controller.end_activity());
    assert!(controller.begin_activity());
    assert!(!old.is_current());
    assert!(!old.finish(), "stale finish stays silent");
    assert_eq!(log.borrow().as_slice(), &[Start, End, Start]);
    assert!(!controller.begin_activity(), "newer bracket still open");
    assert!(controller.end_activity());
    assert_eq!(log.borrow().as_slice(), &[Start, End, Start, End]);
}

#[test]
fn dropped_live_token_aborts_silently() {
    // Unfinished tokens tear down silently on drop when still current;
    // stale drops change nothing at all.
    use incular_scroll::ScrollNotificationType::End;
    let controller = ScrollController::new();
    let ends = Rc::new(std::cell::Cell::new(0usize));
    let counted = ends.clone();
    let _guard = controller.add_listener(move |notification: ScrollNotification| {
        if notification.kind == End {
            counted.set(counted.get() + 1);
        }
        false
    });
    let live = controller.start_owned_activity(ActivityOrigin::Scrollbar);
    drop(live);
    assert_eq!(ends.get(), 0);
    assert!(controller.begin_activity());
    assert!(controller.end_activity());
    assert_eq!(ends.get(), 1);
    let stale = controller.start_owned_activity(ActivityOrigin::Scrollbar);
    assert!(controller.end_activity());
    drop(stale);
    assert_eq!(ends.get(), 2, "stale drop adds no abort");
}

#[test]
fn drive_snapshots_before_notification_delivery() {
    // The owner-checked drive commits under the lock and snapshots the
    // committed offset/revision before `Update` listeners run: a
    // listener that shrinks the bounds mid-drive is visible afterwards
    // as a post-callback difference against the snapshot — the driver
    // then closes its own tenure without resuming from the new
    // position. A pre-tick offset comparison alone could never see
    // this: the interference happens inside the step.
    use incular_scroll::ScrollNotificationType::{End, Start, Update};
    let controller = ScrollController::new();
    controller
        .update_extents(300., 100.)
        .expect("free controller publishes");
    assert!(controller.jump_to(100.));
    let (log, _guard) = event_log(&controller);
    let activity = controller.start_owned_activity(ActivityOrigin::Scrollbar);
    let shrunk = Rc::new(std::cell::Cell::new(false));
    let _shrink = controller.add_listener({
        let controller = controller.clone();
        let shrunk = shrunk.clone();
        move |notification: ScrollNotification| {
            if notification.kind == Update && !shrunk.get() {
                shrunk.set(true);
                controller
                    .update_extents(120., 100.)
                    .expect("listener changes bounds");
            }
            false
        }
    });
    let committed: OwnedDrive = activity.drive(30.).expect("current token drives");
    assert_eq!(committed.offset, 130.);
    assert!(shrunk.get(), "listener ran inside the drive");
    assert_eq!(controller.offset(), 20., "bounds clamp applied after");
    assert_ne!(
        controller.revision(),
        committed.revision,
        "snapshot predates the listener write"
    );
    assert!(activity.is_current(), "bounds change takes nothing over");
    assert!(activity.finish(), "driver closes its own tenure");
    assert_eq!(controller.offset(), 20., "close writes nothing");
    assert_eq!(
        log.borrow().as_slice(),
        &[Start, Update, ScrollNotificationType::Metrics, Update, End]
    );
}

#[test]
fn stale_token_drives_nothing() {
    // A taken-over token refuses the drive outright: no write, no
    // revision bump, no notification — the newer activity is intact.
    let controller = ScrollController::new();
    controller
        .update_extents(300., 100.)
        .expect("free controller publishes");
    assert!(controller.jump_to(100.));
    let (log, _guard) = event_log(&controller);
    let old = controller.start_owned_activity(ActivityOrigin::Scrollbar);
    let new = controller.start_owned_activity(ActivityOrigin::Drag);
    let revision = controller.revision();
    assert!(old.drive(30.).is_none(), "stale drive refused");
    assert_eq!(controller.offset(), 100.);
    assert_eq!(controller.revision(), revision);
    assert_eq!(
        log.borrow().as_slice(),
        &[ScrollNotificationType::Start],
        "takeover and refused drive stay silent"
    );
    assert!(new.finish());
    assert_eq!(
        log.borrow().as_slice(),
        &[ScrollNotificationType::Start, ScrollNotificationType::End]
    );
}

#[test]
fn finished_token_needs_no_further_cleanup() {
    // Consuming finish closes exactly once: the token is gone afterwards
    // (single `End`), and a fresh bracket starts cleanly on the same
    // controller.
    let controller = ScrollController::new();
    let (log, _guard) = event_log(&controller);
    let activity = controller.start_owned_activity(ActivityOrigin::Scrollbar);
    let generation = activity.generation();
    assert!(activity.is_current());
    assert_eq!(controller.current_activity_id(), Some(activity.id()));
    assert!(activity.finish());
    assert_eq!(controller.current_activity_id(), None);
    assert_eq!(
        log.borrow().as_slice(),
        &[ScrollNotificationType::Start, ScrollNotificationType::End,]
    );
    assert!(controller.begin_activity());
    assert_ne!(
        controller
            .start_owned_activity(ActivityOrigin::Scrollbar)
            .generation(),
        generation,
        "generations advance across brackets"
    );
}

#[test]
fn accepted_drag_notifies_only_direction_changes_then_idle_after_end() {
    use incular_scroll::ScrollNotificationType::{End, Start, Update, UserScroll};
    let controller = ScrollController::new();
    controller.update_extents(500., 100.).unwrap();
    let samples = Rc::new(RefCell::new(Vec::new()));
    let observed = samples.clone();
    let _guard = controller.add_listener(move |notification: ScrollNotification| {
        observed
            .borrow_mut()
            .push((notification.kind, notification.delta));
        false
    });
    let drag = controller.start_owned_activity(ActivityOrigin::Drag);
    let physics = incular_scroll::ScrollPhysics::clamping();
    controller.apply_user_drag(physics, 20.);
    controller.apply_user_drag(physics, 15.);
    controller.apply_user_drag(physics, -5.);
    assert!(drag.finish());
    assert_eq!(
        samples.borrow().as_slice(),
        &[
            (Start, 0.),
            (UserScroll, 20.),
            (Update, 20.),
            (Update, 15.),
            (UserScroll, -5.),
            (Update, -5.),
            (End, 0.),
            (UserScroll, 0.),
        ]
    );
}

#[test]
fn reentrant_takeover_in_direction_notification_prevents_the_stale_drag_write() {
    use incular_scroll::ScrollNotificationType::{End, Start, UserScroll};
    let controller = ScrollController::new();
    controller.update_extents(500., 100.).unwrap();
    let (events, _log) = event_log(&controller);
    let takeover = Rc::new(RefCell::new(None));
    let _guard = controller.add_listener({
        let controller = controller.clone();
        let takeover = takeover.clone();
        move |notification: ScrollNotification| {
            if notification.kind == UserScroll && notification.delta != 0. {
                *takeover.borrow_mut() =
                    Some(controller.start_owned_activity(ActivityOrigin::Animation));
            }
            false
        }
    });
    let drag = controller.start_owned_activity(ActivityOrigin::Drag);
    assert!(
        !controller
            .apply_user_drag(incular_scroll::ScrollPhysics::clamping(), 20.)
            .accepted
    );
    assert_eq!(controller.offset(), 0.);
    assert!(!drag.finish());
    let newer = takeover
        .borrow_mut()
        .take()
        .expect("reentrant newer activity");
    assert!(newer.finish());
    assert_eq!(
        events.borrow().as_slice(),
        &[Start, UserScroll, End, UserScroll]
    );
}
