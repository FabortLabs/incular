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
