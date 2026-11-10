//! Flutter's menu keyboard affordances: `MenuAcceleratorLabel` and
//! `MenuAcceleratorCallbackBinding` (Alt mnemonics), the localized shortcut
//! labels `MenuItemButton.shortcut` shows, and the `CheckboxMenuButton` and
//! `RadioMenuButton` items.

use super::controls::MenuItemButton;
use crate::{Checkbox, Radio, Theme};
use incular_config::TargetPlatform;
use incular_core::{KeyboardKey, Modifiers, NamedKey};
use incular_text::TextDecoration;
use incular_widgets::{
    BuildContext, CharacterActivator, ConstrainedBox, ExcludeFocus, IgnorePointer, MainAxisSize,
    Row, ShortcutRegistry, ShortcutRegistryEntry, SingleActivator, Text, TextStyle, Widget,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn platform(context: &BuildContext<'_>) -> TargetPlatform {
    Theme::of(context).map_or_else(TargetPlatform::host, |theme| theme.core().platform)
}

/// `_usesSymbolicModifiers`.
const fn symbolic(platform: TargetPlatform) -> bool {
    matches!(platform, TargetPlatform::MacOs | TargetPlatform::Ios)
}

fn modifier_label(modifier: Modifiers, platform: TargetPlatform) -> &'static str {
    let apple = symbolic(platform);
    if modifier == Modifiers::META {
        return match platform {
            TargetPlatform::Ios | TargetPlatform::MacOs => "⌘",
            TargetPlatform::Windows => "Win",
            _ => "Meta",
        };
    }
    match (modifier, apple) {
        (m, true) if m == Modifiers::ALT => "⌥",
        (m, false) if m == Modifiers::ALT => "Alt",
        (m, true) if m == Modifiers::CONTROL => "⌃",
        (m, false) if m == Modifiers::CONTROL => "Ctrl",
        (m, true) if m == Modifiers::SHIFT => "⇧",
        _ => "Shift",
    }
}

/// `_LocalizedShortcutLabeler._getLocalizedName` with the English
/// `DefaultMaterialLocalizations`, plus Flutter's graphic equivalents.
fn key_label(key: &KeyboardKey) -> String {
    match key {
        KeyboardKey::Character(character) => character.to_uppercase(),
        KeyboardKey::Named(named) => match named {
            NamedKey::ArrowLeft => "←".into(),
            NamedKey::ArrowRight => "→".into(),
            NamedKey::ArrowUp => "↑".into(),
            NamedKey::ArrowDown => "↓".into(),
            NamedKey::Enter => "↵".into(),
            NamedKey::AltGraph => "AltGr".into(),
            NamedKey::Backspace => "Backspace".into(),
            NamedKey::CapsLock => "Caps Lock".into(),
            NamedKey::ChannelDown => "Channel Down".into(),
            NamedKey::ChannelUp => "Channel Up".into(),
            NamedKey::Delete => "Del".into(),
            NamedKey::Eject => "Eject".into(),
            NamedKey::End => "End".into(),
            NamedKey::Escape => "Esc".into(),
            NamedKey::Fn => "Fn".into(),
            NamedKey::Home => "Home".into(),
            NamedKey::Insert => "Insert".into(),
            NamedKey::NumLock => "Num Lock".into(),
            NamedKey::PageDown => "PgDown".into(),
            NamedKey::PageUp => "PgUp".into(),
            NamedKey::Power => "Power".into(),
            NamedKey::PowerOff => "Power Off".into(),
            NamedKey::PrintScreen => "Print Screen".into(),
            NamedKey::ScrollLock => "Scroll Lock".into(),
            NamedKey::Select => "Select".into(),
            other => other.to_string(),
        },
    }
}

/// The label Flutter's Material menus show for a `SingleActivator` shortcut
/// (`_LocalizedShortcutLabeler.getShortcutLabel`): "Ctrl+Shift+S" on
/// Windows and Linux, "⌃ ⇧ ⌘ S" style symbols on Apple platforms.
#[must_use]
pub fn shortcut_label(activator: &SingleActivator, platform: TargetPlatform) -> String {
    let modifiers = activator.modifiers;
    let order: &[Modifiers] = if symbolic(platform) {
        &[
            Modifiers::CONTROL,
            Modifiers::ALT,
            Modifiers::SHIFT,
            Modifiers::META,
        ]
    } else {
        &[
            Modifiers::ALT,
            Modifiers::CONTROL,
            Modifiers::META,
            Modifiers::SHIFT,
        ]
    };
    let mut parts = order
        .iter()
        .filter(|modifier| modifiers.contains(**modifier))
        .map(|modifier| modifier_label(*modifier, platform).to_owned())
        .collect::<Vec<_>>();
    let trigger = key_label(&activator.trigger);
    if !trigger.is_empty() {
        parts.push(trigger);
    }
    parts.join(if symbolic(platform) { " " } else { "+" })
}

