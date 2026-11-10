//! Flutter's `MenuAcceleratorLabel`, `MenuItemButton.shortcut` labels and
//! the check/radio menu items.

use incular_config::{Constraints, TargetPlatform};
use incular_core::{Code, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_material::{
    CheckboxMenuButton, MenuAcceleratorCallbackBinding, MenuAcceleratorLabel, RadioMenuButton,
    Theme, ThemeData, shortcut_label,
};
use incular_widgets::{
    Directionality, ShortcutRegistrar, ShortcutRegistry, SingleActivator, Text, TextDirection,
    Widget, internal::WidgetTree,
};
use std::{cell::Cell, rc::Rc};

#[test]
fn strip_accelerator_markers_matches_flutter() {
    let strip = MenuAcceleratorLabel::strip_accelerator_markers;
    assert_eq!(strip("&File"), ("File".to_owned(), Some(0)));
    assert_eq!(strip("Save &As"), ("Save As".to_owned(), Some(5)));
    // `&&` is a literal ampersand, and only the first marker counts.
    assert_eq!(
        strip("Tom && &Jerry &x"),
        ("Tom & Jerry x".to_owned(), Some(6))
    );
    // A trailing marker and a marked space are not accelerators.
    assert_eq!(strip("End&"), ("End".to_owned(), None));
    assert_eq!(strip("A& b"), ("A b".to_owned(), None));
}

#[test]
fn shortcut_labels_follow_the_platform() {
    let save = SingleActivator::new(
        KeyboardKey::Character("s".into()),
        Modifiers::CONTROL | Modifiers::SHIFT,
    );
    assert_eq!(
        shortcut_label(&save, TargetPlatform::Windows),
        "Ctrl+Shift+S"
    );
    assert_eq!(shortcut_label(&save, TargetPlatform::MacOs), "⌃ ⇧ S");
    let quit = SingleActivator::new(KeyboardKey::Character("q".into()), Modifiers::META);
    assert_eq!(shortcut_label(&quit, TargetPlatform::MacOs), "⌘ Q");
    assert_eq!(shortcut_label(&quit, TargetPlatform::Windows), "Win+Q");
    assert_eq!(shortcut_label(&quit, TargetPlatform::Linux), "Meta+Q");
    let back = SingleActivator::new(KeyboardKey::Named(NamedKey::ArrowLeft), Modifiers::ALT);
    assert_eq!(shortcut_label(&back, TargetPlatform::Linux), "Alt+←");
}

fn alt(down: bool) -> KeyboardEvent {
    let mut event = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Alt), Code::AltLeft);
    if !down {
        event.state = incular_core::KeyState::Up;
    }
    event.modifiers = if down {
        Modifiers::ALT
    } else {
        Modifiers::empty()
    };
    event
}

#[test]
fn accelerator_registers_while_alt_is_held_and_invokes_its_item() {
    let invoked = Rc::new(Cell::new(0));
    let registry = ShortcutRegistry::new();
    let label = MenuAcceleratorCallbackBinding {
        on_invoke: Some(Rc::new({
            let invoked = invoked.clone();
            move || invoked.set(invoked.get() + 1)
        })),
        has_submenu: false,
    }
    .wrap(MenuAcceleratorLabel::new("&File"));
    let theme = ThemeData::light().with_platform(TargetPlatform::Windows);
    let root: Widget = ShortcutRegistrar::with_registry(
        registry.clone(),
        Theme::new(theme, Directionality::new(TextDirection::Ltr, label)),
    )
    .into();
    let mut tree = WidgetTree::new();
    tree.mount(root).unwrap();
    let bounds = Constraints::tight(Size::new(200.0, 40.0));
    tree.layout(bounds).unwrap();
    assert!(registry.is_empty(), "no accelerator without Alt");

    registry.handle_key(&alt(true));
    tree.layout(bounds).unwrap();
    assert_eq!(registry.len(), 1);

    let mut f = KeyboardEvent::key_down(KeyboardKey::Character("f".into()), Code::KeyF);
    f.modifiers = Modifiers::ALT;
    assert!(registry.handle_key(&f));
    assert_eq!(invoked.get(), 1);

    registry.handle_key(&alt(false));
    tree.layout(bounds).unwrap();
    assert!(registry.is_empty(), "releasing Alt removes the accelerator");
}

#[test]
fn macos_has_no_menu_accelerators() {
    let registry = ShortcutRegistry::new();
    let label = MenuAcceleratorCallbackBinding {
        on_invoke: Some(Rc::new(|| {})),
        has_submenu: false,
    }
    .wrap(MenuAcceleratorLabel::new("&File"));
    let theme = ThemeData::light().with_platform(TargetPlatform::MacOs);
    let root: Widget = ShortcutRegistrar::with_registry(
        registry.clone(),
        Theme::new(theme, Directionality::new(TextDirection::Ltr, label)),
    )
    .into();
    let mut tree = WidgetTree::new();
    tree.mount(root).unwrap();
    registry.handle_key(&alt(true));
    tree.layout(Constraints::tight(Size::new(200.0, 40.0)))
        .unwrap();
    assert!(registry.is_empty());
}

#[test]
fn check_and_radio_menu_items_build() {
    let checkbox = CheckboxMenuButton::new(Some(true), Text::new("Wrap")).on_changed(|_| {});
    let radio = RadioMenuButton::new(1, Some(1), Text::new("One")).on_changed(|_| {});
    for item in [Widget::from(checkbox), Widget::from(radio)] {
        let mut tree = WidgetTree::new();
        tree.mount(Directionality::new(TextDirection::Ltr, item).into())
            .unwrap();
        tree.layout(Constraints::tight(Size::new(240.0, 48.0)))
            .unwrap();
    }
}
