use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextAffinity, TextAlign, TextEditingController, TextSelection};
use incular_widgets::{
    Actions, EditableText, ExtendSelectionByCharacterIntent, FocusableActionDetector,
    LogicalShortcutKey, Shortcuts, Widget,
};

const DOCUMENT: &str =
    "Now is the time for\nall good people\nto come to the aid\nof their country.";
const FORWARD: ExtendSelectionByCharacterIntent = ExtendSelectionByCharacterIntent::new(true, true);

fn editor(controller: &TextEditingController) -> EditableText {
    controller.set_selection_with_affinity(TextSelection::collapsed(0), TextAffinity::Upstream);
    EditableText::new(controller.clone())
        .autofocus(true)
        .text_direction(TextDirection::Ltr)
        .text_align(TextAlign::Right)
        .max_lines(Some(10))
        .size(Size::new(400., 170.))
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 200.)))
        .unwrap();
}

fn mount(widget: Widget) -> Runtime {
    let mut runtime = Runtime::new(widget).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Linux);
    frame(&mut runtime);
    assert!(runtime.focused_element().is_some());
    runtime
}

fn send(runtime: &mut Runtime, code: Code, key: KeyboardKey, control: bool) {
    if control {
        let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
            KeyboardKey::Named(NamedKey::Control),
            Code::ControlLeft,
        ));
    }
    let modifiers = if control {
        Modifiers::CONTROL
    } else {
        Modifiers::empty()
    };
    assert!(runtime.dispatch_keyboard_event(KeyboardEvent {
        modifiers,
        ..KeyboardEvent::key_down(key.clone(), code)
    }));
    let _ = runtime.dispatch_keyboard_event(KeyboardEvent {
        modifiers,
        ..KeyboardEvent::key_up(key, code)
    });
    if control {
        let _ = runtime.dispatch_keyboard_event(KeyboardEvent::key_up(
            KeyboardKey::Named(NamedKey::Control),
            Code::ControlLeft,
        ));
    }
    frame(runtime);
}

#[test]
fn an_explicit_typed_shortcut_can_rebind_arrows_and_clipboard_keys_to_forward_motion() {
    let cases = [
        (
            Code::ArrowLeft,
            KeyboardKey::Named(NamedKey::ArrowLeft),
            false,
        ),
        (Code::KeyX, KeyboardKey::Character("x".into()), true),
        (Code::KeyC, KeyboardKey::Character("c".into()), true),
        (Code::KeyV, KeyboardKey::Character("v".into()), true),
        (Code::KeyA, KeyboardKey::Character("a".into()), true),
    ];
    let mut keys = Shortcuts::<ExtendSelectionByCharacterIntent>::typed();
    let text = TextEditingController::with_text(DOCUMENT);
    for (_, key, ctrl) in &cases {
        keys.bind_logical(
            LogicalShortcutKey::new(
                key.clone(),
                if *ctrl {
                    Modifiers::CONTROL
                } else {
                    Modifiers::empty()
                },
            ),
            FORWARD,
        );
    }
    let mut runtime = mount(
        FocusableActionDetector::new(editor(&text))
            .shortcuts(Rc::new(keys))
            .into(),
    );
    send(
        &mut runtime,
        Code::ArrowRight,
        KeyboardKey::Named(NamedKey::ArrowRight),
        false,
    );
    assert_eq!(text.selection(), TextSelection::collapsed(1));
    for (code, key, ctrl) in cases {
        text.set_selection(TextSelection::collapsed(0));
        frame(&mut runtime);
        send(&mut runtime, code, key, ctrl);
        assert_eq!(text.selection(), TextSelection::collapsed(1), "{code:?}");
        assert_eq!(text.text(), DOCUMENT);
    }
}

#[test]
fn the_nearest_typed_action_can_replace_the_default_arrow_edit() {
    let mut actions = Actions::<ExtendSelectionByCharacterIntent>::typed();
    let called = Rc::new(Cell::new(0));
    let observed = called.clone();
    actions.register(FORWARD, move || observed.set(observed.get() + 1));
    let text = TextEditingController::with_text(DOCUMENT);
    let mut runtime = mount(
        FocusableActionDetector::new(editor(&text))
            .actions(Rc::new(actions))
            .into(),
    );
    send(
        &mut runtime,
        Code::ArrowRight,
        KeyboardKey::Named(NamedKey::ArrowRight),
        false,
    );
    assert_eq!(called.get(), 1);
    assert_eq!(text.selection(), TextSelection::collapsed(0));
    assert_eq!(text.text(), DOCUMENT);
}

#[test]
fn macos_host_selectors_use_the_same_mounted_action_override() {
    use serde_json::json;

    let mut actions = Actions::<ExtendSelectionByCharacterIntent>::typed();
    let calls = Rc::new(Cell::new(0));
    let recorded = calls.clone();
    actions.register(FORWARD, move || recorded.set(recorded.get() + 1));
    let text = TextEditingController::with_text(DOCUMENT);
    let mut runtime = Runtime::new(
        FocusableActionDetector::new(editor(&text))
            .actions(Rc::new(actions))
            .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::MacOs);
    frame(&mut runtime);
    let client = runtime.flutter_text_input_configuration().unwrap().0;
    let data = serde_json::to_vec(&json!({
        "method": "TextInputClient.performSelectors",
        "args": [client.get(), ["moveRight:"]],
    }))
    .unwrap();
    assert_eq!(
        runtime
            .handle_flutter_text_input_selectors_call(&data)
            .unwrap(),
        Some(true)
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(text.selection(), TextSelection::collapsed(0));
}
