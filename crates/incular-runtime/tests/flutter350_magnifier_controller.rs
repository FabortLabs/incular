use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_animation::{AnimationController, AnimationStatus};
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{MagnifierController, RawMagnifier, SizedBox, Widget, internal::Key};

fn lens(key: &'static str) -> Widget {
    Widget::from(RawMagnifier::new(Size::new(100., 100.))).with_key(key)
}

fn mount(magnifier: &MagnifierController) -> Runtime {
    Runtime::new(magnifier.host(SizedBox::expand())).unwrap()
}

fn frame(runtime: &mut Runtime, now: Instant) {
    runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), now)
        .unwrap();
}

fn mounted(runtime: &Runtime, key: &str) -> bool {
    runtime.tree().element_with_key(&Key::from(key)).is_some()
}

#[test]
fn hide_without_an_animation_removes_the_native_magnifier_immediately() {
    let magnifier = MagnifierController::new();
    let mut runtime = mount(&magnifier);
    let now = Instant::now();
    frame(&mut runtime, now);
    assert!(!magnifier.has_overlay());
    assert!(magnifier.show(now, |_| lens("lens")).is_complete());
    frame(&mut runtime, now);
    assert!(magnifier.shown());
    assert!(mounted(&runtime, "lens"));

    assert!(magnifier.hide(now, true).is_complete());
    assert!(!magnifier.has_overlay());
    frame(&mut runtime, now);
    assert!(!mounted(&runtime, "lens"));
}

#[test]
fn entering_and_completed_are_shown_but_reversing_and_dismissed_are_not() {
    let duration = Duration::from_secs(120);
    let animation = AnimationController::new(duration);
    let magnifier = MagnifierController::with_animation(animation.clone());
    let mut runtime = mount(&magnifier);
    let now = Instant::now();
    frame(&mut runtime, now);

    let enter = magnifier.show(now, |_| lens("lens"));
    frame(&mut runtime, now);
    assert_eq!(animation.status(), AnimationStatus::Forward);
    assert!(enter.is_pending());
    assert!(magnifier.shown());
    assert!(mounted(&runtime, "lens"));

    // Flutter's interpolator completes on the first tick strictly after
    // the duration. pumpAndSettle supplies that tick in the reference.
    let entered = now + duration + Duration::from_millis(1);
    assert!(animation.tick(entered));
    frame(&mut runtime, entered);
    assert_eq!(animation.status(), AnimationStatus::Completed);
    assert!(enter.is_complete());
    assert!(magnifier.shown());

    let exit = magnifier.hide(entered, true);
    frame(&mut runtime, entered);
    assert_eq!(animation.status(), AnimationStatus::Reverse);
    assert!(magnifier.has_overlay());
    assert!(!magnifier.shown());
    assert!(mounted(&runtime, "lens"));

    let dismissed = entered + duration + Duration::from_millis(1);
    assert!(animation.tick(dismissed));
    assert_eq!(animation.status(), AnimationStatus::Dismissed);
    assert!(exit.is_complete());
    assert!(!magnifier.has_overlay());
    assert!(!magnifier.shown());
    frame(&mut runtime, dismissed);
    assert!(!mounted(&runtime, "lens"));
}

#[test]
fn replacement_does_not_reverse_or_let_a_stale_exit_clear_a_new_lens() {
    let duration = Duration::from_millis(100);
    let animation = AnimationController::new(duration);
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let _listener = animation.add_status_listener({
        let statuses = statuses.clone();
        move |status| statuses.borrow_mut().push(status)
    });
    let magnifier = MagnifierController::with_animation(animation.clone());
    let mut runtime = mount(&magnifier);
    let now = Instant::now();

    let _ = magnifier.show(now, |_| lens("first"));
    let now = now + duration;
    let _ = animation.tick(now);
    frame(&mut runtime, now);
    assert!(mounted(&runtime, "first"));
    let _ = magnifier.show(now, |_| lens("second"));
    frame(&mut runtime, now);
    assert!(!mounted(&runtime, "first"));
    assert!(mounted(&runtime, "second"));
    assert!(!statuses.borrow().contains(&AnimationStatus::Reverse));

    let exit = magnifier.hide(now, true);
    let middle = now + duration / 2;
    let _ = animation.tick(middle);
    let _ = magnifier.show(middle, |_| lens("third"));
    assert!(exit.is_canceled());
    let later = middle + duration;
    let _ = animation.tick(later);
    frame(&mut runtime, later);
    assert!(magnifier.shown());
    assert!(mounted(&runtime, "third"));
}

#[test]
fn a_lens_can_be_inserted_below_a_preexisting_overlay_child() {
    use incular_core::Color;
    use incular_rendering::PaintCommand;
    use incular_widgets::ColoredBox;

    const FOREGROUND: Color = Color::rgba(12, 98, 222, 255);
    let magnifier = MagnifierController::new();
    let above = Widget::from(ColoredBox::new(
        FOREGROUND,
        SizedBox::new().width(50.).height(50.),
    ))
    .with_key("above");
    let mut runtime = Runtime::new(magnifier.host_below(SizedBox::expand(), above)).unwrap();
    let now = Instant::now();
    frame(&mut runtime, now);
    let original = runtime
        .tree()
        .element_with_key(&Key::from("above"))
        .unwrap();
    assert!(magnifier.show(now, |_| lens("lens")).is_complete());

    let (list, _) = runtime
        .run_frame_at(Constraints::tight(Size::new(800., 600.)), now)
        .unwrap();
    assert_eq!(
        runtime.tree().element_with_key(&Key::from("above")),
        Some(original)
    );
    assert!(mounted(&runtime, "lens"));
    let before = list
        .commands()
        .iter()
        .position(|command| matches!(command, PaintCommand::PushBackdropFilter { .. }))
        .unwrap();
    let after = list.commands().iter().position(|command| {
        matches!(command, PaintCommand::Rect { color, .. } | PaintCommand::HardEdgeRect { color, .. } if *color == FOREGROUND)
    }).unwrap();
    assert!(
        before < after,
        "the lens may not sample the explicitly higher child"
    );
}
