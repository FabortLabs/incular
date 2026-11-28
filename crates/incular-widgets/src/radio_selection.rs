//! Radio group and rich selection infrastructure widgets.

use crate::selection::SelectionAreaController;
use crate::{
    BuildContext, EditableTextContextMenuBuilder, Focus, FocusNode, GestureDetector,
    KeyEventResult, MouseCursor, MouseRegion, Semantics, Text, TextContextMenu, Widget,
};
use incular_core::{Code, KeyState, Modifiers};
use incular_semantics::Role;
use std::{fmt, rc::Rc};

type RadioChange<T> = Rc<dyn Fn(Option<T>)>;

/// The controlled value and callback inherited by raw radios of this type.
#[derive(Clone)]
pub struct RadioGroupRegistry<T> {
    value: Option<T>,
    change: Option<RadioChange<T>>,
}

impl<T> RadioGroupRegistry<T> {
    #[must_use]
    pub fn group_value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Calls the application's controlled value callback, if installed.
    pub fn request_value(&self, value: Option<T>) -> bool {
        if let Some(change) = &self.change {
            change(value);
            true
        } else {
            false
        }
    }
}

/// Coordinates mutually exclusive selection for a group of radio buttons.
#[derive(Clone)]
pub struct RadioGroup<T: Clone + PartialEq + 'static> {
    registry: RadioGroupRegistry<T>,
    child: Widget,
}

impl<T: Clone + PartialEq + 'static> RadioGroup<T> {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            registry: RadioGroupRegistry {
                value: None,
                change: None,
            },
            child: child.into(),
        }
    }

    #[must_use]
    pub fn value(mut self, value: Option<T>) -> Self {
        self.registry.value = value;
        self
    }

    #[must_use]
    pub fn group_value(self, value: Option<T>) -> Self {
        self.value(value)
    }

    #[must_use]
    pub fn on_changed(self, callback: impl Fn(T) + 'static) -> Self {
        self.on_changed_optional(move |value| {
            if let Some(value) = value {
                callback(value);
            }
        })
    }

    /// Handles selection as well as clearing a toggleable selected radio.
    #[must_use]
    pub fn on_changed_optional(mut self, callback: impl Fn(Option<T>) + 'static) -> Self {
        self.registry.change = Some(Rc::new(callback));
        self
    }

    /// The nearest mounted group of the same value type.
    #[must_use]
    pub fn maybe_of(context: &BuildContext<'_>) -> Option<RadioGroupRegistry<T>> {
        context.depend_on::<RadioGroupRegistry<T>>()
    }
}

impl<T: Clone + PartialEq + 'static> From<RadioGroup<T>> for Widget {
    fn from(value: RadioGroup<T>) -> Self {
        Semantics::new(Widget::environment_scope(value.registry, value.child))
            .role(Role::RadioGroup)
            .container(true)
            .explicit_child_nodes(true)
            .into()
    }
}

/// The operating state of a raw radio's visual, independent of any theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawRadioState {
    pub selected: bool,
    pub enabled: bool,
}

/// A raw radio button control.
#[derive(Clone)]
pub struct RawRadio<T: Clone + PartialEq + 'static> {
    value: T,
    group_value: Option<T>,
    on_changed: Option<RadioChange<T>>,
    registry: Option<RadioGroupRegistry<T>>,
    focus_node: Option<FocusNode>,
    enabled: bool,
    toggleable: bool,
    autofocus: bool,
    mouse_cursor: MouseCursor,
    visual: Option<Rc<dyn Fn(RawRadioState) -> Widget>>,
}

