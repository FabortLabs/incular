use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, InputEvent, Offset, PointerDeviceKind, PointerPhase, Size};
use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
use incular_runtime::Runtime;
use incular_widgets::{
    DragDropContext, Draggable, GestureDetector, HitTestBehavior, LongPressDraggable, MouseCursor,
    MouseRegion, Positioned, Semantics, Stack, Widget, internal::Key,
};

struct Host {
    runtime: Runtime,
    now: Instant,
}

impl Host {
    fn new(source: impl Into<Widget>) -> Self {
        let mut host = Self {
            runtime: Runtime::new(Stack::new([Positioned::new(source).left(20.).top(20.)]).into())
                .unwrap(),
            now: Instant::now(),
        };
        host.frame();
        host
    }
    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(320., 180.)), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }
    fn pointer(&mut self, kind: PointerDeviceKind, id: u64, phase: PointerPhase, point: Offset) {
        let _ = self.runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: id,
                device: 1,
                position: point,
                kind,
                phase,
                buttons: if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) {
                    0
                } else {
                    1
                },
                button: (kind == PointerDeviceKind::Mouse
                    && matches!(phase, PointerPhase::Down | PointerPhase::Up))
                .then_some(incular_core::PRIMARY_POINTER_BUTTON),
                sample: Default::default(),
            },
            self.now,
        );
        self.frame();
    }
    fn center(&self, key: &str) -> Offset {
        let tree = self.runtime.tree();
        let rect = tree
            .element_bounds(
                tree.element_with_key(&Key::from(key))
                    .unwrap_or_else(|| panic!("missing key {key}")),
            )
            .unwrap();
        rect.origin + Offset::new(rect.size.width / 2., rect.size.height / 2.)
    }
}

fn source() -> Widget {
    Widget::box_(Size::new(80., 50.), Color::WHITE).with_key("source")
}
fn feedback() -> Widget {
    Widget::box_(Size::new(80., 50.), Color::BLACK).with_key("feedback")
}

#[test]
fn stationary_mouse_uses_grabbing_feedback_cursor_only_when_feedback_allows_hits() {
    for ignore in [true, false] {
        let context = DragDropContext::new();
        let source = Draggable::new(context.clone(), 1, source()).feedback(|_| {
            MouseRegion::new(feedback())
                .cursor(MouseCursor::Grabbing)
                .into()
        });
        let source = if ignore {
            source
        } else {
            source.ignoring_feedback_pointer(false)
        };
        let mut host = Host::new(source);
        let point = host.center("source");
        let _ = host
            .runtime
            .handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
                pointer: 1,
                device: 1,
                position: point,
                kind: PointerDeviceKind::Mouse,
                change: HoverDeviceChange::Added,
            }));
        assert_eq!(host.runtime.effective_mouse_cursor(), MouseCursor::Basic);
        host.pointer(PointerDeviceKind::Mouse, 1, PointerPhase::Down, point);
        assert!(
            context.is_dragging(),
            "mouse press starts the sole draggable"
        );
        assert!(!host.runtime.transient_surfaces().is_empty());
        assert_eq!(host.center("feedback"), point);
        assert_eq!(
            host.runtime.effective_mouse_cursor(),
            if ignore {
                MouseCursor::Basic
            } else {
                MouseCursor::Grabbing
            },
        );
        host.pointer(PointerDeviceKind::Mouse, 1, PointerPhase::Up, point);
        assert_eq!(host.runtime.effective_mouse_cursor(), MouseCursor::Basic);
    }
}

#[test]
fn a_second_contact_taps_feedback_during_immediate_and_long_press_drags() {
    for long in [false, true] {
        let taps = Rc::new(Cell::new(0));
        let build_feedback = {
            let taps = taps.clone();
            move |_| {
                let taps = taps.clone();
                GestureDetector::new(feedback())
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap(move || taps.set(taps.get() + 1))
                    .into()
            }
        };
        let context = DragDropContext::new();
        let widget: Widget = if long {
            LongPressDraggable::new(context, 1, source())
                .feedback(build_feedback)
                .ignoring_feedback_pointer(false)
                .into()
        } else {
            Draggable::new(context, 1, source())
                .feedback(build_feedback)
                .ignoring_feedback_pointer(false)
                .into()
        };
        let mut host = Host::new(widget);
        let point = host.center("source");
        host.pointer(PointerDeviceKind::Touch, 1, PointerPhase::Down, point);
        if long {
            host.now += Duration::from_millis(500);
            host.frame();
        }
        host.pointer(
            PointerDeviceKind::Touch,
            1,
            PointerPhase::Move,
            point + Offset::new(7., 7.),
        );
        let center = host.center("feedback");
        host.pointer(PointerDeviceKind::Touch, 2, PointerPhase::Down, center);
        host.pointer(PointerDeviceKind::Touch, 2, PointerPhase::Up, center);
        assert_eq!(taps.get(), 1);
        host.pointer(
            PointerDeviceKind::Touch,
            1,
            PointerPhase::Up,
            point + Offset::new(7., 7.),
        );
        assert_eq!(taps.get(), 1);
    }
}

#[test]
fn feedback_pointer_and_accessibility_exclusions_are_independent() {
    for ignore_semantics in [true, false] {
        let source = Draggable::new(DragDropContext::new(), 1, source())
            .feedback(|_| Semantics::new(feedback()).label("drag feedback").into())
            .ignoring_feedback_semantics(ignore_semantics);
        let mut host = Host::new(source);
        let point = host.center("source");
        host.pointer(PointerDeviceKind::Touch, 1, PointerPhase::Down, point);
        assert_eq!(
            host.runtime
                .tree()
                .semantics()
                .iter()
                .any(|(_, node)| node.label.as_deref() == Some("drag feedback")),
            !ignore_semantics
        );
        host.pointer(PointerDeviceKind::Touch, 1, PointerPhase::Up, point);
        assert!(
            !host
                .runtime
                .tree()
                .semantics()
                .iter()
                .any(|(_, node)| node.label.as_deref() == Some("drag feedback"))
        );
    }
}
