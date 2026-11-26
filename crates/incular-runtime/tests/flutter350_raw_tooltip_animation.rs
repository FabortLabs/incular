use incular_animation::{AnimationStyle, Curve, DoubleAnimation};
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{Center, FadeTransition, RawTooltip, RawTooltipController, SizedBox, Widget};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const KEY: &str = "animated tooltip";
type Captured = Rc<RefCell<Option<DoubleAnimation>>>;

struct Fixture {
    runtime: Runtime,
    controller: RawTooltipController,
    captured: Captured,
    builds: Rc<Cell<u32>>,
    start: Instant,
}

fn scene(
    controller: &RawTooltipController,
    captured: &Captured,
    builds: &Rc<Cell<u32>>,
    style: AnimationStyle,
) -> Widget {
    Center::new(
        RawTooltip::with_animated_builder(SizedBox::square(100.), {
            let (captured, builds) = (captured.clone(), builds.clone());
            move |animation| {
                builds.set(builds.get() + 1);
                captured.replace(Some(animation.clone()));
                FadeTransition::from_animation(
                    animation,
                    Widget::box_(Size::new(60., 20.), Color::BLACK).with_key(KEY),
                )
            }
        })
        .animation_style(style)
        .controller(controller.clone()),
    )
    .into()
}

impl Fixture {
    fn new(style: AnimationStyle) -> Self {
        let controller = RawTooltipController::new();
        let captured = Captured::default();
        let builds = Rc::new(Cell::new(0));
        let runtime = Runtime::new(scene(&controller, &captured, &builds, style)).unwrap();
        let mut fixture = Self {
            runtime,
            controller,
            captured,
            builds,
            start: Instant::now(),
        };
        fixture.frame_us(0);
        assert!(!fixture.mounted());
        assert!(fixture.captured.borrow().is_none());
        fixture
    }

    fn frame_us(&mut self, us: u64) -> DisplayList {
        self.runtime
            .run_frame_at(
                Constraints::tight(VIEW),
                self.start + Duration::from_micros(us),
            )
            .unwrap()
            .0
    }

    fn frame(&mut self, ms: u64) -> DisplayList {
        self.frame_us(ms * 1000)
    }

    fn show(&mut self, ms: u64) {
        assert!(
            self.controller
                .ensure_visible_at(self.start + Duration::from_millis(ms))
        );
        self.frame(ms);
        assert!(self.mounted());
    }

    fn dismiss(&mut self, ms: u64) {
        assert!(RawTooltipController::dismiss_all_at(
            self.start + Duration::from_millis(ms)
        ));
        self.frame(ms);
    }

    fn set_style(&mut self, style: AnimationStyle, ms: u64) {
        let root = self.runtime.tree().root().unwrap();
        self.runtime
            .schedule_update(
                root,
                scene(&self.controller, &self.captured, &self.builds, style),
            )
            .unwrap();
        self.frame(ms);
    }

    fn value(&self) -> f32 {
        self.captured.borrow().as_ref().unwrap().value()
    }

    fn mounted(&self) -> bool {
        self.runtime.tree().element_with_key(&KEY.into()).is_some()
    }
}