/// The label for a `CharacterActivator` shortcut.
#[must_use]
pub fn character_shortcut_label(
    activator: &CharacterActivator,
    platform: TargetPlatform,
) -> String {
    let order: &[(bool, Modifiers)] = if symbolic(platform) {
        &[
            (activator.control, Modifiers::CONTROL),
            (activator.alt, Modifiers::ALT),
            (activator.meta, Modifiers::META),
        ]
    } else {
        &[
            (activator.alt, Modifiers::ALT),
            (activator.control, Modifiers::CONTROL),
            (activator.meta, Modifiers::META),
        ]
    };
    let mut parts = order
        .iter()
        .filter(|(held, _)| *held)
        .map(|(_, modifier)| modifier_label(*modifier, platform).to_owned())
        .collect::<Vec<_>>();
    parts.push(activator.character.clone());
    parts.join(if symbolic(platform) { " " } else { "+" })
}

/// Flutter's `MenuAcceleratorCallbackBinding`: what a
/// [`MenuAcceleratorLabel`] below a menu item invokes for its mnemonic.
#[derive(Clone)]
pub struct MenuAcceleratorCallbackBinding {
    pub on_invoke: Option<Rc<dyn Fn()>>,
    pub has_submenu: bool,
}

impl MenuAcceleratorCallbackBinding {
    /// Provides the binding to `child`.
    #[must_use]
    pub fn wrap(self, child: impl Into<Widget>) -> Widget {
        Widget::environment_scope(self, child.into())
    }

    #[must_use]
    pub fn maybe_of(context: &BuildContext<'_>) -> Option<Self> {
        context.find::<Self>()
    }
}

/// Builds a [`MenuAcceleratorLabel`] from its display label and the index of
/// the character to mark, or `None` while Alt is not held.
pub type MenuAcceleratorChildBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, &str, Option<usize>) -> Widget>;

/// Flutter's `MenuAcceleratorLabel`: a label whose `&`-marked character
/// is underlined while Alt is held and, then, activates the enclosing menu
/// item when typed with Alt. `&&` shows a literal ampersand. Apple
/// platforms have no menu mnemonics, so there the label only strips the
/// markers.
#[derive(Clone)]
pub struct MenuAcceleratorLabel {
    label: String,
    builder: MenuAcceleratorChildBuilder,
}

impl MenuAcceleratorLabel {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            builder: Rc::new(Self::default_label_builder),
        }
    }

    #[must_use]
    pub fn builder(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, &str, Option<usize>) -> Widget + 'static,
    ) -> Self {
        self.builder = Rc::new(builder);
        self
    }

    /// Flutter's `MenuAcceleratorLabel.stripAcceleratorMarkers`: the label
    /// without markers, and the character index of the first accelerator.
    #[must_use]
    pub fn strip_accelerator_markers(label: &str) -> (String, Option<usize>) {
        let characters = label.chars().collect::<Vec<_>>();
        let mut display = String::with_capacity(label.len());
        let mut accelerator = None;
        let mut quoted = 0;
        let mut last_was_ampersand = false;
        for (index, character) in characters.iter().enumerate() {
            if last_was_ampersand {
                last_was_ampersand = false;
                display.push(*character);
                continue;
            }
            if *character != '&' {
                display.push(*character);
                continue;
            }
            if index == characters.len() - 1 {
                break;
            }
            last_was_ampersand = true;
            let next = characters[index + 1];
            if accelerator.is_none() && next != '&' && !next.is_whitespace() {
                accelerator = Some(index - quoted);
            }
            quoted += 1;
        }
        (display, accelerator)
    }

    /// `MenuAcceleratorLabel.defaultLabelBuilder`: the label with the
    /// accelerator character underlined.
    #[must_use]
    pub fn default_label_builder(
        _context: &BuildContext<'_>,
        label: &str,
        index: Option<usize>,
    ) -> Widget {
        let Some(index) = index else {
            return Text::new(label).into();
        };
        let characters = label.chars().collect::<Vec<_>>();
        if index >= characters.len() {
            return Text::new(label).into();
        }
        let before = characters[..index].iter().collect::<String>();
        let marked = characters[index].to_string();
        let after = characters[index + 1..].iter().collect::<String>();
        let mut parts: Vec<Widget> = Vec::with_capacity(3);
        if !before.is_empty() {
            parts.push(Text::new(before).into());
        }
        parts.push(
            Text::new(marked)
                .style(TextStyle::new().decoration(TextDecoration::UNDERLINE))
                .into(),
        );
        if !after.is_empty() {
            parts.push(Text::new(after).into());
        }
        Row::new(parts).main_axis_size(MainAxisSize::Min).into()
    }
}

