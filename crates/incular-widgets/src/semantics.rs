//! Accessible semantic annotation widgets.

use incular_semantics::{
    AccessibilityFocusBlockType, CustomSemanticsAction, SemanticAction, SemanticActionKind,
    SemanticRole, SemanticState,
};
use smart_default::SmartDefault;
use std::rc::Rc;
use typed_builder::TypedBuilder;

use crate::Widget;

#[derive(Clone)]
pub(crate) struct CustomActionBinding {
    pub(crate) action: CustomSemanticsAction,
    pub(crate) callback: Option<Rc<dyn Fn() + 'static>>,
}

impl std::fmt::Debug for CustomActionBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CustomActionBinding")
            .field(&self.action)
            .finish()
    }
}

impl PartialEq for CustomActionBinding {
    fn eq(&self, other: &Self) -> bool {
        self.action == other.action
            && match (&self.callback, &other.callback) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

/// An accessible semantic annotation on a widget subtree.
#[derive(Clone, Default, TypedBuilder)]
pub struct Semantics {
    #[builder(default, setter(strip_option))]
    role: Option<SemanticRole>,
    #[builder(default, setter(strip_option, into))]
    label: Option<String>,
    #[builder(default, setter(strip_option, into))]
    value: Option<String>,
    #[builder(default, setter(strip_option, into))]
    description: Option<String>,
    #[builder(default)]
    actions: Vec<SemanticAction>,
    #[builder(default, setter(skip))]
    custom_actions: Vec<CustomActionBinding>,
    #[builder(default)]
    on_tap: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_increase: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_decrease: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_scroll_forward: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_scroll_backward: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_long_press: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    state: SemanticState,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl Semantics {
    /// Replaces the annotated child.
    pub(crate) fn with_child(mut self, child: impl Into<Widget>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Creates a Semantics annotation.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            role: None,
            label: None,
            value: None,
            description: None,
            actions: Vec::new(),
            custom_actions: Vec::new(),
            on_tap: None,
            on_increase: None,
            on_decrease: None,
            on_scroll_forward: None,
            on_scroll_backward: None,
            on_long_press: None,
            state: SemanticState::default(),
            child: Some(child.into()),
        }
    }

    /// Sets the semantic role (e.g. Button, Header, TextField).
    #[must_use]
    pub fn role(mut self, role: SemanticRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Sets the accessible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the accessible current value.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Sets the accessible extended description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a supported semantic action.
    #[must_use]
    pub fn action(mut self, action: SemanticAction) -> Self {
        if !self
            .actions
            .iter()
            .any(|existing| existing.kind() == action.kind())
        {
            self.actions.push(action);
        }
        self
    }

    /// Attaches a named accessibility action and its callback.
    #[must_use]
    pub fn custom_action(
        mut self,
        action: CustomSemanticsAction,
        callback: impl Fn() + 'static,
    ) -> Self {
        self.custom_actions.push(CustomActionBinding {
            action,
            callback: Some(Rc::new(callback)),
        });
        self
    }

    /// Describes the standard accessibility tap without adding a second action.
    #[must_use]
    pub fn on_tap_hint(self, hint: impl Into<String>) -> Self {
        self.with_standard_action_hint(SemanticActionKind::Activate, hint.into())
    }

    /// Describes the standard accessibility long press without adding a
    /// second action.
    #[must_use]
    pub fn on_long_press_hint(self, hint: impl Into<String>) -> Self {
        self.with_standard_action_hint(SemanticActionKind::LongPress, hint.into())
    }

    fn with_standard_action_hint(mut self, kind: SemanticActionKind, hint: String) -> Self {
        self.custom_actions
            .retain(|binding| binding.callback.is_some() || binding.action.action() != Some(kind));
        self.custom_actions.push(CustomActionBinding {
            action: CustomSemanticsAction::overriding_action(kind, hint),
            callback: None,
        });
        self
    }

    /// Sets the complete retained semantic state for this annotation.
    ///
    /// The state is copied into the retained [`crate::ExplicitSemantics`]
    /// descriptor when this widget is converted. This keeps semantic state
    /// renderer-independent and lets native accessibility projections expose
    /// the same values as the widget tree.
    #[must_use]
    pub fn state(mut self, state: SemanticState) -> Self {
        self.state = state;
        self
    }

    /// Blocks focus through accessibility for this node or its subtree,
    /// without removing semantics or changing actual keyboard focus.
    #[must_use]
    pub fn accessibility_focus_block_type(mut self, block: AccessibilityFocusBlockType) -> Self {
        self.state.accessibility_focus_block_type = block;
        self
    }

    /// Flutter's `hint`. Incular's node has a single description field that
    /// carries the hint, so a hint and a description set together share it.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.description = Some(hint.into());
        self
    }

    /// Flutter's `tooltip`. It is kept apart from the hint and description:
    /// the native tooltip property (`SemanticState::tooltip`) carries it.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.state.tooltip = Some(tooltip.into());
        self
    }

    /// Flutter's `increasedValue`: the value an increment would produce.
    #[must_use]
    pub fn increased_value(mut self, value: impl Into<String>) -> Self {
        self.state.increased_value = Some(value.into());
        self
    }

    /// Flutter's `decreasedValue`: the value a decrement would produce.
    #[must_use]
    pub fn decreased_value(mut self, value: impl Into<String>) -> Self {
        self.state.decreased_value = Some(value.into());
        self
    }

    /// Flutter's `sortKey`: orders this node among its siblings in
    /// traversal. See [`incular_semantics::OrdinalSortKey`].
    #[must_use]
    pub fn sort_key(mut self, key: incular_semantics::OrdinalSortKey) -> Self {
        self.state.sort_key = Some(key);
        self
    }

    /// Flutter's `container`: this annotation is always its own accessibility
    /// node, even without a label or role.
    #[must_use]
    pub fn container(mut self, container: bool) -> Self {
        self.state.container = container;
        self
    }

    /// Flutter's `explicitChildNodes`: when this node merges its descendants
    /// (see [`crate::MergeSemantics`]), its children stay separate nodes.
    #[must_use]
    pub fn explicit_child_nodes(mut self, explicit: bool) -> Self {
        self.state.explicit_child_nodes = explicit;
        self
    }

    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.state.selected = selected;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state.enabled = enabled;
        self
    }

