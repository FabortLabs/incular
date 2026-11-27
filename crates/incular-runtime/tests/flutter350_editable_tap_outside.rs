use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::TextEditingController;
use incular_widgets::{Directionality, EditableText, FocusNode, Row, TextFieldTapRegion, Widget};

const OUTSIDE: Offset = Offset::new(50., 60.);
const INSIDE: Offset = Offset::new(150., 60.);

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(300., 120.)))
        .unwrap();
}

fn sample(runtime: &mut Runtime, phase: PointerPhase, spot: Offset, kind: PointerDeviceKind) {
    let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
        pointer: 1,
        device: 1,
        kind,
        button: (phase != PointerPhase::Move).then_some(PRIMARY_POINTER_BUTTON),
        buttons: if phase == PointerPhase::Up {
            0
        } else {
            PRIMARY_POINTER_BUTTON
        },
        phase,
        position: spot,
        sample: PointerSampleMetadata::EMPTY,
    });
    frame(runtime);
}

fn tap(runtime: &mut Runtime, spot: Offset, kind: PointerDeviceKind, after_down: impl FnOnce()) {
    sample(runtime, PointerPhase::Down, spot, kind);
    after_down();
    sample(runtime, PointerPhase::Up, spot, kind);
}

fn field(node: &FocusNode, autofocus: bool) -> EditableText {
    EditableText::new(TextEditingController::with_text("text"))
        .focus_node(node.clone())
        .autofocus(autofocus)
        .size(Size::new(100., 40.))
}

fn box_() -> Widget {
    Widget::box_(Size::new(100., 40.), Color::WHITE)
}

fn mount(widgets: impl IntoIterator<Item = Widget>, platform: TextShortcutPlatform) -> Runtime {
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, Row::new(widgets)).into()).unwrap();
    runtime.set_text_shortcut_platform(platform);
    frame(&mut runtime);
    runtime
}

fn focused_gate(focus: bool, release: bool) {
    let node = FocusNode::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let editor = field(&node, focus);
    let record = {
        let log = log.clone();
        move |event: incular_widgets::RawPointerEvent| {
            log.borrow_mut()
                .push((event.phase, event.kind, event.position));
        }
    };
    let editor = if release {
        editor.on_tap_up_outside(record)
    } else {
        editor.on_tap_outside(record)
    };
    let mut runtime = mount([box_(), editor.into()], TextShortcutPlatform::Android);
    assert_eq!(node.has_primary_focus(), focus);
    if focus {
        tap(&mut runtime, INSIDE, PointerDeviceKind::Touch, || {});
        assert!(log.borrow().is_empty(), "{:?}", log.borrow());
    }

    let expected_phase = if release {
        PointerPhase::Up
    } else {
        PointerPhase::Down
    };
    for index in 0..3 {
        tap(&mut runtime, OUTSIDE, PointerDeviceKind::Touch, || {
            assert_eq!(
                log.borrow().len(),
                if focus {
                    index + usize::from(!release)
                } else {
                    0
                }
            );
        });
        assert_eq!(log.borrow().len(), if focus { index + 1 } else { 0 });
    }
    assert!(
        log.borrow()
            .iter()
            .all(|&entry| entry == (expected_phase, PointerDeviceKind::Touch, OUTSIDE))
    );
    assert_eq!(node.has_primary_focus(), focus);
}

#[test]
fn a_focused_editor_receives_pointer_down_outside() {
    focused_gate(true, false);
}

#[test]
fn an_unfocused_editor_does_not_receive_pointer_down_outside() {
    focused_gate(false, false);
}

#[test]
fn a_focused_editor_receives_pointer_up_outside() {
    focused_gate(true, true);
}

#[test]
fn an_unfocused_editor_does_not_receive_pointer_up_outside() {
    focused_gate(false, true);
}

#[test]
fn a_custom_desktop_callback_replaces_blur_but_shared_group_members_do_not_notify() {
    let node = FocusNode::new();
    let outside = Rc::new(std::cell::Cell::new(0));
    let editor = field(&node, true).on_tap_outside({
        let outside = outside.clone();
        move |_| outside.set(outside.get() + 1)
    });
    let mut runtime = mount(
        [
            box_(),
            editor.into(),
            TextFieldTapRegion::new(box_()).into(),
        ],
        TextShortcutPlatform::Linux,
    );
    assert!(node.has_primary_focus());
    tap(
        &mut runtime,
        Offset::new(250., 60.),
        PointerDeviceKind::Mouse,
        || {},
    );
    assert_eq!(outside.get(), 0);
    assert!(node.has_primary_focus());
    tap(&mut runtime, OUTSIDE, PointerDeviceKind::Mouse, || {});
    assert_eq!(outside.get(), 1);
    assert!(node.has_primary_focus());
}

#[test]
fn pointer_up_reports_the_press_group_even_when_it_releases_across_the_boundary() {
    let node = FocusNode::new();
    let releases = Rc::new(RefCell::new(Vec::new()));
    let editor = field(&node, true)
        .on_tap_outside(|_| {})
        .on_tap_up_outside({
            let releases = releases.clone();
            move |event| releases.borrow_mut().push(event.position)
        });
    let kind = PointerDeviceKind::Mouse;
    let mut runtime = mount([box_(), editor.into()], TextShortcutPlatform::Linux);
    sample(&mut runtime, PointerPhase::Down, INSIDE, kind);
    sample(&mut runtime, PointerPhase::Move, OUTSIDE, kind);
    sample(&mut runtime, PointerPhase::Up, OUTSIDE, kind);
    assert!(releases.borrow().is_empty());
    assert!(node.has_primary_focus());

    sample(&mut runtime, PointerPhase::Down, OUTSIDE, kind);
    sample(&mut runtime, PointerPhase::Move, INSIDE, kind);
    sample(&mut runtime, PointerPhase::Up, INSIDE, kind);
    assert_eq!(releases.borrow().as_slice(), [INSIDE]);
    assert!(node.has_primary_focus());
}
