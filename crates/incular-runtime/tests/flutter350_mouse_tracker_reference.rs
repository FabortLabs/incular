use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
use incular_runtime::Runtime;
use incular_widgets::{Align, Alignment, Column, MouseRegion, SizedBox, Transform, Widget};

use PointerDeviceKind::{Mouse, Stylus, Touch};
use PointerPhase::{Down, Enter, Exit, Move, Up};

#[derive(Debug, PartialEq)]
enum Kind {
    Callback(&'static str, PointerPhase),
    Connection(bool),
}

#[derive(Debug, PartialEq)]
struct Event {
    kind: Kind,
    global: Offset,
    local: Offset,
}

fn event(name: &'static str, phase: PointerPhase, global: Offset, local: Offset) -> Event {
    Event {
        kind: Kind::Callback(name, phase),
        global,
        local,
    }
}

fn connection(connected: bool) -> Event {
    Event {
        kind: Kind::Connection(connected),
        global: Offset::ZERO,
        local: Offset::ZERO,
    }
}

type Log = Rc<RefCell<Vec<Event>>>;

fn region(name: &'static str, child: impl Into<Widget>, log: &Log) -> MouseRegion {
    let enter = log.clone();
    let hover = log.clone();
    let exit = log.clone();
    MouseRegion::new(child)
        .on_enter(move |e| {
            enter
                .borrow_mut()
                .push(event(name, Enter, e.position, e.local_position))
        })
        .on_hover(move |e| {
            hover
                .borrow_mut()
                .push(event(name, Move, e.position, e.local_position))
        })
        .on_exit(move |e| {
            exit.borrow_mut()
                .push(event(name, Exit, e.position, e.local_position))
        })
}

fn mount(widget: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(widget.into()).unwrap();
    frame(&mut runtime);
    runtime
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(300., 400.)))
        .unwrap();
}

fn presence(
    runtime: &mut Runtime,
    kind: PointerDeviceKind,
    position: Offset,
    change: HoverDeviceChange,
) {
    let _ = runtime.handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
        pointer: 1,
        device: 1,
        position,
        kind,
        change,
    }));
}

fn pointer(
    runtime: &mut Runtime,
    kind: PointerDeviceKind,
    position: Offset,
    phase: PointerPhase,
    pressed: bool,
) {
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 1,
        device: 1,
        position,
        kind,
        phase,
        buttons: if pressed { PRIMARY_POINTER_BUTTON } else { 0 },
        button: matches!(phase, Down | Up).then_some(PRIMARY_POINTER_BUTTON),
        sample: PointerSampleMetadata::default(),
    });
}

fn kinds(log: &Log) -> Vec<Kind> {
    log.take().into_iter().map(|e| e.kind).collect()
}

#[test]
fn mouse_and_stylus_report_connection_only_after_enter_and_exit() {
    for kind in [Mouse, Stylus] {
        let log = Log::default();
        let mut runtime = mount(region("area", SizedBox::new(), &log));
        let listener_log = log.clone();
        let _sub = runtime.add_hover_device_connection_listener(move |connected| {
            listener_log.borrow_mut().push(connection(connected));
        });
        assert!(!runtime.has_hover_device());

        presence(&mut runtime, kind, Offset::ZERO, HoverDeviceChange::Added);
        assert_eq!(
            log.take(),
            [
                event("area", Enter, Offset::ZERO, Offset::ZERO),
                connection(true)
            ]
        );
        assert!(runtime.has_hover_device());

        let p = Offset::new(1., 101.);
        pointer(&mut runtime, kind, p, Move, false);
        assert_eq!(log.take(), [event("area", Move, p, p)]);
        assert!(runtime.has_hover_device());

        presence(&mut runtime, kind, p, HoverDeviceChange::Removed);
        assert_eq!(log.take(), [event("area", Exit, p, p), connection(false)]);
        assert!(!runtime.has_hover_device());

        let p = Offset::new(0., 301.);
        presence(&mut runtime, kind, p, HoverDeviceChange::Added);
        assert_eq!(log.take(), [event("area", Enter, p, p), connection(true)]);
    }
}

