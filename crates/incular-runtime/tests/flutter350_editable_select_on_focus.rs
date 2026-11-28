use incular_config::{Constraints, TextDirection};
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PRIMARY_POINTER_BUTTON,
    PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAffinity, TextEditingController, TextSelection};
use incular_widgets::{Column, Directionality, EditableText, FocusNode};

fn tab(runtime: &mut Runtime) {
    let key = KeyboardKey::Named(NamedKey::Tab);
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(key.clone(), Code::Tab)));
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(key, Code::Tab));
}

const PLATFORMS: [(TextShortcutPlatform, bool); 5] = [
    (TextShortcutPlatform::Linux, true),
    (TextShortcutPlatform::MacOs, true),
    (TextShortcutPlatform::Windows, true),
    (TextShortcutPlatform::Android, false),
    (TextShortcutPlatform::Ios, false),
];

fn tap_caret(runtime: &mut Runtime, offset: usize) {
    let targeted = runtime.focused_element().unwrap();
    let caret = runtime
        .tree()
        .text_field_offset_rect(targeted, offset, TextAffinity::Downstream)
        .unwrap();
    let position = caret.origin + Offset::new(0.0, caret.size.height / 2.0);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind: PointerDeviceKind::Touch,
            buttons: if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
            button: Some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::EMPTY,
            position,
            phase,
        });
    }
}

#[test]
fn desktop_tab_selects_single_line_but_not_multiline_mobile_keeps_the_caret() {
    for (platform, desktop) in PLATFORMS {
        let first = TextEditingController::with_text("Text1");
        let second = TextEditingController::with_text("Text2\nLine2");
        let a = FocusNode::new();
        let b = FocusNode::new();
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                Column::new([
                    EditableText::new(first.clone()).focus_node(a.clone()),
                    EditableText::new(second.clone())
                        .focus_node(b.clone())
                        .multiline(true),
                ]),
            )
            .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime
            .run_frame(Constraints::tight(Size::new(360.0, 240.0)))
            .unwrap();

        assert_eq!(first.selection_if_set(), None);
        assert_eq!(second.selection_if_set(), None);
        assert!(!a.has_focus() && !b.has_focus());
        tab(&mut runtime);
        assert!(a.has_primary_focus() && !b.has_focus());
        let all = TextSelection::new(0, 5);
        assert_eq!(
            first.selection(),
            if desktop {
                all
            } else {
                TextSelection::collapsed(5)
            },
            "{platform:?}"
        );
        tap_caret(&mut runtime, 4);
        assert_eq!(first.selection(), TextSelection::collapsed(4));

        tab(&mut runtime);
        assert!(b.has_primary_focus() && !a.has_focus());
        assert_eq!(second.selection(), TextSelection::collapsed(11));
        tap_caret(&mut runtime, 10);
        assert_eq!(second.selection(), TextSelection::collapsed(10));

        tab(&mut runtime);
        assert!(a.has_primary_focus() && !b.has_focus());
        assert_eq!(
            first.selection(),
            if desktop {
                all
            } else {
                TextSelection::collapsed(4)
            }
        );
        tab(&mut runtime);
        assert!(b.has_primary_focus() && !a.has_focus());
        assert_eq!(second.selection(), TextSelection::collapsed(10));
    }
}

#[test]
fn explicit_select_all_override_wins_on_all_supported_platforms() {
    for (platform, _) in PLATFORMS {
        for select_all in [false, true] {
            let field = TextEditingController::with_text("Text");
            let node = FocusNode::new();
            let mut runtime = Runtime::new(
                Directionality::new(
                    TextDirection::Ltr,
                    EditableText::new(field.clone())
                        .focus_node(node.clone())
                        .select_all_on_focus(select_all),
                )
                .into(),
            )
            .unwrap();
            runtime.set_text_shortcut_platform(platform);
            runtime
                .run_frame(Constraints::tight(Size::new(200.0, 40.0)))
                .unwrap();
            assert!(!node.has_focus());
            field.set_selection(TextSelection::collapsed(1));
            tab(&mut runtime);
            assert!(node.has_primary_focus());
            assert_eq!(
                field.selection(),
                if select_all {
                    TextSelection::new(0, 4)
                } else {
                    TextSelection::collapsed(1)
                },
            );
        }
    }
}

#[test]
fn clicking_a_desktop_editor_does_not_select_all() {
    use incular_core::{InputEvent, Offset, PointerPhase};
    let field = TextEditingController::with_text("Text");
    let node = FocusNode::new();
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            EditableText::new(field.clone()).focus_node(node.clone()),
        )
        .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime
        .run_frame(Constraints::tight(Size::new(200.0, 40.0)))
        .unwrap();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(9.0, 12.0),
        });
    }
    assert!(node.has_primary_focus());
    assert!(field.selection().is_collapsed());
}

fn pause_and_resume(runtime: &mut Runtime) {
    use incular_platform::{PlatformEvent, PlatformLifecycle};
    for state in [PlatformLifecycle::Inactive, PlatformLifecycle::Resumed] {
        let _ = runtime.handle_platform_event(PlatformEvent::Lifecycle(state));
    }
}

#[test]
fn autofocus_and_resuming_do_not_replace_a_single_line_caret() {
    for (platform, _) in PLATFORMS {
        let field = TextEditingController::with_text("Flutter!");
        let node = FocusNode::new();
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                EditableText::new(field.clone())
                    .focus_node(node.clone())
                    .autofocus(true),
            )
            .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime
            .run_frame(Constraints::tight(Size::new(200.0, 40.0)))
            .unwrap();
        assert!(node.has_focus());
        assert_eq!(field.selection(), TextSelection::collapsed(8));
        pause_and_resume(&mut runtime);
        assert!(node.has_focus());
        assert_eq!(field.selection(), TextSelection::collapsed(8));
    }
}

