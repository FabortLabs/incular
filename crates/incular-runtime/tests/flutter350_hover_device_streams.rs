//! Per-device hover position, lifecycle, and layout refresh comparisons.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
use incular_runtime::Runtime;
use incular_widgets::{Align, Alignment, MouseCursor, MouseRegion, SizedBox, Stack, Widget};
use std::{cell::RefCell, rc::Rc};

type Events = Rc<RefCell<Vec<(&'static str, PointerPhase, u64, Offset)>>>;

fn region(
    name: &'static str,
    cursor: MouseCursor,
    child: impl Into<Widget>,
    events: &Events,
) -> MouseRegion {
    let enter = events.clone();
    let hover = events.clone();
    let exit = events.clone();
    MouseRegion::new(child)
        .cursor(cursor)
        .on_enter(move |event| {
            enter
                .borrow_mut()
                .push((name, PointerPhase::Enter, event.device, event.position));
        })
        .on_hover(move |event| {
            hover
                .borrow_mut()
                .push((name, PointerPhase::Move, event.device, event.position));
        })
        .on_exit(move |event| {
            exit.borrow_mut()
                .push((name, PointerPhase::Exit, event.device, event.position));
        })
}

fn scene(events: &Events, swap: bool) -> Widget {
    let region_a = region("a", MouseCursor::Text, SizedBox::square(100.), events);
    let region_b = region("b", MouseCursor::NotAllowed, SizedBox::square(100.), events);
    let (a, b) = if swap {
        (Alignment::TOP_RIGHT, Alignment::TOP_LEFT)
    } else {
        (Alignment::TOP_LEFT, Alignment::TOP_RIGHT)
    };
    Stack::new([Align::new(a, region_a), Align::new(b, region_b)]).into()
}

fn frame(runtime: &mut Runtime, size: Size) {
    runtime.run_frame(Constraints::tight(size)).unwrap();
}

fn presence(runtime: &mut Runtime, device: u64, position: Offset, change: HoverDeviceChange) {
    let _ = runtime.handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
        pointer: 1,
        device,
        kind: PointerDeviceKind::Mouse,
        position,
        change,
    }));
}

fn pointer(
    runtime: &mut Runtime,
    device: u64,
    kind: PointerDeviceKind,
    position: Offset,
    phase: PointerPhase,
) {
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 1,
        device,
        kind,
        position,
        phase,
        buttons: if phase == PointerPhase::Down {
            PRIMARY_POINTER_BUTTON
        } else {
            0
        },
        button: matches!(phase, PointerPhase::Down | PointerPhase::Up)
            .then_some(PRIMARY_POINTER_BUTTON),
        sample: PointerSampleMetadata::default(),
    });
}

#[test]
fn a_touch_on_another_region_cannot_move_the_mouse_cursor_or_its_hover() {
    let events = Events::default();
    let mut runtime = Runtime::new(scene(&events, false)).unwrap();
    let size = Size::new(200., 100.);
    frame(&mut runtime, size);
    let mouse = Offset::new(50., 50.);
    let tap = Offset::new(150., 50.);
    presence(&mut runtime, 1, mouse, HoverDeviceChange::Added);
    assert_eq!(events.take(), [("a", PointerPhase::Enter, 1, mouse)]);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Text);

    pointer(
        &mut runtime,
        2,
        PointerDeviceKind::Touch,
        tap,
        PointerPhase::Down,
    );
    pointer(
        &mut runtime,
        2,
        PointerDeviceKind::Touch,
        tap,
        PointerPhase::Up,
    );
    let immediate_cursor = runtime.effective_mouse_cursor();
    runtime
        .schedule_update(runtime.tree().root().unwrap(), scene(&events, false))
        .unwrap();
    frame(&mut runtime, Size::new(220., 100.));
    assert_eq!(
        (
            immediate_cursor,
            runtime.effective_mouse_cursor(),
            events.take()
        ),
        (MouseCursor::Text, MouseCursor::Text, Vec::new())
    );
}

#[test]
fn a_layout_change_refreshes_both_devices_at_their_own_stationary_positions() {
    let events = Events::default();
    let mut runtime = Runtime::new(scene(&events, false)).unwrap();
    let size = Size::new(200., 100.);
    frame(&mut runtime, size);
    let left = Offset::new(50., 50.);
    let right = Offset::new(150., 50.);
    presence(&mut runtime, 1, left, HoverDeviceChange::Added);
    presence(&mut runtime, 2, right, HoverDeviceChange::Added);
    assert_eq!(
        events.take(),
        [
            ("a", PointerPhase::Enter, 1, left),
            ("b", PointerPhase::Enter, 2, right),
        ]
    );
    runtime
        .schedule_update(runtime.tree().root().unwrap(), scene(&events, true))
        .unwrap();
    frame(&mut runtime, size);
    assert_eq!(
        events.take(),
        [
            ("a", PointerPhase::Exit, 1, left),
            ("b", PointerPhase::Enter, 1, left),
            ("b", PointerPhase::Exit, 2, right),
            ("a", PointerPhase::Enter, 2, right),
        ]
    );
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Text);
    presence(&mut runtime, 2, right, HoverDeviceChange::Removed);
    assert_eq!(events.take(), [("a", PointerPhase::Exit, 2, right)]);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::NotAllowed);
    frame(&mut runtime, size);
    assert!(events.take().is_empty());
}

