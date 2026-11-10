//! Flutter's `TextSelectionGestureDetectorBuilder.onSecondaryTap` and
//! `EditableTextState.contextMenuButtonItems`: a secondary click on a field
//! with a `contextMenuBuilder` opens the platform's menu with the buttons the
//! selection and clipboard allow, and each button runs its action and closes
//! the menu.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata,
    SECONDARY_POINTER_BUTTON, Size,
};
use incular_runtime::{Application, Runtime, TextShortcutPlatform};
use incular_widgets::{
    ContextMenuButtonType, EditableText, IgnorePointer, SizedBox, TextContextMenu, TextDirection,
    Widget,
    internal::{TextEditingController, TextSelection},
};
use std::{cell::RefCell, rc::Rc};

type Shown = Rc<RefCell<Vec<TextContextMenu>>>;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(300., 40.))
}

fn mount(text: &str, platform: TextShortcutPlatform) -> (Runtime, TextEditingController, Shown) {
    let controller = TextEditingController::with_text(text);
    let shown: Shown = Rc::default();
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(300., 32.))
        .context_menu_builder({
            let shown = shown.clone();
            move |_, menu| {
                shown.borrow_mut().push(menu.clone());
                // Like a real toolbar, the view-filling overlay itself is
                // not hit; only its content would be.
                IgnorePointer::new(SizedBox::shrink()).into()
            }
        });
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller, shown)
}

fn secondary_click(runtime: &mut Runtime, position: Offset) {
    for (phase, buttons) in [
        (PointerPhase::Down, SECONDARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 3,
            kind: PointerDeviceKind::Mouse,
            buttons,
            button: Some(SECONDARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        });
    }
    runtime.run_frame(bounds()).unwrap();
}

fn types(menu: &TextContextMenu) -> Vec<ContextMenuButtonType> {
    menu.button_items
        .iter()
        .map(|item| item.button_type)
        .collect()
}

fn last(shown: &Shown) -> TextContextMenu {
    shown.borrow().last().cloned().expect("menu built")
}

/// Runs callbacks the menu's buttons queued for the UI owner.
fn process(runtime: Runtime) -> Runtime {
    let mut application = Application::from_runtime(runtime, |_| {});
    application.process_runtime_work();
    let mut runtime = application.into_runtime();
    runtime.run_frame(bounds()).unwrap();
    runtime
}

#[test]
fn windows_secondary_click_places_the_caret_and_offers_select_all_and_paste() {
    let (mut runtime, controller, shown) = mount("hello world", TextShortcutPlatform::Windows);
    runtime.set_clipboard_text("pasted ");
    secondary_click(&mut runtime, Offset::new(12., 10.));
    // An unfocused field collapses the selection at the click and takes focus.
    assert!(controller.selection().is_collapsed());
    assert!(runtime.focused_element().is_some());
    let menu = last(&shown);
    assert_eq!(menu.anchors.primary_anchor, Offset::new(12., 10.));
    // Nothing is selected, so there is nothing to cut or copy.
    assert_eq!(
        types(&menu),
        vec![
            ContextMenuButtonType::Paste,
            ContextMenuButtonType::SelectAll
        ]
    );
}

