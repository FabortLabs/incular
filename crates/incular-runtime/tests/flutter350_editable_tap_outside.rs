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

#[test]
fn raw_editors_preserve_each_custom_group_and_share_the_default_group() {
    use incular_widgets::{GlobalKey, TapRegionGroupId};

    let specs = ["Group A", "Group B", "Group C"].map(TapRegionGroupId::named);
    for override_groups in [true, false] {
        let keys: [GlobalKey; 3] = std::array::from_fn(|_| GlobalKey::new());
        let children = std::array::from_fn::<_, 3, _>(|index| {
            let mut editor = EditableText::new(TextEditingController::new());
            if override_groups {
                editor = editor.group_id(specs[index].clone());
            }
            Widget::from(editor.size(Size::new(100., 40.))).with_key(keys[index])
        });
        let runtime = mount(children, TextShortcutPlatform::Linux);
        let tree = runtime.tree();
        for (index, key) in keys.into_iter().enumerate() {
            let id = tree.element_with_key(&key.into()).unwrap();
            let expected = if override_groups {
                &specs[index]
            } else {
                &TapRegionGroupId::TextField
            };
            assert_eq!(tree.text_field_group_id(id), Some(expected));
        }
    }
}

#[test]
fn an_editors_custom_group_matches_companions_by_group_on_both_press_phases() {
    use incular_widgets::{TapRegion, TapRegionSurface};

    let node = FocusNode::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let editor = field(&node, true)
        .group_id("Group A")
        .on_tap_outside({
            let events = events.clone();
            move |_| events.borrow_mut().push(PointerPhase::Down)
        })
        .on_tap_up_outside({
            let events = events.clone();
            move |_| events.borrow_mut().push(PointerPhase::Up)
        });

    let row = Row::new([
        Widget::from(TapRegion::new(box_()).group_id("Group A")),
        editor.into(),
        TextFieldTapRegion::new(box_()).group_id("Group B").into(),
    ]);
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, TapRegionSurface::new(row)).into())
            .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    frame(&mut runtime);
    let companion = OUTSIDE;
    let foreign = Offset::new(250., 60.);
    let kind = PointerDeviceKind::Mouse;

    assert!(node.has_primary_focus());
    sample(&mut runtime, PointerPhase::Down, companion, kind);
    sample(&mut runtime, PointerPhase::Move, foreign, kind);
    sample(&mut runtime, PointerPhase::Up, foreign, kind);
    assert!(events.borrow().is_empty());
    assert!(node.has_primary_focus());

    sample(&mut runtime, PointerPhase::Down, foreign, kind);
    assert_eq!(*events.borrow(), [PointerPhase::Down]);
    sample(&mut runtime, PointerPhase::Move, companion, kind);
    sample(&mut runtime, PointerPhase::Up, companion, kind);
    assert_eq!(*events.borrow(), [PointerPhase::Down, PointerPhase::Up]);
    assert!(node.has_primary_focus());
}

#[test]
fn only_the_current_editors_companion_prevents_default_desktop_blur() {
    let node = FocusNode::new();
    let editor = field(&node, true).group_id("Group A");
    let mut runtime = mount(
        [
            TextFieldTapRegion::new(box_()).group_id("Group A").into(),
            editor.into(),
            TextFieldTapRegion::new(box_()).group_id("Group B").into(),
        ],
        TextShortcutPlatform::Linux,
    );
    let mouse = PointerDeviceKind::Mouse;
    tap(&mut runtime, OUTSIDE, mouse, || {});
    assert!(node.has_primary_focus());
    tap(&mut runtime, Offset::new(250., 60.), mouse, || {});
    assert!(!node.has_focus());
}
