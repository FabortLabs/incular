//! Radio group and rich selection infrastructure widgets.

use crate::selection::SelectionAreaController;
use crate::{
    BuildContext, CustomFocusTraversalPolicy, EditableTextContextMenuBuilder, Focus, FocusNode,
    FocusTraversalGroup, GestureDetector, KeyEventResult, MouseCursor, MouseRegion, Semantics,
    Text, TextContextMenu, Widget,
};
use incular_core::{Code, KeyState, KeyboardKey, Modifiers, NamedKey, Rect};
use incular_gestures::reading_order_by;
use incular_semantics::Role;
use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::{Rc, Weak},
};

type RadioChange<T> = Rc<dyn Fn(Option<T>)>;

/// The controlled value and callback inherited by raw radios of this type.
#[derive(Clone)]
pub struct RadioGroupRegistry<T> {
    value: Option<T>,
    change: Option<RadioChange<T>>,
    clients: Option<Rc<RadioClients<T>>>,
    rtl: bool,
}

struct RadioClient<T> {
    node: FocusNode,
    value: T,
}

struct RadioClients<T>(RefCell<Vec<Weak<RadioClient<T>>>>);

impl<T> RadioClients<T> {
    fn live(&self) -> Vec<Rc<RadioClient<T>>> {
        let mut clients = self.0.borrow_mut();
        clients.retain(|client| client.strong_count() != 0);
        clients.iter().filter_map(Weak::upgrade).collect()
    }
}

impl<T: 'static> crate::tree::RetainedState for RadioClients<T> {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

fn radio_order_rect(mut rect: Rect, rtl: bool) -> Rect {
    if rtl {
        rect.origin.x = -rect.origin.x - rect.size.width;
    }
    rect
}

impl<T> RadioGroupRegistry<T> {
    /// Supplies independent controlled state for a RawRadio that is not
    /// registered with a mounted RadioGroup's traversal policy.
    #[must_use]
    pub fn new(value: Option<T>, on_changed: impl Fn(Option<T>) + 'static) -> Self {
        Self {
            value,
            change: Some(Rc::new(on_changed)),
            clients: None,
            rtl: false,
        }
    }

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

impl<T: Clone + PartialEq + 'static> RadioGroupRegistry<T> {
    fn move_selection(&self, current: &FocusNode, forward: bool) -> bool {
        let Some(clients) = &self.clients else {
            return false;
        };
        let active = clients.live();
        if !active.iter().any(|entry| &entry.node == current) {
            return false;
        }
        if active.len() < 2 {
            return true;
        }
        let sorted = reading_order_by(active, |entry| {
            radio_order_rect(entry.node.bounds(), self.rtl)
        });
        let Some(index) = sorted.iter().position(|entry| &entry.node == current) else {
            return false;
        };
        let next = if forward {
            (index + 1) % sorted.len()
        } else if index == 0 {
            sorted.len() - 1
        } else {
            index - 1
        };
        let picked = &sorted[next];
        self.request_value(Some(picked.value.clone()));
        picked.node.request_focus();
        true
    }

    fn focus_policy(&self) -> CustomFocusTraversalPolicy {
        let registry = self.clone();
        CustomFocusTraversalPolicy::new(move |candidates, current| {
            let order = reading_order_by((0..candidates.len()).collect(), |&index| {
                radio_order_rect(candidates[index].bounds, registry.rtl)
            });
            let Some(clients) = &registry.clients else {
                return order;
            };
            let active = clients.live();
            let is_radio = |node: &FocusNode| active.iter().any(|client| &client.node == node);
            let selected = active
                .iter()
                .find(|client| Some(&client.value) == registry.value.as_ref())
                .map(|client| &client.node)
                .or_else(|| {
                    order.iter().find_map(|&index| {
                        candidates[index]
                            .node
                            .as_ref()
                            .filter(|node| is_radio(node))
                    })
                });
            order
                .into_iter()
                .filter(|&index| {
                    candidates[index].node.as_ref().is_none_or(|node| {
                        !is_radio(node) || selected == Some(node) || current == Some(node)
                    })
                })
                .collect()
        })
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
                clients: None,
                rtl: false,
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
        Widget::mount_scoped_builder(
            |_| RadioClients::<T>(RefCell::new(Vec::new())),
            move |clients, context| {
                let mut registry = value.registry.clone();
                registry.clients = Some(clients.clone());
                registry.rtl = context.text_direction() == Some(incular_config::TextDirection::Rtl);
                let policy = registry.focus_policy();
                let content = Widget::environment_scope(registry, value.child.clone());
                Semantics::new(FocusTraversalGroup::new(content).custom_policy(policy))
                    .role(Role::RadioGroup)
                    .container(true)
                    .explicit_child_nodes(true)
                    .into()
            },
        )
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

struct RawRadioMount<T> {
    node: FocusNode,
    registration: RefCell<Option<Rc<RadioClient<T>>>>,
}

impl<T: 'static> crate::tree::RetainedState for RawRadioMount<T> {
    const DISPOSE_ON_UNMOUNT: bool = true;
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
    fn dispose(&self) {
        self.registration.borrow_mut().take();
    }
}

impl<T: Clone + PartialEq + 'static> From<RawRadio<T>> for Widget {
    fn from(value: RawRadio<T>) -> Self {
        Widget::mount_scoped_builder(
            |_| RawRadioMount::<T> {
                node: FocusNode::new(),
                registration: RefCell::new(None),
            },
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
                let focus_node = value.focus_node.as_ref().unwrap_or(&mount.node);
                mount.registration.borrow_mut().take();
                if value.enabled
                    && let Some(clients) = registry
                        .as_ref()
                        .and_then(|registry| registry.clients.as_ref())
                {
                    let client = Rc::new(RadioClient {
                        node: focus_node.clone(),
                        value: value.value.clone(),
                    });
                    let mut live = clients.0.borrow_mut();
                    live.retain(|entry| entry.strong_count() != 0);
                    live.push(Rc::downgrade(&client));
                    *mount.registration.borrow_mut() = Some(client);
                }
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
                    let registry = registry.clone();
                    focus = focus.on_key_event(move |node, event| {
                        if event.state != KeyState::Down
                            || event.modifiers.intersects(
                                Modifiers::CONTROL
                                    | Modifiers::SHIFT
                                    | Modifiers::ALT
                                    | Modifiers::META,
                            )
                        {
                            return KeyEventResult::Ignored;
                        }
                        let key = match &event.key {
                            KeyboardKey::Character(value) if value == " " => Code::Space,
                            KeyboardKey::Named(NamedKey::ArrowDown) => Code::ArrowDown,
                            KeyboardKey::Named(NamedKey::ArrowRight) => Code::ArrowRight,
                            KeyboardKey::Named(NamedKey::ArrowUp) => Code::ArrowUp,
                            KeyboardKey::Named(NamedKey::ArrowLeft) => Code::ArrowLeft,
                            KeyboardKey::Named(NamedKey::Unidentified) => event.code,
                            _ => return KeyEventResult::Ignored,
                        };
                        let handled = match key {
                            Code::Space => {
                                activate();
                                true
                            }
                            Code::ArrowDown | Code::ArrowRight => registry
                                .as_ref()
                                .is_some_and(|registry| registry.move_selection(node, true)),
                            Code::ArrowUp | Code::ArrowLeft => registry
                                .as_ref()
                                .is_some_and(|registry| registry.move_selection(node, false)),
                            _ => false,
                        };
                        if handled {
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