struct AcceleratorState {
    revision: Rc<Cell<u64>>,
    entry: RefCell<Option<ShortcutRegistryEntry>>,
    listener: RefCell<Option<ShortcutRegistryEntry>>,
}

impl From<MenuAcceleratorLabel> for Widget {
    fn from(value: MenuAcceleratorLabel) -> Self {
        let (display, index) = MenuAcceleratorLabel::strip_accelerator_markers(&value.label);
        let state = Rc::new(AcceleratorState {
            revision: Rc::new(Cell::new(0)),
            entry: RefCell::new(None),
            listener: RefCell::new(None),
        });
        let revision = state.revision.clone();
        Widget::stateful_builder(revision, move |context| {
            let supported = !matches!(
                platform(context),
                TargetPlatform::MacOs | TargetPlatform::Ios
            );
            let registry = ShortcutRegistry::maybe_of(context).filter(|_| supported);
            let Some(registry) = registry else {
                return (value.builder)(context, &display, None);
            };
            if state.listener.borrow().is_none() {
                let weak = Rc::downgrade(&state);
                let handle = registry.add_modifier_listener(move |_| {
                    if let Some(state) = weak.upgrade() {
                        state.revision.set(state.revision.get().wrapping_add(1));
                    }
                });
                *state.listener.borrow_mut() = Some(handle);
            }
            let show = registry.modifiers().alt();
            let binding = MenuAcceleratorCallbackBinding::maybe_of(context);
            let entry = match (show, index, binding.and_then(|binding| binding.on_invoke)) {
                (true, Some(index), Some(on_invoke)) => {
                    display.chars().nth(index).map(|character| {
                        registry.add(
                            CharacterActivator::new(character.to_lowercase().to_string()).alt(true),
                            move || on_invoke(),
                        )
                    })
                }
                _ => None,
            };
            *state.entry.borrow_mut() = entry;
            (value.builder)(context, &display, if show { index } else { None })
        })
    }
}

/// Flutter's `CheckboxMenuButton`: a menu item led by a checkbox. Pressing
/// it reports the next value: `true` from `false`, `false` (or `None` when
/// `tristate`) from `true`, and `false` from `None`.
#[derive(Clone)]
pub struct CheckboxMenuButton {
    value: Option<bool>,
    tristate: bool,
    on_changed: Option<Rc<dyn Fn(Option<bool>)>>,
    child: Widget,
    shortcut: Option<SingleActivator>,
    trailing_icon: Option<Widget>,
    close_on_activate: bool,
}

impl CheckboxMenuButton {
    #[must_use]
    pub fn new(value: Option<bool>, child: impl Into<Widget>) -> Self {
        Self {
            value,
            tristate: false,
            on_changed: None,
            child: child.into(),
            shortcut: None,
            trailing_icon: None,
            close_on_activate: true,
        }
    }

    #[must_use]
    pub fn tristate(mut self, tristate: bool) -> Self {
        self.tristate = tristate;
        self
    }

