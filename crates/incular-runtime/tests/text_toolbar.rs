//! Flutter's selection toolbar triggers on text fields: a touch double tap
//! selects a word and shows the toolbar (`onDoubleTapDown` with
//! `shouldShowSelectionToolbar`), while a mouse double click does not; typed
//! text hides an open toolbar (`updateEditingValue`).

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Size,
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_widgets::{
    ContextMenuButtonType, EditableText, IgnorePointer, SizedBox, TextContextMenu, TextDirection,
    Widget, internal::TextEditingController,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

type Shown = Rc<RefCell<Vec<TextContextMenu>>>;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(300., 40.))
}

fn mount(text: &str) -> (Runtime, TextEditingController, Shown) {
    let controller = TextEditingController::with_text(text);
    let shown: Shown = Rc::default();
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(300., 32.))
        .context_menu_builder({
            let shown = shown.clone();
            move |_, menu| {
                shown.borrow_mut().push(menu.clone());
                IgnorePointer::new(SizedBox::shrink()).into()
            }
        });
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller, shown)
}

fn tap(runtime: &mut Runtime, position: Offset, kind: PointerDeviceKind, at: Instant) {
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input_at(
            InputEvent::PointerWithMetadata {
                pointer: 1,
                device: 3,
                kind,
                buttons,
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: PointerSampleMetadata::default(),
                phase,
                position,
            },
            at,
        );
    }
}

/// Two quick taps on the same spot: a double tap.
fn double_tap(runtime: &mut Runtime, kind: PointerDeviceKind) -> Instant {
    let position = Offset::new(12., 10.);
    let start = Instant::now();
    tap(runtime, position, kind, start);
    let second = start + Duration::from_millis(120);
    tap(runtime, position, kind, second);
    second + Duration::from_millis(10)
}

#[test]
fn touch_double_tap_selects_the_word_and_shows_the_toolbar() {
    let (mut runtime, controller, shown) = mount("hello world");
    let at = double_tap(&mut runtime, PointerDeviceKind::Touch);
    runtime.run_frame_at(bounds(), at).unwrap();
    assert_eq!(controller.selection().base, 0);
    assert_eq!(controller.selection().extent, 5);
    let menu = shown.borrow().last().cloned().expect("toolbar shown");
    assert!(
        menu.button_items
            .iter()
            .any(|item| item.button_type() == ContextMenuButtonType::Copy),
        "{menu:?}"
    );
}

#[test]
fn mouse_double_click_selects_the_word_without_the_toolbar() {
    let (mut runtime, controller, shown) = mount("hello world");
    let at = double_tap(&mut runtime, PointerDeviceKind::Mouse);
    runtime.run_frame_at(bounds(), at).unwrap();
    assert_eq!(controller.selection().extent, 5);
    assert!(shown.borrow().is_empty());
}

/// Whether the focused field's toolbar is open.
fn toolbar_open(runtime: &Runtime) -> bool {
    let field = runtime.focused_element().expect("focused field");
    runtime
        .tree()
        .text_field_context_menu(field)
        .is_some_and(|menu| menu.is_showing())
}

#[test]
fn typing_text_hides_the_open_toolbar() {
    let (mut runtime, _controller, _shown) = mount("hello world");
    let at = double_tap(&mut runtime, PointerDeviceKind::Touch);
    runtime.run_frame_at(bounds(), at).unwrap();
    assert!(toolbar_open(&runtime));
    let _ = runtime.handle_input(InputEvent::Text("x".into()));
    runtime.run_frame_at(bounds(), at).unwrap();
    assert!(!toolbar_open(&runtime));
}

#[test]
fn native_text_toolbar_request_shows_once_without_changing_selection() {
    use incular_platform::TextInputCommand;
    use incular_text::TextSelection;
    use serde_json::json;

    let (mut runtime, controller, shown) = mount("hello world");
    let start = Instant::now();
    tap(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Mouse,
        start,
    );
    runtime.run_frame_at(bounds(), start).unwrap();
    let client = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .expect("mounted input attachment");
    let selection = TextSelection::new(0, 5);
    controller.set_selection(selection);
    runtime.run_frame(bounds()).unwrap();

    let show = |runtime: &mut Runtime, id| {
        let wire = json!({"method":"TextInputClient.showToolbar", "args":[id,0,1]});
        runtime
            .handle_flutter_text_toolbar_call(wire.to_string().as_bytes())
            .unwrap()
    };
    assert_eq!(show(&mut runtime, client.get() + 1), Some(false));
    assert!(!toolbar_open(&runtime));
    assert_eq!(show(&mut runtime, client.get()), Some(true));
    runtime.run_frame(bounds()).unwrap();
    let builds = shown.borrow().len();
    assert!(builds > 0);
    assert_eq!(controller.selection(), selection);
    assert_eq!(show(&mut runtime, client.get()), Some(false));
    runtime.run_frame(bounds()).unwrap();
    assert_eq!(shown.borrow().len(), builds);
}

#[test]
fn direct_context_menu_replaces_the_focused_editor_toolbar_without_losing_focus() {
    use incular_core::Color;
    use incular_platform::TextInputCommand;
    use incular_text::TextSelection;
    use incular_widgets::{ContextMenuController, internal::Key};
    use serde_json::json;

    let editor = TextEditingController::with_text("hello world");
    let direct = ContextMenuController::new();
    let manual = Key::from("direct menu");
    let built_in = Key::from("editor menu");
    let field = EditableText::new(editor.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(400., 40.))
        .context_menu_builder({
            let built_in = built_in.clone();
            move |_, _| Widget::box_(Size::new(70., 20.), Color::WHITE).with_key(built_in.clone())
        });
    let mut runtime = Runtime::new(direct.host(field)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime.run_frame(bounds()).unwrap();
    tap(
        &mut runtime,
        Offset::new(12., 10.),
        PointerDeviceKind::Mouse,
        Instant::now(),
    );
    runtime.run_frame(bounds()).unwrap();
    let field = runtime.focused_element().unwrap();
    let state = TextSelection::new(0, 5);
    editor.set_selection(state);
    runtime.run_frame(bounds()).unwrap();
    let client = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .expect("mounted input attachment");
    let request =
        json!({"method":"TextInputClient.showToolbar", "args":[client.get(),0,1]}).to_string();
    let was_visible = |runtime: &Runtime, key: &Key| runtime.tree().element_with_key(key).is_some();

    assert_eq!(
        runtime
            .handle_flutter_text_toolbar_call(request.as_bytes())
            .unwrap(),
        Some(true)
    );
    runtime.run_frame(bounds()).unwrap();
    assert!(was_visible(&runtime, &built_in));

    direct.show({
        let manual = manual.clone();
        move |_| Widget::box_(Size::new(48., 20.), Color::WHITE).with_key(manual.clone())
    });
    runtime.run_frame(bounds()).unwrap();
    assert!(!was_visible(&runtime, &built_in));
    assert!(was_visible(&runtime, &manual));
    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(editor.selection(), state);

    assert_eq!(
        runtime
            .handle_flutter_text_toolbar_call(request.as_bytes())
            .unwrap(),
        Some(true)
    );
    runtime.run_frame(bounds()).unwrap();
    assert!(was_visible(&runtime, &built_in));
    assert!(!was_visible(&runtime, &manual));
    assert!(!direct.remove());
    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(editor.selection(), state);
}