fn duration(ms: u64) -> Duration {
    Duration::from_millis(ms)
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn forward_motion_is_linear_but_reverse_motion_uses_its_own_curve() {
    let style = AnimationStyle::new()
        .curve(Curve::Linear)
        .reverse_curve(Curve::EaseInExpo)
        .duration(duration(100))
        .reverse_duration(duration(100));
    let mut tooltip = Fixture::new(style);
    tooltip.show(10);
    let builds = tooltip.builds.get();
    let mid = tooltip.frame(60);
    close(tooltip.value(), 0.5);
    assert!(mid.commands().iter().any(|command| {
        matches!(command, PaintCommand::PushOpacity { alpha, .. } if (*alpha - 0.5).abs() < 0.0001)
    }));
    // Flutter's original uses pumpAndSettle before reversing. A
    // frame exactly at 110 ms has value 1 but is not yet completed.
    tooltip.frame_us(110_001);
    close(tooltip.value(), 1.0);

    tooltip.dismiss(120);
    assert!(tooltip.mounted());
    tooltip.frame(170);
    close(tooltip.value(), Curve::EaseInExpo.apply(0.5));
    assert_eq!(
        tooltip.builds.get(),
        builds,
        "animate without rebuilding visible content"
    );
    tooltip.frame(220);
    close(tooltip.value(), 0.0);
    assert!(
        tooltip.mounted(),
        "animation is still active at the endpoint"
    );
    tooltip.frame_us(220_001);
    assert!(!tooltip.mounted());
}

#[test]
fn an_unspecified_reverse_curve_reuses_the_forward_curve() {
    let style = AnimationStyle::new()
        .curve(Curve::EaseInExpo)
        .duration(duration(100))
        .reverse_duration(duration(100));
    let mut tooltip = Fixture::new(style);
    tooltip.show(10);
    tooltip.frame(110);
    tooltip.dismiss(120);
    tooltip.frame(170);
    close(tooltip.value(), Curve::EaseInExpo.apply(0.5));
    tooltip.frame_us(220_001);
    assert!(!tooltip.mounted());
}

#[test]
fn a_retained_tooltip_uses_the_updated_duration_on_its_second_show() {
    let initial = AnimationStyle::new()
        .curve(Curve::Linear)
        .duration(duration(100))
        .reverse_duration(duration(100));
    let mut tooltip = Fixture::new(initial.clone());
    tooltip.show(10);
    tooltip.frame(110);
    tooltip.dismiss(120);
    tooltip.frame_us(220_001);
    assert!(!tooltip.mounted());

    tooltip.set_style(initial.duration(duration(300)), 230);
    tooltip.show(240);
    tooltip.frame(390);
    close(tooltip.value(), 0.5);
    tooltip.frame(540);
    close(tooltip.value(), 1.0);
    tooltip.dismiss(550);
    tooltip.frame_us(650_001);
    assert!(!tooltip.mounted());
}

#[test]
fn missing_durations_use_150ms_to_show_and_75ms_to_hide() {
    let mut tooltip = Fixture::new(AnimationStyle::new().curve(Curve::Linear));
    tooltip.show(10);
    tooltip.frame(85);
    close(tooltip.value(), 0.5);
    tooltip.frame(160);
    close(tooltip.value(), 1.0);
    tooltip.dismiss(170);
    tooltip.frame_us(207_500);
    close(tooltip.value(), 0.5);
    tooltip.frame_us(245_001);
    assert!(!tooltip.mounted());
}

#[test]
fn a_custom_show_duration_does_not_change_the_default_hide_duration() {
    let mut tooltip = Fixture::new(
        AnimationStyle::new()
            .curve(Curve::Linear)
            .duration(duration(400)),
    );
    tooltip.show(10);
    tooltip.frame(410);
    close(tooltip.value(), 1.0);
    tooltip.dismiss(420);
    tooltip.frame_us(457_500);
    close(tooltip.value(), 0.5);
    tooltip.frame_us(495_001);
    assert!(!tooltip.mounted());
}

fn short_linear() -> AnimationStyle {
    AnimationStyle::new()
        .curve(Curve::Linear)
        .duration(duration(100))
        .reverse_duration(duration(100))
}

#[test]
fn a_new_style_replaces_both_curves_without_replacing_the_animation() {
    let mut tooltip = Fixture::new(short_linear());
    tooltip.show(10);
    let original = tooltip.captured.borrow().as_ref().unwrap().clone();
    tooltip.frame(111);
    tooltip.dismiss(120);
    tooltip.frame(221);
    assert!(!tooltip.mounted());

    tooltip.set_style(
        short_linear()
            .curve(Curve::EaseInExpo)
            .reverse_curve(Curve::EaseOutBounce),
        230,
    );
    tooltip.show(240);
    assert_eq!(tooltip.captured.borrow().as_ref(), Some(&original));
    tooltip.frame(290);
    close(original.value(), Curve::EaseInExpo.apply(0.5));
    tooltip.frame(341);

    tooltip.dismiss(350);
    tooltip.frame(400);
    close(original.value(), 0.765625);
    tooltip.frame(451);
    assert!(!tooltip.mounted());
}

#[test]
fn style_update_while_open_changes_the_next_reverse_duration_and_curve() {
    let mut tooltip = Fixture::new(short_linear());
    tooltip.show(10);
    tooltip.frame(111);
    close(tooltip.value(), 1.);

    tooltip.set_style(
        short_linear()
            .reverse_curve(Curve::EaseInExpo)
            .reverse_duration(duration(200)),
        115,
    );
    assert!(tooltip.mounted());
    tooltip.dismiss(120);
    tooltip.frame(220);
    close(tooltip.value(), Curve::EaseInExpo.apply(0.5));
    tooltip.frame(321);
    assert!(!tooltip.mounted());
}

#[test]
fn changing_style_mid_show_keeps_the_in_flight_timebase() {
    let mut tooltip = Fixture::new(short_linear());
    tooltip.show(10);
    let original = tooltip.captured.borrow().as_ref().unwrap().clone();
    tooltip.frame(35);
    close(original.value(), 0.25);

    tooltip.set_style(
        short_linear()
            .duration(duration(300))
            .curve(Curve::EaseInExpo),
        35,
    );
    assert_eq!(tooltip.captured.borrow().as_ref(), Some(&original));
    close(original.value(), Curve::EaseInExpo.apply(0.25));
    tooltip.frame(110);
    close(original.value(), 1.);
    assert!(tooltip.mounted());

    tooltip.dismiss(120);
    tooltip.frame(221);
    assert!(!tooltip.mounted());
}

#[test]
fn zero_show_duration_still_uses_the_default_reverse_duration() {
    let mut tooltip = Fixture::new(
        AnimationStyle::new()
            .curve(Curve::Linear)
            .duration(Duration::ZERO),
    );
    tooltip.show(10);
    close(tooltip.value(), 1.);
    tooltip.dismiss(20);
    tooltip.frame_us(57_500);
    close(tooltip.value(), 0.5);
    tooltip.frame_us(95_001);
    assert!(!tooltip.mounted());
}

#[cfg(debug_assertions)]
fn animation_events() -> (
    Rc<RefCell<Vec<incular_core::memory::allocations::ObjectEvent>>>,
    incular_core::memory::allocations::AllocationSubscription,
) {
    use incular_core::memory::allocations::MemoryAllocations;

    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let sub = MemoryAllocations::current().add_listener(move |event| {
        if event.library == "animation" && event.class_name == "AnimationController" {
            seen.borrow_mut().push(event);
        }
    });
    (events, sub)
}

impl Fixture {
    fn touch(&mut self, phase: incular_core::PointerPhase, ms: u64) {
        use incular_core::{
            InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
            PointerSampleMetadata,
        };
        self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Down {
                    PRIMARY_POINTER_BUTTON
                } else {
                    0
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position: Offset::new(400., 300.),
            },
            self.start + duration(ms),
        );
    }

    fn remove(&mut self, ms: u64) {
        let root = self.runtime.tree().root().unwrap();
        self.runtime
            .schedule_update(root, Center::new(SizedBox::shrink()).into())
            .unwrap();
        self.frame(ms);
        assert!(!self.mounted());
        assert!(self.runtime.tree().transient_surfaces().is_empty());
    }
}