    #[must_use]
    pub fn checked(mut self, checked: impl Into<incular_semantics::CheckedState>) -> Self {
        self.state.checked = Some(checked.into());
        self
    }

    #[must_use]
    pub fn toggled(mut self, toggled: bool) -> Self {
        self.state.toggled = Some(toggled);
        self
    }

    /// Flutter's `hidden`: in the tree but not currently visible.
    #[must_use]
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.state.hidden = hidden;
        self
    }

    /// Flutter's `liveRegion`: changes are announced politely.
    #[must_use]
    pub fn live_region(mut self, live_region: bool) -> Self {
        self.state.live_region = live_region;
        self
    }

    /// Flutter's `linkUrl`.
    #[must_use]
    pub fn link_url(mut self, url: impl Into<String>) -> Self {
        self.state.link_url = Some(url.into());
        self
    }

    /// Flutter's `identifier`, a stable id for UI automation.
    #[must_use]
    pub fn identifier(mut self, identifier: impl Into<String>) -> Self {
        self.state.identifier = Some(identifier.into());
        self
    }

    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.state.focused = focused;
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.state.read_only = read_only;
        self
    }

    #[must_use]
    pub fn obscured(mut self, obscured: bool) -> Self {
        self.state.obscured = obscured;
        self
    }

    #[must_use]
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.state.multiline = multiline;
        self
    }

    #[must_use]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.state.invalid = invalid;
        self
    }

    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.state.required = required;
        self
    }

    #[must_use]
    pub fn busy(mut self, busy: bool) -> Self {
        self.state.busy = busy;
        self
    }

    #[must_use]
    pub fn numeric_value(mut self, value: f64) -> Self {
        self.state.numeric_value = value.is_finite().then_some(value);
        self
    }

    #[must_use]
    pub fn numeric_range(mut self, minimum: f64, maximum: f64) -> Self {
        self.state.numeric_min = minimum.is_finite().then_some(minimum);
        self.state.numeric_max = maximum.is_finite().then_some(maximum);
        self
    }

    #[must_use]
    pub fn numeric_step(mut self, step: f64) -> Self {
        self.state.numeric_step = step.is_finite().then_some(step.max(0.0));
        self
    }

    #[must_use]
    pub fn heading_level(mut self, level: impl TryInto<u8>) -> Self {
        let level = level
            .try_into()
            .ok()
            .filter(|level| (1..=6).contains(level));
        debug_assert!(level.is_some(), "heading level must be between 1 and 6");
        if let Some(level) = level {
            self.state.heading_level = Some(level);
            self.role = Some(SemanticRole::Heading);
        }
        self
    }

    #[must_use]
    pub fn header(mut self, is_header: bool) -> Self {
        if is_header {
            self.role = Some(SemanticRole::Heading);
        }
        self
    }

    #[must_use]
    pub fn button(mut self, is_button: bool) -> Self {
        if is_button {
            self.role = Some(SemanticRole::Button);
        }
        self
    }

    #[must_use]
    pub fn slider(mut self, is_slider: bool) -> Self {
        if is_slider {
            self.role = Some(SemanticRole::Slider);
        }
        self
    }

    #[must_use]
    pub fn link(mut self, is_link: bool) -> Self {
        if is_link {
            self.role = Some(SemanticRole::Link);
        }
        self
    }

    #[must_use]
    pub fn image(mut self, is_image: bool) -> Self {
        if is_image {
            self.role = Some(SemanticRole::Image);
        }
        self
    }

    #[must_use]
    pub fn progress_bar(mut self, is_progress_bar: bool) -> Self {
        if is_progress_bar {
            self.role = Some(SemanticRole::ProgressBar);
        }
        self
    }

    #[must_use]
    pub fn group(mut self, is_group: bool) -> Self {
        if is_group {
            self.role = Some(SemanticRole::Group);
        }
        self
    }

    /// Adds an activation callback. The action does not change the node's
    /// role: the application declares the role (for example
    /// [`Semantics::button`]), as Flutter's `onTap` does.
    #[must_use]
    pub fn on_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self.action(SemanticAction::Activate)
    }

    /// Adds a long-press callback (Flutter's `onLongPress`).
    #[must_use]
    pub fn on_long_press(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_long_press = Some(Rc::new(callback));
        self.action(SemanticAction::LongPress)
    }

    #[must_use]
    pub fn on_increase(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_increase = Some(Rc::new(callback));
        self.action(SemanticAction::Increment)
    }

    #[must_use]
    pub fn on_decrease(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_decrease = Some(Rc::new(callback));
        self.action(SemanticAction::Decrement)
    }

    #[must_use]
    pub fn on_scroll_left(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_backward = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollBackward)
    }

    #[must_use]
    pub fn on_scroll_right(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_forward = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollForward)
    }

    #[must_use]
    pub fn on_scroll_up(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_backward = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollBackward)
    }

    #[must_use]
    pub fn on_scroll_down(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_forward = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollForward)
    }

    /// Returns the retained semantic descriptor that will be attached during
    /// conversion. This is an observation hook for framework tooling; normal
    /// applications should use the semantic widget itself.
    ///
    /// A wrapper without a role still describes its metadata (label, value,
    /// state, actions): it becomes a generic container node, as Flutter's
    /// labelled `Semantics` does. Nothing is returned for a wrapper that
    /// carries no semantic information at all.
    #[doc(hidden)]
    #[must_use]
    pub fn explicit(&self) -> Option<crate::ExplicitSemantics> {
        let has_metadata = self.role.is_some()
            || self.label.is_some()
            || self.value.is_some()
            || self.description.is_some()
            || !self.actions.is_empty()
            || self.state != SemanticState::default();
        if !has_metadata {
            return None;
        }
        let role = self.role.unwrap_or(SemanticRole::GenericContainer);
        let mut explicit = crate::ExplicitSemantics::new(role)
            .state(self.state.clone())
            .actions(self.actions.iter().map(SemanticAction::kind));
        if let Some(label) = &self.label {
            explicit = explicit.label(label.clone());
        }
        if let Some(value) = &self.value {
            explicit = explicit.value(value.clone());
        }
        if let Some(description) = &self.description {
            explicit = explicit.description(description.clone());
        }
        Some(explicit)
    }
}