    #[must_use]
    pub fn on_changed(mut self, on_changed: impl Fn(Option<bool>) + 'static) -> Self {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    #[must_use]
    pub fn shortcut(mut self, shortcut: SingleActivator) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    #[must_use]
    pub fn trailing_icon(mut self, icon: impl Into<Widget>) -> Self {
        self.trailing_icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn close_on_activate(mut self, close: bool) -> Self {
        self.close_on_activate = close;
        self
    }
}

/// Flutter's `Checkbox.width`.
const CHECKBOX_WIDTH: f32 = 18.0;

impl From<CheckboxMenuButton> for Widget {
    fn from(value: CheckboxMenuButton) -> Self {
        let mut checkbox = Checkbox::new(value.value.unwrap_or(false))
            .value(value.value)
            .tristate(value.tristate);
        if let Some(on_changed) = value.on_changed.clone() {
            checkbox = checkbox.on_changed(move |next| on_changed(next));
        }
        let leading = ExcludeFocus::new(IgnorePointer::new(ConstrainedBox::new(
            incular_config::Constraints::new(0.0, CHECKBOX_WIDTH, 0.0, CHECKBOX_WIDTH),
            checkbox,
        )));
        let mut item = MenuItemButton::new(value.child)
            .leading_icon(leading)
            .close_on_activate(value.close_on_activate);
        if let Some(shortcut) = value.shortcut {
            item = item.shortcut(shortcut);
        }
        if let Some(icon) = value.trailing_icon {
            item = item.trailing_icon(icon);
        }
        if let Some(on_changed) = value.on_changed {
            let current = value.value;
            let tristate = value.tristate;
            item = item.on_pressed(move || {
                on_changed(match current {
                    Some(false) => Some(true),
                    Some(true) => {
                        if tristate {
                            None
                        } else {
                            Some(false)
                        }
                    }
                    None => Some(false),
                });
            });
        } else {
            item = item.enabled(false);
        }
        item.into()
    }
}

/// Flutter's `RadioMenuButton`: a menu item led by a radio button that
/// reports its value when pressed.
#[derive(Clone)]
pub struct RadioMenuButton<T: Clone + PartialEq + 'static> {
    value: T,
    group_value: Option<T>,
    toggleable: bool,
    on_changed: Option<Rc<dyn Fn(Option<T>)>>,
    child: Widget,
    shortcut: Option<SingleActivator>,
    trailing_icon: Option<Widget>,
    close_on_activate: bool,
}

impl<T: Clone + PartialEq + 'static> RadioMenuButton<T> {
    #[must_use]
    pub fn new(value: T, group_value: Option<T>, child: impl Into<Widget>) -> Self {
        Self {
            value,
            group_value,
            toggleable: false,
            on_changed: None,
            child: child.into(),
            shortcut: None,
            trailing_icon: None,
            close_on_activate: true,
        }
    }

    /// Pressing the selected item reports `None` (Flutter's `toggleable`).
    #[must_use]
    pub fn toggleable(mut self, toggleable: bool) -> Self {
        self.toggleable = toggleable;
        self
    }

    #[must_use]
    pub fn on_changed(mut self, on_changed: impl Fn(Option<T>) + 'static) -> Self {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    #[must_use]
    pub fn shortcut(mut self, shortcut: SingleActivator) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    #[must_use]
    pub fn trailing_icon(mut self, icon: impl Into<Widget>) -> Self {
        self.trailing_icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn close_on_activate(mut self, close: bool) -> Self {
        self.close_on_activate = close;
        self
    }
}

impl<T: Clone + PartialEq + 'static> From<RadioMenuButton<T>> for Widget {
    fn from(value: RadioMenuButton<T>) -> Self {
        let selected = value.group_value.as_ref() == Some(&value.value);
        let _ = selected;
        let radio = Radio::new(value.value.clone()).group_value(value.group_value.clone());
        let leading = ExcludeFocus::new(IgnorePointer::new(ConstrainedBox::new(
            incular_config::Constraints::new(0.0, CHECKBOX_WIDTH, 0.0, CHECKBOX_WIDTH),
            radio,
        )));
        let mut item = MenuItemButton::new(value.child)
            .leading_icon(leading)
            .close_on_activate(value.close_on_activate);
        if let Some(shortcut) = value.shortcut {
            item = item.shortcut(shortcut);
        }
        if let Some(icon) = value.trailing_icon {
            item = item.trailing_icon(icon);
        }
        if let Some(on_changed) = value.on_changed {
            let own = value.value.clone();
            let toggleable = value.toggleable;
            item = item.on_pressed(move || {
                if toggleable && selected {
                    on_changed(None);
                } else {
                    on_changed(Some(own.clone()));
                }
            });
        } else {
            item = item.enabled(false);
        }
        item.into()
    }
}