#[test]
fn a_press_drag_and_release_inside_an_annotation_do_not_send_hover() {
    let log = Log::default();
    let mut runtime = mount(region("area", SizedBox::new(), &log));
    let first = Offset::new(0., 101.);
    presence(&mut runtime, Mouse, first, HoverDeviceChange::Added);
    pointer(&mut runtime, Mouse, first, Down, true);
    assert_eq!(log.take(), [event("area", Enter, first, first)]);

    pointer(&mut runtime, Mouse, Offset::new(0., 201.), Move, true);
    assert!(log.take().is_empty());
    pointer(&mut runtime, Mouse, Offset::new(0., 301.), Up, false);
    assert!(log.take().is_empty());
}

fn translated_area(offset: Offset, log: &Log) -> Widget {
    Align::new(
        Alignment::TOP_LEFT,
        Transform::translation(offset, region("area", SizedBox::square(160.), log)),
    )
    .into()
}

#[test]
fn stationary_device_uses_new_hit_transform_on_enter_and_previous_on_exit() {
    let log = Log::default();
    let outside = Offset::new(180., -20.);
    let inside = Offset::new(-10., -20.);
    let global = Offset::new(0., 100.);
    let local = Offset::new(10., 120.);
    let mut runtime = mount(translated_area(outside, &log));
    presence(&mut runtime, Mouse, global, HoverDeviceChange::Added);
    assert!(log.take().is_empty());

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, translated_area(inside, &log))
        .unwrap();
    assert!(log.take().is_empty());
    frame(&mut runtime);
    assert_eq!(log.take(), [event("area", Enter, global, local)]);
    frame(&mut runtime);
    assert!(log.take().is_empty());

    runtime
        .schedule_update(root, translated_area(outside, &log))
        .unwrap();
    frame(&mut runtime);
    assert_eq!(log.take(), [event("area", Exit, global, local)]);
    frame(&mut runtime);
    assert!(log.take().is_empty());
}

#[test]
fn positioned_device_add_and_remove_are_localized_without_a_motion_sample() {
    let log = Log::default();
    let mut runtime = mount(translated_area(Offset::new(-10., -20.), &log));
    let global = Offset::new(0., 100.);
    let local = Offset::new(10., 120.);
    presence(&mut runtime, Mouse, global, HoverDeviceChange::Added);
    assert_eq!(log.take(), [event("area", Enter, global, local)]);
    presence(&mut runtime, Mouse, global, HoverDeviceChange::Removed);
    assert_eq!(log.take(), [event("area", Exit, global, local)]);
}

#[test]
fn motion_delivers_enter_then_hover_and_exit_reuses_the_annotation_coordinates() {
    let log = Log::default();
    let mut runtime = mount(translated_area(Offset::new(-10., -20.), &log));
    let outside = Offset::new(200., 100.);
    let inside = Offset::new(0., 100.);
    presence(&mut runtime, Mouse, outside, HoverDeviceChange::Added);
    assert!(log.take().is_empty());
    pointer(&mut runtime, Mouse, inside, Move, false);
    let local = Offset::new(10., 120.);
    assert_eq!(
        log.take(),
        [
            event("area", Enter, inside, local),
            event("area", Move, inside, local)
        ]
    );
    pointer(&mut runtime, Mouse, outside, Move, false);
    assert_eq!(
        log.take(),
        [event("area", Exit, outside, Offset::new(210., 120.))]
    );
}

#[test]
fn touch_presence_never_establishes_a_hover_device_or_requests_a_frame() {
    let log = Log::default();
    let mut runtime = mount(SizedBox::new());
    frame(&mut runtime);
    assert!(!runtime.frame_requested());
    let listener_log = log.clone();
    let _sub = runtime.add_hover_device_connection_listener(move |value| {
        listener_log.borrow_mut().push(connection(value));
    });
    presence(
        &mut runtime,
        Touch,
        Offset::new(0., 100.),
        HoverDeviceChange::Added,
    );
    assert!(!runtime.has_hover_device());
    assert!(!runtime.frame_requested());
    assert!(log.take().is_empty());
}

#[test]
fn regions_with_only_an_enter_or_only_an_exit_callback_are_safe() {
    let mut runtime = mount(
        Column::new([
            Widget::from(MouseRegion::new(SizedBox::square(30.)).on_enter(|_| {})),
            Widget::from(MouseRegion::new(SizedBox::square(30.)).on_exit(|_| {})),
        ])
        .cross_axis_alignment(incular_config::CrossAxisAlignment::Start)
        .text_direction(incular_config::TextDirection::Ltr),
    );
    let start = Offset::new(0., 31.);
    presence(&mut runtime, Mouse, start, HoverDeviceChange::Added);
    pointer(&mut runtime, Mouse, Offset::new(1., 31.), Move, false);
    presence(&mut runtime, Mouse, start, HoverDeviceChange::Removed);
    assert!(!runtime.has_hover_device());
}