#[test]
fn removing_an_unknown_device_first_is_a_no_op() {
    let events = Events::default();
    let mut runtime = Runtime::new(scene(&events, false)).unwrap();
    frame(&mut runtime, Size::new(200., 100.));
    presence(&mut runtime, 1, Offset::ZERO, HoverDeviceChange::Removed);
    assert!(!runtime.has_hover_device());
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Basic);
    assert!(events.take().is_empty());
}

#[test]
fn multiple_devices_add_move_and_remove_independently() {
    let events = Events::default();
    let root = region("area", MouseCursor::Text, SizedBox::new(), &events);
    let mut runtime = Runtime::new(root.into()).unwrap();
    frame(&mut runtime, Size::new(800., 800.));
    assert!(!runtime.has_hover_device());

    let added = |runtime: &mut Runtime, device, location| {
        presence(runtime, device, location, HoverDeviceChange::Added);
    };
    let moved = |runtime: &mut Runtime, device, location| {
        pointer(
            runtime,
            device,
            PointerDeviceKind::Mouse,
            location,
            PointerPhase::Move,
        );
    };
    let removed = |runtime: &mut Runtime, device, location| {
        presence(runtime, device, location, HoverDeviceChange::Removed);
    };
    let a0 = Offset::ZERO;
    let a1 = Offset::new(0., 1.);
    let a2 = Offset::new(0., 101.);
    let b0 = Offset::new(0., 401.);
    let b1 = Offset::new(1., 401.);
    let b2 = Offset::new(1., 501.);
    let b3 = Offset::new(1., 601.);

    added(&mut runtime, 0, a0);
    moved(&mut runtime, 0, a1);
    assert_eq!(
        events.take(),
        [
            ("area", PointerPhase::Enter, 0, a0),
            ("area", PointerPhase::Move, 0, a1)
        ]
    );
    assert!(runtime.has_hover_device());
    added(&mut runtime, 1, b0);
    moved(&mut runtime, 1, b1);
    assert_eq!(
        events.take(),
        [
            ("area", PointerPhase::Enter, 1, b0),
            ("area", PointerPhase::Move, 1, b1)
        ]
    );
    assert!(runtime.has_hover_device());

    moved(&mut runtime, 0, a2);
    assert_eq!(events.take(), [("area", PointerPhase::Move, 0, a2)]);
    moved(&mut runtime, 1, b2);
    assert_eq!(events.take(), [("area", PointerPhase::Move, 1, b2)]);
    assert!(runtime.has_hover_device());

    removed(&mut runtime, 0, a2);
    assert_eq!(events.take(), [("area", PointerPhase::Exit, 0, a2)]);
    assert!(runtime.has_hover_device());
    moved(&mut runtime, 1, b3);
    assert_eq!(events.take(), [("area", PointerPhase::Move, 1, b3)]);
    assert!(runtime.has_hover_device());
    removed(&mut runtime, 1, b3);
    assert_eq!(events.take(), [("area", PointerPhase::Exit, 1, b3)]);
    assert!(!runtime.has_hover_device());
}

#[test]
fn a_new_pointer_sequence_on_the_same_device_does_not_reenter() {
    let events = Events::default();
    let mut runtime = Runtime::new(scene(&events, false)).unwrap();
    frame(&mut runtime, Size::new(200., 100.));
    let mouse = Offset::new(50., 50.);
    presence(&mut runtime, 1, mouse, HoverDeviceChange::Added);
    assert_eq!(events.take(), [("a", PointerPhase::Enter, 1, mouse)]);
    let moved = Offset::new(51., 50.);
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 42,
        device: 1,
        kind: PointerDeviceKind::Mouse,
        position: moved,
        phase: PointerPhase::Move,
        buttons: 0,
        button: None,
        sample: PointerSampleMetadata::default(),
    });
    assert_eq!(events.take(), [("a", PointerPhase::Move, 1, moved)]);
    presence(&mut runtime, 1, moved, HoverDeviceChange::Removed);
    assert_eq!(events.take(), [("a", PointerPhase::Exit, 1, moved)]);
    assert!(!runtime.has_hover_device());
}
