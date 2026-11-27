use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Actions, ActivateIntent, FocusableActionDetector, Shortcuts, SingleActivator, SizedBox, Widget,
    WidgetsApp,
};

fn mount(app: WidgetsApp) -> Runtime {
    let mut runtime = Runtime::new(app.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    runtime
}

fn checkbox(value: Rc<Cell<bool>>) -> Widget {
    let mut actions = Actions::<ActivateIntent>::typed();
    actions.register(ActivateIntent, move || value.set(!value.get()));
    FocusableActionDetector::new(SizedBox::square(25.))
        .actions(Rc::new(actions))
        .autofocus(true)
        .into()
}

fn press(runtime: &mut Runtime, key: KeyboardKey, code: Code, modifiers: Modifiers) -> bool {
    let down = KeyboardEvent {
        modifiers,
        ..KeyboardEvent::key_down(key.clone(), code)
    };
    let up = KeyboardEvent {
        modifiers,
        ..KeyboardEvent::key_up(key, code)
    };
    let result = runtime.dispatch_keyboard_event(down);
    assert!(!runtime.dispatch_keyboard_event(up));
    result
}

fn space() -> KeyboardKey {
    KeyboardKey::Character(" ".to_owned())
}

#[test]
fn the_builder_only_app_supplies_default_space_activation_to_focused_actions() {
    let checked = Rc::new(Cell::new(false));
    let body = checkbox(checked.clone());
    let mut runtime = mount(WidgetsApp::empty().builder(move |_| body.clone()));
    assert!(press(
        &mut runtime,
        space(),
        Code::Space,
        Modifiers::empty()
    ));
    assert!(checked.get());
}

#[test]
fn custom_app_shortcuts_and_actions_replace_the_default_activation_map() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Command {
        Overridden,
    }
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut actions = Actions::<Command>::typed();
    actions.register(Command::Overridden, move || count.set(count.get() + 1));
    let mut shortcuts = Shortcuts::<Command>::typed();
    shortcuts.bind_activator(
        SingleActivator::new(space(), Modifiers::empty()),
        Command::Overridden,
    );
    let checked = Rc::new(Cell::new(false));
    let body = checkbox(checked.clone());
    let app = WidgetsApp::empty()
        .actions(Rc::new(actions))
        .shortcuts(Rc::new(shortcuts))
        .builder(move |_| body.clone());
    let mut runtime = mount(app);
    assert!(press(
        &mut runtime,
        space(),
        Code::Space,
        Modifiers::empty()
    ));
    assert!(!checked.get());
    assert_eq!(calls.get(), 1);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn logical_native_activation_distinguishes_modifier_chords_and_supports_select() {
    let checked = Rc::new(Cell::new(false));
    let body = checkbox(checked.clone());
    let mut runtime = mount(WidgetsApp::empty().builder(move |_| body.clone()));
    for (key, code) in [
        (space(), Code::Space),
        (KeyboardKey::Named(NamedKey::Enter), Code::Enter),
        (KeyboardKey::Named(NamedKey::Enter), Code::NumpadEnter),
        (KeyboardKey::Named(NamedKey::Select), Code::Select),
    ] {
        checked.set(false);
        assert!(!press(&mut runtime, key.clone(), code, Modifiers::CONTROL));
        assert!(!checked.get());
        assert!(press(&mut runtime, key, code, Modifiers::empty()));
        assert!(checked.get());
    }
}