#[test]
fn nested_regions_enter_outer_first_hover_inner_first_and_exit_inner_first() {
    let log = Log::default();
    let nested = region("A", region("B", SizedBox::square(10.), &log), &log);
    let mut runtime = mount(
        Column::new([Widget::from(SizedBox::square(10.)), Widget::from(nested)])
            .cross_axis_alignment(incular_config::CrossAxisAlignment::Start)
            .text_direction(incular_config::TextDirection::Ltr),
    );
    presence(
        &mut runtime,
        Mouse,
        Offset::new(0., 1.),
        HoverDeviceChange::Added,
    );
    assert!(log.take().is_empty());
    pointer(&mut runtime, Mouse, Offset::new(0., 10.), Move, false);
    assert_eq!(
        kinds(&log),
        [
            Kind::Callback("A", Enter),
            Kind::Callback("B", Enter),
            Kind::Callback("B", Move),
            Kind::Callback("A", Move),
        ]
    );
    pointer(&mut runtime, Mouse, Offset::new(0., 20.), Move, false);
    assert_eq!(
        kinds(&log),
        [Kind::Callback("B", Exit), Kind::Callback("A", Exit)]
    );
}

#[test]
fn disjoint_siblings_exit_before_enter_and_hover_in_either_direction() {
    let log = Log::default();
    let a = region("A", SizedBox::square(10.), &log);
    let b = region("B", SizedBox::square(10.), &log);
    let mut runtime = mount(
        Column::new([Widget::from(a), Widget::from(b)])
            .cross_axis_alignment(incular_config::CrossAxisAlignment::Start)
            .text_direction(incular_config::TextDirection::Ltr),
    );
    let a = Offset::new(0., 1.);
    let b = Offset::new(0., 10.);
    presence(&mut runtime, Mouse, a, HoverDeviceChange::Added);
    assert_eq!(kinds(&log), [Kind::Callback("A", Enter)]);
    pointer(&mut runtime, Mouse, b, Move, false);
    assert_eq!(
        kinds(&log),
        [
            Kind::Callback("A", Exit),
            Kind::Callback("B", Enter),
            Kind::Callback("B", Move)
        ]
    );
    pointer(&mut runtime, Mouse, a, Move, false);
    assert_eq!(
        kinds(&log),
        [
            Kind::Callback("B", Exit),
            Kind::Callback("A", Enter),
            Kind::Callback("A", Move)
        ]
    );
}

#[test]
fn a_mouse_region_includes_top_and_left_but_excludes_exact_bottom_and_right() {
    let log = Log::default();
    let area = region("area", SizedBox::new().width(20.).height(10.), &log);
    let mut runtime = mount(Align::new(
        Alignment::TOP_LEFT,
        Transform::translation(Offset::new(10., 20.), area),
    ));
    let start = Offset::new(10., 20.);
    presence(&mut runtime, Mouse, start, HoverDeviceChange::Added);
    assert_eq!(log.take(), [event("area", Enter, start, Offset::ZERO)]);
    let inner = Offset::new(29.75, 29.75);
    pointer(&mut runtime, Mouse, inner, Move, false);
    assert_eq!(
        log.take(),
        [event("area", Move, inner, Offset::new(19.75, 9.75))]
    );
    let right = Offset::new(30., 25.);
    pointer(&mut runtime, Mouse, right, Move, false);
    assert_eq!(
        log.take(),
        [event("area", Exit, right, Offset::new(20., 5.))]
    );
    pointer(&mut runtime, Mouse, start, Move, false);
    assert_eq!(
        log.take(),
        [
            event("area", Enter, start, Offset::ZERO),
            event("area", Move, start, Offset::ZERO)
        ]
    );
    let bottom = Offset::new(15., 30.);
    pointer(&mut runtime, Mouse, bottom, Move, false);
    assert_eq!(
        log.take(),
        [event("area", Exit, bottom, Offset::new(5., 10.))]
    );
}