impl From<Semantics> for Widget {
    fn from(value: Semantics) -> Self {
        let explicit = value.explicit();
        let on_tap = value.on_tap.clone();
        let on_long_press = value.on_long_press.clone();
        let on_increase = value.on_increase.clone();
        let on_decrease = value.on_decrease.clone();
        let on_scroll_forward = value.on_scroll_forward.clone();
        let on_scroll_backward = value.on_scroll_backward.clone();
        let mut widget: Widget = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        if let Some(explicit) = explicit {
            widget = widget.semantics(explicit);
        }
        if let Some(callback) = on_tap {
            widget = widget
                .with_semantic_callback(incular_semantics::SemanticActionKind::Activate, callback);
        }
        if let Some(callback) = on_long_press {
            widget = widget
                .with_semantic_callback(incular_semantics::SemanticActionKind::LongPress, callback);
        }
        if let Some(callback) = on_increase {
            widget = widget
                .with_semantic_callback(incular_semantics::SemanticActionKind::Increment, callback);
        }
        if let Some(callback) = on_decrease {
            widget = widget
                .with_semantic_callback(incular_semantics::SemanticActionKind::Decrement, callback);
        }
        if let Some(callback) = on_scroll_forward {
            widget = widget.with_semantic_callback(
                incular_semantics::SemanticActionKind::ScrollForward,
                callback,
            );
        }
        if let Some(callback) = on_scroll_backward {
            widget = widget.with_semantic_callback(
                incular_semantics::SemanticActionKind::ScrollBackward,
                callback,
            );
        }
        if !value.custom_actions.is_empty() {
            widget = widget.with_custom_semantic_actions(value.custom_actions);
        }
        widget
    }
}