#[cfg(debug_assertions)]
#[test]
fn unmounting_during_an_unaccepted_press_never_allocates_a_timeline() {
    use incular_core::PointerPhase;
    let (events, _sub) = animation_events();
    let mut tooltip = Fixture::new(short_linear());
    tooltip.touch(PointerPhase::Down, 10);
    tooltip.remove(11);
    tooltip.touch(PointerPhase::Up, 12);
    tooltip.frame(1_000);
    assert!(events.borrow().is_empty());
    assert!(tooltip.captured.borrow().is_none());
    assert_eq!(tooltip.controller.pending_deadline(), None);
}

#[test]
fn unmounting_after_show_request_clears_a_pending_press_and_the_open_registry() {
    use incular_core::PointerPhase;
    let mut tooltip = Fixture::new(short_linear());
    tooltip.touch(PointerPhase::Down, 10);
    assert!(
        tooltip
            .controller
            .ensure_visible_at(tooltip.start + duration(10))
    );
    tooltip.remove(11);
    tooltip.touch(PointerPhase::Up, 12);
    tooltip.frame(1_000);
    assert!(!tooltip.controller.is_visible());
    assert_eq!(tooltip.controller.pending_deadline(), None);
    assert!(!RawTooltipController::dismiss_all_at(
        tooltip.start + duration(1_000)
    ));
}

