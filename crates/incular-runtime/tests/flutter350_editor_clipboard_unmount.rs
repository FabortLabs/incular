use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{Clipboard, MemoryClipboard, TextInputCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{
    ContextMenuButtonType, EditableText, FocusNode, SizedBox, StatefulBuilder, TextContextMenu,
    Widget, internal::Key,
};
use serde_json::json;

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 120.)))
        .unwrap();
}

fn exercise(kind: ContextMenuButtonType, final_text: &str) {
    let controller = TextEditingController::with_text("text");
    let focus = FocusNode::new();
    let show = Rc::new(Cell::new(true));
    let menu = Rc::new(RefCell::new(None::<TextContextMenu>));
    let root = StatefulBuilder::new({
        let show = show.clone();
        let menu = menu.clone();
        let controller = controller.clone();
        let focus = focus.clone();
        move |_, setter| {
            if !show.get() {
                return Widget::from(SizedBox::shrink()).with_key("replacement");
            }
            Widget::from(
                EditableText::new(controller.clone())
                    .focus_node(focus.clone())
                    .text_direction(TextDirection::Ltr)
                    .on_changed({
                        let show = show.clone();
                        move |_| {
                            assert!(setter.set_state(|| show.set(false)));
                        }
                    })
                    .context_menu_builder({
                        let menu = menu.clone();
                        move |_, current| {
                            menu.replace(Some(current.clone()));
                            SizedBox::shrink().into()
                        }
                    }),
            )
        }
    });
    let mut clipboard = MemoryClipboard::default();
    clipboard.set_text("pasted".into());
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime.set_clipboard(Box::new(clipboard));
    focus.request_focus();
    frame(&mut runtime);
    let field = runtime.focused_element().unwrap();
    controller.set_selection(TextSelection::new(0, 4));
    frame(&mut runtime);
    let client = runtime.flutter_text_input_configuration().unwrap().0;
    let _ = runtime.take_text_input_commands();
    let request =
        serde_json::to_vec(&json!({"method":"TextInputClient.showToolbar","args":[client.get()]}))
            .unwrap();
    assert_eq!(
        runtime.handle_flutter_text_toolbar_call(&request).unwrap(),
        Some(true)
    );
    frame(&mut runtime);
    let action = menu
        .borrow()
        .as_ref()
        .unwrap()
        .button_items
        .iter()
        .find(|button| button.button_type() == kind)
        .unwrap()
        .clone();
    action.press();
    frame(&mut runtime);
    frame(&mut runtime);

    assert!(!show.get());
    assert_eq!(controller.text(), final_text);
    assert!(
        runtime
            .tree()
            .element_with_key(&Key::from("replacement"))
            .is_some()
    );
    assert!(runtime.tree().text_controller(field).is_none());
    assert!(runtime.focused_element().is_none());
    assert!(runtime.take_text_input_commands().iter().any(|command| {
        matches!(command, TextInputCommand::Clear { client: closed } if *closed == client)
    }));
    // Reusing a retained callback from a removed toolbar is a no-op.
    action.press();
    frame(&mut runtime);
    assert_eq!(controller.text(), final_text);
}

#[test]
fn paste_can_remove_the_editor_that_owns_its_toolbar() {
    exercise(ContextMenuButtonType::Paste, "pasted");
}

#[test]
fn cut_can_remove_the_editor_that_owns_its_toolbar() {
    exercise(ContextMenuButtonType::Cut, "");
}

#[test]
fn a_paste_button_in_the_mounted_menu_reports_the_accepted_text_once() {
    use std::time::{Duration, Instant};

    use incular_config::Alignment;
    use incular_core::{
        Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
        PointerSampleMetadata,
    };
    use incular_widgets::{Align, GestureDetector};

    const PASTE: &str = "neutral paste action";
    const CLIPBOARD: &str = "Dobunezumi mitai ni utsukushiku naritai";
    let controller = TextEditingController::new();
    let clipboard_changes = Rc::new(RefCell::new(Vec::<String>::new()));
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., 32.))
        .on_changed({
            let clipboard_changes = clipboard_changes.clone();
            move |value| clipboard_changes.borrow_mut().push(value.to_owned())
        })
        .selection_handles_builder(|_, _| SizedBox::shrink().into())
        .context_menu_builder(|_, menu| {
            let Some(paste) = menu
                .button_items
                .iter()
                .find(|item| item.button_type() == ContextMenuButtonType::Paste)
            else {
                return SizedBox::shrink().into();
            };
            let paste = paste.clone();
            let action = GestureDetector::new(Widget::box_(Size::new(64., 30.), Color::WHITE))
                .on_tap(move || paste.press());
            Align::new(
                Alignment::BOTTOM_RIGHT,
                Widget::from(action).with_key(PASTE),
            )
            .into()
        });
    let mut runtime = Runtime::new(Align::new(Alignment::TOP_LEFT, field).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let mut clipboard = MemoryClipboard::default();
    clipboard.set_text(CLIPBOARD.into());
    runtime.set_clipboard(Box::new(clipboard));
    let bounds = Constraints::tight(Size::new(400., 120.));
    let start = Instant::now();
    let time = |ms| start + Duration::from_millis(ms);
    runtime.run_frame_at(bounds, time(0)).unwrap();
    let send = |runtime: &mut Runtime, kind, phase, position, ms| {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 3,
                kind,
                buttons: if phase == PointerPhase::Up {
                    0
                } else {
                    PRIMARY_POINTER_BUTTON
                },
                button: Some(PRIMARY_POINTER_BUTTON),
                phase,
                position,
                sample: PointerSampleMetadata::default(),
            },
            time(ms),
        );
    };
    let press = Offset::new(4., 10.);
    send(
        &mut runtime,
        PointerDeviceKind::Touch,
        PointerPhase::Down,
        press,
        5,
    );
    runtime.run_frame_at(bounds, time(510)).unwrap();
    send(
        &mut runtime,
        PointerDeviceKind::Touch,
        PointerPhase::Up,
        press,
        525,
    );
    runtime.run_frame_at(bounds, time(530)).unwrap();

    let id = runtime.focused_element().unwrap();
    let button = runtime
        .tree()
        .element_with_key(&Key::from(PASTE))
        .expect("mounted Paste action");
    let button = runtime.tree().element_bounds(button).unwrap();
    let at = button.origin + Offset::new(button.size.width / 2., button.size.height / 2.);
    send(
        &mut runtime,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        at,
        650,
    );
    send(
        &mut runtime,
        PointerDeviceKind::Mouse,
        PointerPhase::Up,
        at,
        670,
    );
    runtime.run_frame_at(bounds, time(680)).unwrap();
    assert_eq!(*clipboard_changes.borrow(), [CLIPBOARD]);
    assert_eq!(controller.text(), CLIPBOARD);
    assert_eq!(
        controller.selection(),
        TextSelection::collapsed(CLIPBOARD.len())
    );
    assert_eq!(runtime.focused_element(), Some(id));
    assert!(runtime.tree().element_with_key(&Key::from(PASTE)).is_none());
}
