use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
};
use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
use incular_runtime::{MouseCursorUpdate, Runtime};
use incular_widgets::{MouseCursor, MouseRegion, Positioned, SizedBox, Stack, Widget};

const VIEW: Size = Size {
    width: 128.,
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
