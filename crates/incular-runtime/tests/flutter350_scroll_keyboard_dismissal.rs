use std::time::Duration;

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size, time::Instant,
};
use incular_platform::TextInputCommand;
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{
    Column, Directionality, EditableText, Expanded, FocusNode, FocusScope, ScrollBehavior,
    ScrollConfiguration, ScrollController, ScrollViewKeyboardDismissBehavior as Dismiss,
    SingleChildScrollView, Widget,
};

fn editor(node: &FocusNode) -> Widget {
    EditableText::new(TextEditingController::with_text("still editing"))
        .focus_node(node.clone())
        // Isolate scroll dismissal from the independent outside-press policy.
        .on_tap_outside(|_| {})
        .size(Size::new(300., 50.))
        .into()
}

fn scene(
    focus: &FocusNode,
    scroll: &ScrollController,
    policy: Option<Dismiss>,
    ambient: Dismiss,
    other_scope: bool,
) -> Widget {
    let bars = || Widget::box_(Size::new(300., 1000.), Color::WHITE);
    let child = if other_scope {
        bars()
    } else {
        Column::new([editor(focus), bars()]).into()
    };
    let view = SingleChildScrollView::new(child).controller(scroll.clone());
    let view = if let Some(policy) = policy {
        view.keyboard_dismiss_behavior(policy)
    } else {
        view
    };
    let view = FocusScope::new(view);
    let scene: Widget = if other_scope {
        Column::new([
            Widget::from(FocusScope::new(editor(focus))),
            Expanded::new(view).into(),
        ])
        .into()
    } else {
        view.into()
    };
    Directionality::new(
        TextDirection::Ltr,
        ScrollConfiguration::new(scene)
            .behavior(ScrollBehavior::new().keyboard_dismiss_behavior(ambient)),
    )
    .into()
}

struct Host {
    runtime: Runtime,
    focus: FocusNode,
    scroll: ScrollController,
    time: Instant,
}

impl Host {
    fn new(policy: Option<Dismiss>, ambient: Dismiss, other_scope: bool) -> Self {
        let focus = FocusNode::new();
        let scroll = ScrollController::new();
        let runtime = Runtime::new(scene(&focus, &scroll, policy, ambient, other_scope)).unwrap();
        let mut host = Self {
            runtime,
            focus,
            scroll,
            time: Instant::now(),
        };
        host.frame();
        host.focus.request_focus();
        host.frame();
        assert!(host.focus.has_primary_focus());
        assert!(
            host.runtime
                .take_text_input_commands()
                .iter()
                .any(|command| matches!(command, TextInputCommand::SetClient { .. }))
        );
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(300., 220.)), self.time)
            .unwrap();
        self.time += Duration::from_millis(20);
    }

    fn contact(&mut self, phase: PointerPhase, y: f32) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 7,
                device: 1,
                kind: PointerDeviceKind::Touch,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::EMPTY,
                phase,
                position: Offset::new(120., y),
            },
            self.time,
        );
        self.frame();
    }

    fn drag(&mut self, travel: f32) {
        self.drag_from(150., travel);
    }

    fn drag_from(&mut self, y: f32, travel: f32) {
        self.contact(PointerPhase::Down, y);
        self.contact(PointerPhase::Move, y + travel.signum() * 20.);
        self.contact(PointerPhase::Move, y + travel);
        self.contact(PointerPhase::Up, y + travel);
    }
}

#[test]
fn only_drag_updates_end_editing_for_the_effective_policy() {
    for (policy, ambient, dismissed) in [
        (Some(Dismiss::OnDrag), Dismiss::Manual, true),
        (Some(Dismiss::Manual), Dismiss::OnDrag, false),
        (None, Dismiss::Manual, false),
        (None, Dismiss::OnDrag, true),
    ] {
        let mut host = Host::new(policy, ambient, false);
        // A jump and a wheel move generate updates, but neither has drag details.
        assert!(host.scroll.jump_to(35.));
        host.frame();
        let _ = host.runtime.handle_input_at(
            InputEvent::ScrollAt {
                position: Offset::new(120., 140.),
                delta: Offset::new(0., 30.),
            },
            host.time,
        );
        host.frame();
        assert!(host.focus.has_primary_focus());
        let before = host.scroll.offset();
        host.drag(-70.);
        assert!(host.scroll.offset() > before);
        assert_eq!(host.focus.has_primary_focus(), !dismissed);
        assert_eq!(
            host.runtime
                .take_text_input_commands()
                .iter()
                .any(|command| matches!(command, TextInputCommand::Hide { .. })),
            dismissed
        );
    }
}

#[test]
fn an_overscroll_without_a_scroll_update_does_not_end_editing() {
    let mut host = Host::new(Some(Dismiss::OnDrag), Dismiss::Manual, false);
    assert_eq!(host.scroll.offset(), 0.);
    host.drag(60.);
    assert_eq!(host.scroll.offset(), 0.);
    assert!(host.focus.has_primary_focus());
    host.drag(-65.);
    assert!(!host.focus.has_primary_focus());
}

#[test]
fn a_drag_does_not_unfocus_an_editor_in_another_focus_scope() {
    let mut host = Host::new(Some(Dismiss::OnDrag), Dismiss::Manual, true);
    host.drag(-65.);
    assert!(host.scroll.offset() > 0.);
    assert!(host.focus.has_primary_focus());
}

#[test]
fn dragging_from_the_focused_editor_itself_obeys_the_policy() {
    for policy in [Dismiss::OnDrag, Dismiss::Manual] {
        let mut host = Host::new(Some(policy), Dismiss::Manual, false);
        host.drag_from(25., -40.);
        assert!(host.scroll.offset() > 0.);
        assert_eq!(host.focus.has_primary_focus(), policy == Dismiss::Manual);
    }
}

#[test]
fn updating_ambient_policy_rebinds_the_same_retained_scroller() {
    let mut host = Host::new(None, Dismiss::Manual, false);
    host.drag_from(25., -40.);
    assert!(host.focus.has_primary_focus());

    let root = host.runtime.tree().root().unwrap();
    host.runtime
        .schedule_update(
            root,
            scene(&host.focus, &host.scroll, None, Dismiss::OnDrag, false),
        )
        .unwrap();
    host.frame();
    assert!(host.focus.has_primary_focus());
    host.drag(-50.);
    assert!(!host.focus.has_primary_focus());

    host.focus.request_focus();
    host.frame();
    assert!(host.focus.has_primary_focus());
    host.runtime
        .schedule_update(
            root,
            scene(
                &host.focus,
                &host.scroll,
                Some(Dismiss::Manual),
                Dismiss::OnDrag,
                false,
            ),
        )
        .unwrap();
    host.frame();
    host.drag(-50.);
    assert!(host.focus.has_primary_focus());
}