/// Merges descendant accessibility nodes into a single semantic element.
#[derive(Clone, Default, TypedBuilder)]
pub struct MergeSemantics {
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl MergeSemantics {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: Some(child.into()),
        }
    }
}

impl From<MergeSemantics> for Widget {
    fn from(value: MergeSemantics) -> Self {
        let widget: Widget = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        widget.merge_semantics()
    }
}

/// Excludes a subtree from the accessibility tree.
#[derive(Clone, TypedBuilder, SmartDefault)]
pub struct ExcludeSemantics {
    #[builder(default = true)]
    #[default(true)]
    excluding: bool,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl ExcludeSemantics {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            excluding: true,
            child: Some(child.into()),
        }
    }

    #[must_use]
    pub fn excluding(mut self, excluding: bool) -> Self {
        self.excluding = excluding;
        self
    }
}

impl From<ExcludeSemantics> for Widget {
    fn from(value: ExcludeSemantics) -> Self {
        let widget: Widget = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        if value.excluding {
            widget.exclude_semantics()
        } else {
            widget
        }
    }
}

/// Blocks preceding semantic siblings at the same stacking level (e.g. for modal dialogs).
#[derive(Clone, TypedBuilder, SmartDefault)]
pub struct BlockSemantics {
    #[builder(default = true)]
    #[default(true)]
    blocking: bool,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl BlockSemantics {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            blocking: true,
            child: Some(child.into()),
        }
    }

    #[must_use]
    pub fn blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }
}

impl From<BlockSemantics> for Widget {
    fn from(value: BlockSemantics) -> Self {
        let widget: Widget = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        if value.blocking {
            widget.block_semantics()
        } else {
            widget
        }
    }
}