#[test]
fn copy_button_writes_the_selection_and_closes_the_menu() {
    let (mut runtime, controller, shown) = mount("hello world", TextShortcutPlatform::Linux);
    secondary_click(&mut runtime, Offset::new(12., 10.));
    controller.set_selection(TextSelection::new(0, 5));
    // A second click on the open menu's field toggles it closed...
    secondary_click(&mut runtime, Offset::new(12., 10.));
    let builds = shown.borrow().len();
    // ...and a third opens it with the selection's buttons.
    secondary_click(&mut runtime, Offset::new(12., 10.));
    assert!(shown.borrow().len() > builds);
    let menu = last(&shown);
    assert_eq!(
        types(&menu),
        vec![
            ContextMenuButtonType::Cut,
            ContextMenuButtonType::Copy,
            ContextMenuButtonType::SelectAll,
        ]
    );
    let copy = menu
        .button_items
        .iter()
        .find(|item| item.button_type == ContextMenuButtonType::Copy)
        .unwrap()
        .clone();
    copy.press();
    let mut runtime = process(runtime);
    let clipboard = runtime
        .read_clipboard(incular_platform::TransferReadRequest::format(
            incular_platform::TransferFormat::PlainText,
        ))
        .unwrap()
        .materialize(&incular_platform::TransferFormat::PlainText)
        .unwrap();
    assert_eq!(
        clipboard,
        Some(incular_platform::TransferData::text("hello"))
    );
    // The menu closed: no further builds while frames run.
    let builds = shown.borrow().len();
    runtime.run_frame(bounds()).unwrap();
    assert_eq!(shown.borrow().len(), builds);
}

#[test]
fn macos_secondary_click_selects_the_word_and_never_offers_select_all() {
    let (mut runtime, controller, shown) = mount("hello world", TextShortcutPlatform::MacOs);
    secondary_click(&mut runtime, Offset::new(12., 10.));
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
    assert_eq!(
        types(&last(&shown)),
        vec![ContextMenuButtonType::Cut, ContextMenuButtonType::Copy]
    );
}

#[test]
fn a_field_without_a_builder_still_moves_the_caret() {
    let controller = TextEditingController::with_text("hello world");
    controller.set_selection(TextSelection::collapsed(11));
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(300., 32.));
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    runtime.run_frame(bounds()).unwrap();
    secondary_click(&mut runtime, Offset::new(12., 10.));
    assert!(controller.selection().base < 11);
}

fn mount_area(
    text: &str,
    platform: TextShortcutPlatform,
) -> (Runtime, Shown, incular_widgets::internal::ElementId) {
    let shown: Shown = Rc::default();
    let builder: incular_widgets::EditableTextContextMenuBuilder = Rc::new({
        let shown = shown.clone();
        move |_, menu| {
            shown.borrow_mut().push(menu.clone());
            IgnorePointer::new(SizedBox::shrink()).into()
        }
    });
    let mut runtime = Runtime::new(Widget::selection_area_with_context_menu(
        incular_widgets::internal::SelectionAreaController::new(),
        Widget::selectable_text_styled(
            text,
            incular_text::TextStyle::default(),
            incular_text::TextAlign::Start,
        ),
        builder,
    ))
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    let label = runtime
        .tree()
        .selectable_text_at(Offset::new(4., 8.))
        .expect("selectable label");
    (runtime, shown, label)
}

#[test]
fn selection_area_right_click_on_windows_collapses_and_offers_select_all() {
    let (mut runtime, shown, label) = mount_area("hello world", TextShortcutPlatform::Windows);
    secondary_click(&mut runtime, Offset::new(12., 8.));
    assert_eq!(runtime.focused_element(), Some(label));
    assert_eq!(types(&last(&shown)), vec![ContextMenuButtonType::SelectAll]);
    let select_all = last(&shown).button_items[0].clone();
    select_all.press();
    let runtime = process(runtime);
    assert_eq!(
        runtime
            .tree()
            .selectable_text_selected_text(label)
            .as_deref(),
        Some("hello world")
    );
}

#[test]
fn selection_area_right_click_on_macos_selects_the_word_to_copy() {
    let (mut runtime, shown, label) = mount_area("hello world", TextShortcutPlatform::MacOs);
    secondary_click(&mut runtime, Offset::new(12., 8.));
    assert_eq!(
        runtime
            .tree()
            .selectable_text_selected_text(label)
            .as_deref(),
        Some("hello")
    );
    assert_eq!(
        types(&last(&shown)),
        vec![
            ContextMenuButtonType::Copy,
            ContextMenuButtonType::SelectAll
        ]
    );
}