#[test]
fn external_focus_to_a_different_field_after_resuming_still_selects_all_on_desktop() {
    for (platform, desktop) in PLATFORMS {
        if !desktop {
            continue;
        }
        let a = FocusNode::new();
        let b = FocusNode::new();
        let first = TextEditingController::with_text("Flutter!");
        let second = TextEditingController::with_text("Dart!");
        first.set_selection(TextSelection::collapsed(8));
        second.set_selection(TextSelection::collapsed(5));
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                Column::new([
                    EditableText::new(first.clone())
                        .focus_node(a.clone())
                        .autofocus(true),
                    EditableText::new(second.clone()).focus_node(b.clone()),
                ]),
            )
            .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime
            .run_frame(Constraints::tight(Size::new(200.0, 120.0)))
            .unwrap();
        assert!(a.has_primary_focus() && !b.has_focus());
        assert_eq!(first.selection(), TextSelection::collapsed(8));
        assert_eq!(second.selection(), TextSelection::collapsed(5));
        pause_and_resume(&mut runtime);
        tab(&mut runtime);
        assert!(!a.has_focus() && b.has_primary_focus());
        assert_eq!(first.selection(), TextSelection::collapsed(8));
        assert_eq!(second.selection(), TextSelection::new(0, 5));
    }
}

fn editable_focus_race() -> (Runtime, TextEditingController, FocusNode, FocusNode) {
    use incular_widgets::{Focus, SizedBox, Widget};

    let editor = TextEditingController::with_text("Text1");
    let first = FocusNode::new();
    let second = FocusNode::new();
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            Column::new([
                Widget::from(EditableText::new(editor.clone()).focus_node(first.clone()))
                    .with_key("editable-focus-race"),
                SizedBox::new().height(200.).into(),
                Focus::new(SizedBox::shrink())
                    .node(second.clone())
                    .into(),
            ]),
        )
        .into(),
    )
    .unwrap();
    // Linux uses the same external select-all default as Flutter Web.
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    (runtime, editor, first, second)
}

fn request_by_repeating_value(runtime: &mut Runtime, editor: &TextEditingController) {
    let field = runtime
        .tree()
        .element_with_key(&"editable-focus-race".into())
        .unwrap();
    assert!(runtime.user_update_editable_value(
        field,
        editor.value(),
        incular_text::SelectionChangedCause::Keyboard,
    ));
}

#[test]
fn stealing_focus_before_a_frame_does_not_suppress_external_select_all_later() {
    let (mut runtime, editor, first, second) = editable_focus_race();
    let bounds = Constraints::tight(Size::new(800., 600.));
    assert!(!first.has_focus() && !second.has_focus());
    assert_eq!(editor.selection_if_set(), None);

    request_by_repeating_value(&mut runtime, &editor);
    assert!(!first.has_focus() && !second.has_focus());
    assert_eq!(editor.selection_if_set(), None);

    second.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(!first.has_focus() && second.has_primary_focus());
    assert_eq!(editor.selection_if_set(), None);

    first.request_focus();
    runtime.run_frame(bounds).unwrap();
    assert!(first.has_primary_focus() && !second.has_focus());
    assert_eq!(editor.selection_if_set(), Some(TextSelection::new(0, 5)));
}

#[test]
fn editor_focus_does_not_select_all_unless_a_newer_external_request_wins() {
    for external in [false, true] {
        let (mut runtime, editor, first, second) = editable_focus_race();
        request_by_repeating_value(&mut runtime, &editor);
        if external {
            first.request_focus();
        }
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        assert!(first.has_primary_focus() && !second.has_focus());
        assert_eq!(
            editor.selection_if_set(),
            Some(if external {
                TextSelection::new(0, 5)
            } else {
                TextSelection::collapsed(5)
            })
        );
    }
}

#[test]
fn a_user_edit_callback_can_supply_the_newest_focus_request() {
    use incular_text::{SelectionChangedCause, TextEditingValue};
    use incular_widgets::{Focus, SizedBox, Widget};

    let editor = TextEditingController::with_text("old");
    let edit_focus = FocusNode::new();
    let other_focus = FocusNode::new();
    let field = EditableText::new(editor.clone())
        .focus_node(edit_focus.clone())
        .on_changed({
            let other_focus = other_focus.clone();
            move |text| {
                assert_eq!(text, "new");
                other_focus.request_focus();
            }
        });
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            Column::new([
                Widget::from(field).with_key("edit"),
                Focus::new(SizedBox::shrink()).node(other_focus.clone()).into(),
            ]),
        )
        .into(),
    )
    .unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    let target = runtime.tree().element_with_key(&"edit".into()).unwrap();
    let mut next = TextEditingValue::new("new");
    next.selection = TextSelection::collapsed(1);
    assert!(runtime.user_update_editable_value(target, next, SelectionChangedCause::Keyboard));
    assert!(!edit_focus.has_focus() && !other_focus.has_focus());
    runtime.run_frame(bounds).unwrap();
    assert_eq!(editor.text(), "new");
    assert_eq!(editor.selection_if_set(), Some(TextSelection::collapsed(1)));
    assert!(!edit_focus.has_focus() && other_focus.has_primary_focus());
}
