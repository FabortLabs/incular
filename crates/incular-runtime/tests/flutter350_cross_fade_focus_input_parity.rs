//! Flutter 3.50.0 neutral AnimatedCrossFade interaction policies.
use incular_animation::AnimationController;
use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    AnimatedCrossFade, CrossFadeState, Directionality, Focus, FocusNode, GestureDetector,
    Semantics, TextDirection, Widget,
};
use std::{cell::Cell, rc::Rc, time::Duration};

const SWITCH: Duration = Duration::from_millis(50);
const SURFACE: Size = Size {
    width: 300.,
    height: 600.,
};
const FOCUS_A: u64 = 1;
const FOCUS_B: u64 = 2;

fn scene(first: Widget, second: Widget, state: CrossFadeState, exclude: Option<bool>) -> Widget {
    let mut fade = AnimatedCrossFade::new(first, second, state, SWITCH);
    if let Some(exclude) = exclude {
        fade = fade.exclude_bottom_focus(exclude);
    }
    Directionality::new(TextDirection::Ltr, fade).into()
}

struct Probe {
    runtime: Runtime,
    now: Instant,
}
impl Probe {
    fn new(widget: Widget) -> Self {
        let mut probe = Self {
            runtime: Runtime::new(widget).unwrap(),
            now: Instant::now(),
        };
        probe.frame(0);
        probe
    }
    fn frame(&mut self, millis: u64) {
        self.now += Duration::from_millis(millis);
        self.runtime
            .run_frame_at(Constraints::tight(SURFACE), self.now)
            .unwrap();
    }
    fn update(&mut self, scene: Widget) {
        let root = self.runtime.tree().root().unwrap();
        self.runtime.schedule_update(root, scene).unwrap();
        self.frame(0);
        self.frame(0);
    }
    fn tap(&mut self, id: u64) {
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            self.now += Duration::from_millis(10);
            self.runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: id,
                    device: 0,
                    kind: PointerDeviceKind::Touch,
                    buttons: if phase == PointerPhase::Down {
                        PRIMARY_POINTER_BUTTON
                    } else {
                        0
                    },
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    phase,
                    position: Offset::new(150., 300.),
                },
                self.now,
            );
        }
    }
    fn has_label(&self, label: &str) -> bool {
        self.runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, n)| n.label.as_deref() == Some(label))
    }
}

fn focus_comparison(exclude: Option<bool>) -> ([bool; 2], bool, bool) {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let focus = |node: FocusNode, key| {
        Widget::from(Focus::new(Widget::box_(SURFACE, Color::WHITE)).node(node)).with_key(key)
    };
    let mut p = Probe::new(scene(
        focus(a.clone(), FOCUS_A),
        focus(b.clone(), FOCUS_B),
        CrossFadeState::ShowFirst,
        exclude,
    ));
    a.request_focus();
    p.frame(0);
    assert!(a.has_primary_focus());
    let host_a = p
        .runtime
        .focused_element()
        .expect("positive visible-focus control");
    b.request_focus();
    p.frame(0);
    (
        [a.has_primary_focus(), b.has_primary_focus()],
        p.runtime.focused_element() == Some(host_a),
        p.runtime.focused_element().is_some(),
    )
}

#[test]
fn bottom_child_is_excluded_from_focus_by_default() {
    assert_eq!(focus_comparison(None), ([true, false], true, true));
}
#[test]
fn explicitly_allowed_bottom_child_can_take_focus() {
    assert_eq!(focus_comparison(Some(false)), ([false, true], false, true));
}

#[test]
fn second_child_cannot_receive_taps_after_switching_to_first() {
    let calls = Rc::new(Cell::new(0));
    let build = |state| {
        let sink = calls.clone();
        let first = Widget::box_(Size::new(300., 40.), Color::WHITE);
        let second = GestureDetector::new(Widget::box_(SURFACE, Color::BLACK))
            .on_tap(move || sink.set(sink.get() + 1))
            .into();
        scene(first, second, state, None)
    };
    let mut p = Probe::new(build(CrossFadeState::ShowSecond));
    p.tap(1);
    assert_eq!(calls.get(), 1);
    p.update(build(CrossFadeState::ShowFirst));
    p.tap(2);
    p.tap(3);
    assert_eq!(calls.get(), 1);
}

#[test]
fn outgoing_child_keeps_ticking_until_settled_but_loses_semantics_immediately() {
    let controller = AnimationController::new(Duration::from_secs(5));
    let build = |state| {
        let first = Widget::animation_ticker(
            controller.clone(),
            Semantics::new(Widget::box_(SURFACE, Color::WHITE)).label("first"),
        );
        let second = Semantics::new(Widget::box_(SURFACE, Color::BLACK))
            .label("second")
            .into();
        scene(first, second, state, None)
    };
    let mut p = Probe::new(build(CrossFadeState::ShowFirst));
    controller.forward_on_next_tick();
    p.frame(1);
    assert!(p.has_label("first"));
    assert!(!p.has_label("second"));
    p.update(build(CrossFadeState::ShowSecond));
    let before = controller.value();
    p.frame(25);
    let middle = (
        !p.has_label("first"),
        p.has_label("second"),
        controller.value() > before,
    );
    p.frame(25);
    p.frame(25);
    let settled = controller.value();
    p.frame(25);
    let settled_behavior = (
        !p.has_label("first"),
        p.has_label("second"),
        controller.value() == settled,
    );
    assert_eq!(
        (middle, settled_behavior),
        ((true, true, true), (true, true, true))
    );
}
