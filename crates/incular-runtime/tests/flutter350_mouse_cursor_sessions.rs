use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
};
use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
use incular_runtime::{MouseCursorUpdate, Runtime};
use incular_widgets::{MouseCursor, MouseRegion, Positioned, SizedBox, Stack, Widget};

const VIEW: Size = Size {
    width: 310.,
    height: 80.,
};
type Activations = Rc<RefCell<Vec<MouseCursorUpdate>>>;

fn frame(runtime: &mut Runtime) {
    runtime.run_frame(Constraints::tight(VIEW)).unwrap();
}

fn mount(root: impl Into<Widget>) -> (Runtime, Activations) {
    let mut runtime = Runtime::new(root.into()).unwrap();
    let activations = Rc::new(RefCell::new(Vec::new()));
    let trace = Rc::clone(&activations);
    runtime.set_mouse_cursor_update_handler(Some(Rc::new(move |update| {
        trace.borrow_mut().push(update)
    })));
    frame(&mut runtime);
    (runtime, activations)
}

fn one_annotation(cursor: MouseCursor, left: f32) -> Stack {
    Stack::new([
        Positioned::new(MouseRegion::new(SizedBox::square(10.)).cursor(cursor))
            .left(left)
            .top(0.),
    ])
    .text_direction(TextDirection::Ltr)
}

fn place(runtime: &mut Runtime, device: u64, x: f32, change: HoverDeviceChange) {
    let _ = runtime.handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
        pointer: 1,
        device,
        kind: PointerDeviceKind::Mouse,
        position: Offset::new(x, 0.),
        change,
    }));
}

fn hover(runtime: &mut Runtime, device: u64, x: f32) {
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 1,
        device,
        kind: PointerDeviceKind::Mouse,
        position: Offset::new(x, 0.),
        phase: PointerPhase::Move,
        buttons: 0,
        button: None,
        sample: PointerSampleMetadata::default(),
    });
}

fn expect(trace: &Activations, expected: &[(u64, MouseCursor)]) {
    let values = trace.take();
    assert!(
        values
            .iter()
            .all(|entry| entry.kind == PointerDeviceKind::Mouse && entry.pointer == 1)
    );
    let pairs: Vec<_> = values
        .iter()
        .map(|entry| (entry.device, entry.cursor))
        .collect();
    assert_eq!(pairs, expected);
}

#[test]
fn outside_add_internal_moves_and_remove_activate_only_changed_cursors() {
    let (mut runtime, trace) = mount(one_annotation(MouseCursor::Grabbing, 5.));
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::Basic)]);
    hover(&mut runtime, 0, 5.);
    expect(&trace, &[(0, MouseCursor::Grabbing)]);
    hover(&mut runtime, 0, 10.);
    expect(&trace, &[]);
    hover(&mut runtime, 0, 0.);
    expect(&trace, &[(0, MouseCursor::Basic)]);
    place(&mut runtime, 0, 0., HoverDeviceChange::Removed);
    expect(&trace, &[]);
    assert_eq!(runtime.mouse_cursor_for_device(0, 1), None);
}

#[test]
fn a_device_added_inside_a_region_activates_its_cursor_and_removal_sends_nothing() {
    let (mut runtime, trace) = mount(one_annotation(MouseCursor::Grabbing, 0.));
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::Grabbing)]);
    assert_eq!(
        runtime.mouse_cursor_for_device(0, 1),
        Some(MouseCursor::Grabbing)
    );
    place(&mut runtime, 0, 0., HoverDeviceChange::Removed);
    expect(&trace, &[]);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Basic);
}

#[test]
fn a_frame_updates_a_stationary_pointer_without_reactivation_when_unchanged() {
    let (mut runtime, trace) = mount(one_annotation(MouseCursor::Defer, 0.));
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::Basic)]);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, one_annotation(MouseCursor::Grabbing, 0.).into())
        .unwrap();
    frame(&mut runtime);
    expect(&trace, &[(0, MouseCursor::Grabbing)]);
    runtime
        .schedule_update(root, one_annotation(MouseCursor::Grabbing, 0.).into())
        .unwrap();
    frame(&mut runtime);
    expect(&trace, &[]);
    place(&mut runtime, 0, 0., HoverDeviceChange::Removed);
    expect(&trace, &[]);
}

fn nested(ordered_front_to_back: [MouseCursor; 3]) -> Widget {
    ordered_front_to_back
        .into_iter()
        .fold(Widget::from(SizedBox::expand()), |child, cursor| {
            MouseRegion::new(child).cursor(cursor).opaque(false).into()
        })
}