#[cfg(debug_assertions)]
#[test]
fn unmounting_while_animating_disposes_the_clock_even_when_builder_retained_the_animation() {
    use incular_core::memory::allocations::ObjectEventKind;
    let (events, _sub) = animation_events();
    let mut tooltip = Fixture::new(short_linear());
    let root = tooltip.runtime.tree().root().unwrap();
    let captured = tooltip.captured.clone();
    tooltip
        .runtime
        .schedule_update(
            root,
            Center::new(
                RawTooltip::with_animated_builder(SizedBox::square(100.), move |animation| {
                    captured.replace(Some(animation));
                    Widget::box_(Size::new(60., 20.), Color::BLACK).with_key(KEY)
                })
                .animation_style(short_linear())
                .controller(tooltip.controller.clone()),
            )
            .into(),
        )
        .unwrap();
    tooltip.frame(1);
    assert!(events.borrow().is_empty());
    tooltip.show(10);
    tooltip.frame(50);
    close(tooltip.value(), 0.4);
    tooltip.remove(51);
    let retained = tooltip.captured.borrow().as_ref().unwrap().clone();
    let last = retained.value();
    tooltip.frame(1_000);
    close(retained.value(), last);
    assert!(!tooltip.controller.is_visible());
    let seen = events.borrow();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(seen[0].kind, ObjectEventKind::Created);
    assert_eq!(seen[1].kind, ObjectEventKind::Disposed);
    assert_eq!(seen[0].id, seen[1].id);
}

#[test]
fn programmatic_show_is_rejected_after_the_mounted_tooltip_is_removed() {
    let mut tooltip = Fixture::new(short_linear());
    assert!(
        tooltip
            .controller
            .ensure_visible_at(tooltip.start + duration(10))
    );
    tooltip.remove(11);
    assert!(
        !tooltip
            .controller
            .ensure_visible_at(tooltip.start + duration(12))
    );
    tooltip.frame(1_000);
    assert!(!tooltip.mounted());
}

#[test]
fn removing_the_tooltip_cancels_its_queued_pointer_hover() {
    use incular_core::{
        InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata,
    };

    let mut tooltip = Fixture::new(short_linear());
    let root = tooltip.runtime.tree().root().unwrap();
    tooltip
        .runtime
        .schedule_update(
            root,
            Center::new(
                RawTooltip::with_animated_builder(SizedBox::square(100.), |_| {
                    Widget::box_(Size::new(60., 20.), Color::BLACK).with_key(KEY)
                })
                .hover_delay(Duration::from_secs(1))
                .controller(tooltip.controller.clone()),
            )
            .into(),
        )
        .unwrap();
    tooltip.frame(1);
    tooltip.runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 2,
            device: 2,
            kind: PointerDeviceKind::Mouse,
            buttons: 0,
            button: None,
            sample: PointerSampleMetadata::EMPTY,
            phase: PointerPhase::Move,
            position: Offset::new(400., 300.),
        },
        tooltip.start + duration(10),
    );
    tooltip.frame(10);
    assert!(tooltip.controller.is_waiting());
    tooltip.remove(11);
    assert_eq!(tooltip.controller.pending_deadline(), None);
    tooltip.frame(1_100);
    assert!(!tooltip.controller.is_visible());
    assert!(!tooltip.mounted());
}
