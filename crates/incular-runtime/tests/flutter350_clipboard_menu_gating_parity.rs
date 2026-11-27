//! Neutral editable-text policy checks against Flutter 3.50.0.
//! The Rust fixture injects an in-memory clipboard and captures menu items;
//! it does not test a design system's toolbar or the OS clipboard.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset,
    PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, SECONDARY_POINTER_BUTTON, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{
    ContextMenuButtonType, EditableText, IgnorePointer, SizedBox, TextContextMenu, TextDirection,
};

type ShownMenu = Rc<RefCell<Option<TextContextMenu>>>;

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(300., 40.)))
        .unwrap();
}

fn click(runtime: &mut Runtime, button: u32) {
    for (phase, buttons) in [(PointerPhase::Down, button), (PointerPhase::Up, 0)] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 3,
            kind: PointerDeviceKind::Mouse,
            buttons,
            button: Some(button),
            sample: Default::default(),
            phase,
            position: Offset::new(12., 10.),
        });
    }
    frame(runtime);
}

fn mount(
    text: &str,
    read_only: bool,
    obscured: bool,
) -> (Runtime, TextEditingController, ShownMenu) {
    mount_with_selection(text, read_only, obscured, None)
}

fn mount_with_selection(
    text: &str,
    read_only: bool,
    obscured: bool,
    interactive: Option<bool>,
) -> (Runtime, TextEditingController, ShownMenu) {
    let controller = TextEditingController::with_text(text);
    let shown = ShownMenu::default();
    let mut field = EditableText::new(controller.clone())
        .size(Size::new(300., 40.))
        .text_direction(TextDirection::Ltr)
        .read_only(read_only)
        .obscure_text(obscured)
        .context_menu_builder({
            let shown = shown.clone();
            move |_, menu| {
                *shown.borrow_mut() = Some(menu.clone());
                IgnorePointer::new(SizedBox::shrink()).into()
            }
        });
    if let Some(interactive) = interactive {
        field = field.enable_interactive_selection(interactive);
    }
    let mut runtime = Runtime::new(field.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    runtime.set_clipboard(Box::new(incular_platform::MemoryClipboard::default()));
    frame(&mut runtime);
    click(&mut runtime, PRIMARY_POINTER_BUTTON);
    assert!(runtime.focused_element().is_some());
    (runtime, controller, shown)
}

fn actions(menu: &TextContextMenu) -> Vec<ContextMenuButtonType> {
    menu.button_items
        .iter()
        .map(|item| item.button_type())
        .collect()
}

// Upstream widgets/editable_text_test.dart:2788, neutral paste-availability
// assertions only. Native toolbar invocation and web suppression are separate.
#[test]
fn paste_item_tracks_nonempty_clipboard_when_menu_is_reopened() {
    let (mut runtime, _, shown) = mount("", false, false);
    runtime.set_clipboard_text("Clipboard data");
    click(&mut runtime, SECONDARY_POINTER_BUTTON);
    let initial = shown
        .borrow_mut()
        .take()
        .expect("nonempty clipboard opens menu");
    assert!(actions(&initial).contains(&ContextMenuButtonType::Paste));

    initial.hide();
    frame(&mut runtime);
    runtime.set_clipboard_text("");
    click(&mut runtime, SECONDARY_POINTER_BUTTON);
    assert!(
        shown
            .borrow()
            .as_ref()
            .is_none_or(|menu| !actions(menu).contains(&ContextMenuButtonType::Paste))
    );
}

// Upstream :3118 menu policy, without the deprecated ToolbarOptions toggle.
#[test]
fn read_only_keeps_copy_and_select_all_but_hides_cut_and_paste() {
    let (mut runtime, controller, shown) = mount("blah blah", true, false);
    runtime.set_clipboard_text("Clipboard data");
    controller.set_selection(TextSelection::new(0, 4));
    click(&mut runtime, SECONDARY_POINTER_BUTTON);
    assert_eq!(controller.selection(), TextSelection::new(0, 4));
    assert_eq!(
        actions(shown.borrow().as_ref().expect("read-only menu")),
        [
            ContextMenuButtonType::Copy,
            ContextMenuButtonType::SelectAll
        ]
    );
}

// Upstream :3152 menu policy. The actual selection is a proper subset of
// the editor text, so lack of Cut and Copy cannot be due to a collapsed caret.
#[test]
fn obscured_field_allows_paste_and_select_all_but_hides_cut_and_copy() {
    let (mut runtime, controller, shown) = mount("blah blah", false, true);
    runtime.set_clipboard_text("Clipboard data");
    controller.set_selection(TextSelection::new(0, 4));
    click(&mut runtime, SECONDARY_POINTER_BUTTON);
    assert_eq!(controller.selection(), TextSelection::new(0, 4));
    assert_eq!(
        actions(shown.borrow().as_ref().expect("obscured menu")),
        [
            ContextMenuButtonType::Paste,
            ContextMenuButtonType::SelectAll
        ]
    );
}

fn select_all_shortcut(runtime: &mut Runtime) {
    let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), Code::KeyA);
    event.modifiers = Modifiers::CONTROL;
    let _ = runtime.handle_input(InputEvent::Key(event));
    frame(runtime);
}