#[test]
fn the_first_nondeferring_cursor_wins_over_a_further_annotation() {
    let (mut runtime, trace) = mount(nested([
        MouseCursor::Defer,
        MouseCursor::Click,
        MouseCursor::Grabbing,
    ]));
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::Click)]);
    place(&mut runtime, 0, 5., HoverDeviceChange::Removed);
    expect(&trace, &[]);
}

#[test]
fn all_deferring_annotations_in_front_are_skipped() {
    let (mut runtime, trace) = mount(nested([
        MouseCursor::Defer,
        MouseCursor::Defer,
        MouseCursor::Grabbing,
    ]));
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::Grabbing)]);
    place(&mut runtime, 0, 5., HoverDeviceChange::Removed);
    expect(&trace, &[]);
}

#[test]
fn cursor_selection_is_safe_when_the_host_has_no_cursor_callback() {
    let mut runtime = Runtime::new(one_annotation(MouseCursor::Grabbing, 0.).into()).unwrap();
    frame(&mut runtime);
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Grabbing);
    place(&mut runtime, 0, 0., HoverDeviceChange::Removed);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Basic);
}

#[test]
fn explicit_basic_and_no_annotation_are_the_same_activation() {
    let (mut runtime, trace) = mount(one_annotation(MouseCursor::Basic, 5.));
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::Basic)]);
    hover(&mut runtime, 0, 5.);
    expect(&trace, &[]);
    hover(&mut runtime, 0, 0.);
    expect(&trace, &[]);
    place(&mut runtime, 0, 0., HoverDeviceChange::Removed);
    expect(&trace, &[]);
}

#[test]
fn adding_a_removed_device_starts_a_fresh_session_with_basic() {
    let (mut runtime, trace) = mount(one_annotation(MouseCursor::Click, 5.));
    place(&mut runtime, 0, 5., HoverDeviceChange::Added);
    hover(&mut runtime, 0, 6.);
    expect(&trace, &[(0, MouseCursor::Click)]);
    place(&mut runtime, 0, 6., HoverDeviceChange::Removed);
    expect(&trace, &[]);
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::Basic)]);
    assert_eq!(
        runtime.mouse_cursor_for_device(0, 1),
        Some(MouseCursor::Basic)
    );
}

#[test]
fn two_devices_activate_and_keep_their_cursors_independently() {
    let layer = |x, cursor| {
        Positioned::new(MouseRegion::new(SizedBox::square(100.)).cursor(cursor))
            .left(x)
            .top(0.)
    };
    let (mut runtime, trace) = mount(
        Stack::new([
            layer(100.5, MouseCursor::Click),
            layer(200.5, MouseCursor::NotAllowed),
        ])
        .text_direction(TextDirection::Ltr),
    );
    place(&mut runtime, 1, 0., HoverDeviceChange::Added);
    place(&mut runtime, 2, 0., HoverDeviceChange::Added);
    expect(&trace, &[(1, MouseCursor::Basic), (2, MouseCursor::Basic)]);
    hover(&mut runtime, 1, 101.);
    expect(&trace, &[(1, MouseCursor::Click)]);
    hover(&mut runtime, 2, 102.);
    expect(&trace, &[(2, MouseCursor::Click)]);
    hover(&mut runtime, 2, 202.);
    expect(&trace, &[(2, MouseCursor::NotAllowed)]);
    assert_eq!(
        runtime.mouse_cursor_for_device(1, 100),
        Some(MouseCursor::Click)
    );
    assert_eq!(
        runtime.mouse_cursor_for_device(2, 100),
        Some(MouseCursor::NotAllowed)
    );
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::NotAllowed);
    place(&mut runtime, 2, 202., HoverDeviceChange::Removed);
    expect(&trace, &[]);
    assert_eq!(runtime.effective_mouse_cursor(), MouseCursor::Click);
}

#[test]
fn changing_resize_direction_reactivates_the_cursor_under_a_still_pointer() {
    use incular_core::WindowResizeDirection;
    use incular_widgets::WindowResizeRegion;
    let region = |direction| WindowResizeRegion::new(direction, SizedBox::expand());
    let (mut runtime, trace) = mount(region(WindowResizeDirection::East));
    place(&mut runtime, 0, 0., HoverDeviceChange::Added);
    expect(&trace, &[(0, MouseCursor::ResizeHorizontal)]);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, region(WindowResizeDirection::North).into())
        .unwrap();
    frame(&mut runtime);
    expect(&trace, &[(0, MouseCursor::ResizeVertical)]);
}