impl<T: Clone + PartialEq + 'static> RawRadio<T> {
    #[must_use]
    pub fn new(value: T, group_value: Option<T>) -> Self {
        Self {
            value,
            group_value,
            on_changed: None,
            registry: None,
            focus_node: None,
            enabled: true,
            toggleable: false,
            autofocus: false,
            mouse_cursor: MouseCursor::Defer,
            visual: None,
        }
    }

    #[must_use]
    pub fn on_changed(self, callback: impl Fn(T) + 'static) -> Self {
        self.on_changed_optional(move |value| {
            if let Some(value) = value {
                callback(value);
            }
        })
    }

    #[must_use]
    pub fn on_changed_optional(mut self, callback: impl Fn(Option<T>) + 'static) -> Self {
        self.on_changed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn toggleable(mut self, toggleable: bool) -> Self {
        self.toggleable = toggleable;
        self
    }

    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    #[must_use]
    pub fn focus_node(mut self, node: FocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    #[must_use]
    pub fn mouse_cursor(mut self, cursor: MouseCursor) -> Self {
        self.mouse_cursor = cursor;
        self
    }

    #[must_use]
    pub fn group_registry(mut self, registry: RadioGroupRegistry<T>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Supplies the visual child, which can respond to controlled state
    /// without depending on a design system.
    #[must_use]
    pub fn builder(mut self, visual: impl Fn(RawRadioState) -> Widget + 'static) -> Self {
        self.visual = Some(Rc::new(visual));
        self
    }
}

struct RawRadioMount(FocusNode);
impl crate::tree::RetainedState for RawRadioMount {
    fn rebind_revision(&self, _revision: Rc<std::cell::Cell<u64>>) {}
}

impl<T: Clone + PartialEq + 'static> From<RawRadio<T>> for Widget {
    fn from(value: RawRadio<T>) -> Self {
        Widget::mount_scoped_builder(
            |_| RawRadioMount(FocusNode::new()),
            move |mount, context| {
                let registry = value
                    .registry
                    .clone()
                    .or_else(|| RadioGroup::maybe_of(context));
                let selected = registry.as_ref().map_or_else(
                    || value.group_value.as_ref() == Some(&value.value),
                    |registry| registry.group_value() == Some(&value.value),
                );
                let change = registry
                    .as_ref()
                    .and_then(|group| group.change.clone())
                    .or_else(|| value.on_changed.clone());
                let focus_node = value.focus_node.as_ref().unwrap_or(&mount.0);
                let activate = change.filter(|_| value.enabled).map(|change| {
                    let selected_value = value.value.clone();
                    let toggleable = value.toggleable;
                    Rc::new(move || {
                        if !selected {
                            change(Some(selected_value.clone()));
                        } else if toggleable {
                            change(None);
                        }
                    }) as Rc<dyn Fn()>
                });
                let visual = value.visual.as_ref().map_or_else(
                    || Text::new(if selected { "(•)" } else { "( )" }).into(),
                    |visual| {
                        visual(RawRadioState {
                            selected,
                            enabled: value.enabled,
                        })
                    },
                );
                let mut gesture = GestureDetector::new(visual);
                if let Some(activate) = &activate {
                    let activate = activate.clone();
                    let node = focus_node.clone();
                    gesture = gesture.on_tap(move || {
                        node.request_focus();
                        activate();
                    });
                }
                let cursor = MouseRegion::new(gesture).cursor(value.mouse_cursor);
                let mut focus = Focus::new(cursor)
                    .node(focus_node.clone())
                    .can_request_focus(value.enabled)
                    .autofocus(value.enabled && value.autofocus);
                if let Some(activate) = &activate {
                    let activate = activate.clone();
                    focus = focus.on_key_event(move |_, event| {
                        if event.code == Code::Space
                            && event.state == KeyState::Down
                            && !event.modifiers.intersects(
                                Modifiers::CONTROL
                                    | Modifiers::SHIFT
                                    | Modifiers::ALT
                                    | Modifiers::META,
                            )
                        {
                            activate();
                            KeyEventResult::Handled
                        } else {
                            KeyEventResult::Ignored
                        }
                    });
                }
                let mut semantics = Semantics::new(focus)
                    .role(Role::Radio)
                    .checked(selected)
                    .in_mutually_exclusive_group(true)
                    .enabled(value.enabled);
                if let Some(activate) = activate {
                    semantics = semantics.on_tap(move || activate());
                }
                semantics.into()
            },
        )
    }
}

/// An area that supports pointer-based text selection.
#[derive(Clone)]
pub struct SelectableRegion {
    child: Widget,
    context_menu_builder: Option<EditableTextContextMenuBuilder>,
}

impl fmt::Debug for SelectableRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectableRegion")
            .field("child", &self.child)
            .field(
                "has_context_menu_builder",
                &self.context_menu_builder.is_some(),
            )
            .finish()
    }
}

impl PartialEq for SelectableRegion {
    fn eq(&self, other: &Self) -> bool {
        self.child == other.child
            && match (&self.context_menu_builder, &other.context_menu_builder) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}

impl SelectableRegion {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            context_menu_builder: None,
        }
    }

    /// Builds the selection toolbar at the supplied anchors. The menu model
    /// contains the platform's ordered actions without prescribing a style.
    #[must_use]
    pub fn context_menu_builder(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, &TextContextMenu) -> Widget + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(builder));
        self
    }
}

impl From<SelectableRegion> for Widget {
    fn from(value: SelectableRegion) -> Self {
        let controller = SelectionAreaController::new();
        match value.context_menu_builder {
            Some(builder) => {
                Widget::selection_area_with_context_menu(controller, value.child, builder)
            }
            None => Widget::selection_area(controller, value.child),
        }
    }
}