// Outcome from upstream :3235, through the public keyboard action instead
// of Flutter's programmatic EditableTextState.selectAll method.
#[test]
fn select_all_action_keeps_read_only_obscured_text_unselected() {
    let (mut ordinary, controller, _) = mount("blah blah", true, false);
    controller.set_selection(TextSelection::collapsed(0));
    select_all_shortcut(&mut ordinary);
    assert_eq!(controller.selection(), TextSelection::new(0, 9));

    let (mut protected, controller, _) = mount("blah blah", true, true);
    controller.set_selection(TextSelection::collapsed(0));
    select_all_shortcut(&mut protected);
    assert!(
        controller.selection().is_collapsed(),
        "read-only obscured selection became {:?}",
        controller.selection()
    );
}

#[test]
fn select_all_action_respects_explicit_selection_overrides() {
    for (read_only, obscured, interactive, allowed) in [
        (false, false, Some(false), false),
        (true, true, Some(true), true),
        (false, true, None, true),
    ] {
        let (mut runtime, controller, _) =
            mount_with_selection("blah blah", read_only, obscured, interactive);
        controller.set_selection(TextSelection::collapsed(0));
        select_all_shortcut(&mut runtime);
        let expected = if allowed {
            TextSelection::new(0, 9)
        } else {
            TextSelection::collapsed(0)
        };
        assert_eq!(
            controller.selection(),
            expected,
            "interactive={interactive:?}"
        );
    }
}

#[test]
fn keyboard_selection_policy_changes_without_replacing_the_focused_editor() {
    let controller = TextEditingController::with_text("blah blah");
    let scene = |interactive: Option<bool>| {
        let field = EditableText::new(controller.clone())
            .size(Size::new(300., 40.))
            .text_direction(TextDirection::Ltr)
            .read_only(true)
            .obscure_text(true);
        match interactive {
            Some(interactive) => field.enable_interactive_selection(interactive).into(),
            None => field.into(),
        }
    };
    let mut runtime = Runtime::new(scene(None)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    frame(&mut runtime);
    click(&mut runtime, PRIMARY_POINTER_BUTTON);
    let focused = runtime.focused_element().expect("editor focus");
    let root = runtime.tree().root().unwrap();
    for (configured, expected) in [
        (Some(true), TextSelection::new(0, 9)),
        (None, TextSelection::collapsed(0)),
    ] {
        runtime.schedule_update(root, scene(configured)).unwrap();
        frame(&mut runtime);
        assert_eq!(runtime.focused_element(), Some(focused));
        controller.set_selection(TextSelection::collapsed(0));
        select_all_shortcut(&mut runtime);
        assert_eq!(controller.selection(), expected);
    }
}
