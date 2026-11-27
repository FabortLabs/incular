//! Accessible semantic annotation widgets.

use incular_semantics::{
    AccessibilityFocusBlockType, AttributedString, CustomSemanticsAction, SemanticAction,
    SemanticActionKind, SemanticAnnotationFlag, SemanticRole, SemanticState,
    SemanticTextAttributes, SemanticsInputType, TextSelection,
};
use smart_default::SmartDefault;
use std::{rc::Rc, sync::Arc};
use typed_builder::TypedBuilder;

use crate::{
    Widget,
    tree::{ChildLayoutMode, WidgetKind},
};

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

pub(crate) type BoundSemanticCallback = Rc<dyn Fn(SemanticAction) + 'static>;

#[derive(Clone)]
pub(crate) struct BoundSemanticAction {
    pub(crate) kind: SemanticActionKind,
    pub(crate) callback: BoundSemanticCallback,
}

impl std::fmt::Debug for BoundSemanticAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoundSemanticAction")
            .field(&self.kind)
            .finish()
    }
}

impl PartialEq for BoundSemanticAction {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && Rc::ptr_eq(&self.callback, &other.callback)
    }
}

/// An accessible semantic annotation on a widget subtree.
#[derive(Clone, Default, TypedBuilder)]
pub struct Semantics {
    #[builder(default, setter(strip_option))]
    role: Option<SemanticRole>,
    #[builder(default, setter(strip_option, into))]
    label: Option<AttributedString>,
    #[builder(default, setter(strip_option, into))]
    value: Option<AttributedString>,
    #[builder(default, setter(strip_option, into))]
    description: Option<AttributedString>,
    #[builder(default)]
    actions: Vec<SemanticAction>,
    #[builder(default, setter(skip))]
    custom_actions: Vec<CustomActionBinding>,
    #[builder(default, setter(skip))]
    bound_callbacks: Vec<BoundSemanticAction>,
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
    on_scroll_left: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_scroll_right: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_scroll_up: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_scroll_down: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_long_press: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    on_dismiss: Option<Rc<dyn Fn() + 'static>>,
    #[builder(default)]
    state: SemanticState,
    #[builder(default)]
    block_user_actions: bool,
    #[builder(default, setter(skip))]
    // The outer option keeps the explicit annotation when a caller clears
    // its tag, so handling that update does not remount the subtree.
    tag_for_children: Option<Option<incular_semantics::SemanticsTag>>,
    #[builder(default, setter(strip_option))]
    locale_for_subtree: Option<incular_config::Locale>,
    #[builder(default, setter(strip_option, into))]
    child: Option<Widget>,
}

impl Semantics {
    /// Sets the pronunciation locale for this annotation and its descendants.
    #[must_use]
    pub fn locale_for_subtree(mut self, locale: incular_config::Locale) -> Self {
        self.locale_for_subtree = Some(locale);
        self
    }

    /// Suppresses user actions in the accessibility tree without disabling
    /// pointer input, keyboard focus, or labels beneath this annotation.
    #[must_use]
    pub fn block_user_actions(mut self, block: bool) -> Self {
        self.block_user_actions = block;
        self
    }

    pub(crate) const fn blocks_user_actions(&self) -> bool {
        self.block_user_actions
    }

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
            bound_callbacks: Vec::new(),
            on_tap: None,
            on_increase: None,
            on_decrease: None,
            on_scroll_forward: None,
            on_scroll_backward: None,
            on_scroll_left: None,
            on_scroll_right: None,
            on_scroll_up: None,
            on_scroll_down: None,
            on_long_press: None,
            on_dismiss: None,
            state: SemanticState::default(),
            block_user_actions: false,
            tag_for_children: None,
            locale_for_subtree: None,
            child: Some(child.into()),
        }
    }

    /// Sets the semantic role (e.g. Button, Header, TextField).
    #[must_use]
    pub fn role(mut self, role: SemanticRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Declares the maximum number of Unicode scalar values an editor
    /// accepts. This is accessibility metadata, not an input limiter.
    #[must_use]
    pub fn max_value_length(mut self, length: usize) -> Self {
        self.state.max_value_length = Some(length);
        self
    }

    /// Declares the current number of Unicode scalar values.
    #[must_use]
    pub fn current_value_length(mut self, length: usize) -> Self {
        self.state.current_value_length = Some(length);
        self
    }

    /// Declares the semantic kind of input without changing the keyboard.
    #[must_use]
    pub fn input_type(mut self, input_type: SemanticsInputType) -> Self {
        self.state.input_type = input_type;
        self
    }

    /// Declares which identified nodes this widget controls. Repeated
    /// identifiers are ignored without changing discovery order.
    #[must_use]
    pub fn controls_nodes(
        mut self,
        identifiers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let mut nodes = Vec::new();
        for identifier in identifiers {
            let identifier = identifier.into();
            if !nodes.contains(&identifier) {
                nodes.push(identifier);
            }
        }
        self.state.controls_nodes = Some(nodes.into());
        self
    }

    /// Sets the accessible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(AttributedString::new(label));
        self
    }

    /// Sets the reading direction of the accessible label independently of
    /// its visual layout or the surrounding Directionality.
    #[must_use]
    pub fn text_direction(mut self, direction: incular_config::TextDirection) -> Self {
        self.state.text_direction = Some(direction);
        self
    }

    /// Sets the accessible current value.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(AttributedString::new(value));
        self
    }

    /// Sets the accessible extended description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(AttributedString::new(description));
        self
    }

    /// Sets the accessible label with pronunciation attributes.
    #[must_use]
    pub fn attributed_label(mut self, label: AttributedString) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the accessible value with pronunciation attributes.
    #[must_use]
    pub fn attributed_value(mut self, value: AttributedString) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets the accessible description with pronunciation attributes.
    #[must_use]
    pub fn attributed_hint(mut self, hint: AttributedString) -> Self {
        self.description = Some(hint);
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

    /// Reads the heading and validity this annotation contributes.
    #[must_use]
    pub fn annotation_metadata(&self) -> incular_semantics::SemanticAnnotationMetadata {
        incular_semantics::SemanticAnnotationMetadata {
            heading_level: self.state.heading_level,
            validation_result: self.state.effective_validation_result(),
        }
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
    pub fn hint(self, hint: impl Into<String>) -> Self {
        self.description(hint)
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
        self.state.custom_sort_key = None;
        self
    }

    /// Orders this node with adjacent keys of the same Rust type and name.
    #[must_use]
    pub fn custom_sort_key(mut self, key: incular_semantics::CustomSemanticsSortKey) -> Self {
        self.state.sort_key = None;
        self.state.custom_sort_key = Some(key);
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
        self.state.has_selected_state = true;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state.set_enabled_state(Some(enabled));
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

    /// Adds framework metadata to this semantics annotation without
    /// changing the accessibility label or sending it to the platform.
    #[must_use]
    pub fn tag(mut self, tag: incular_semantics::SemanticsTag) -> Self {
        self.state.add_tag(tag);
        self
    }

    pub(crate) fn as_custom_painter_region(mut self) -> Self {
        // Flutter assembles the region directly as a semantic node and
        // gives this tag to the synthetic node, not to a widget child.
        if let Some(Some(tag)) = self.tag_for_children.take() {
            self.state.add_tag(tag);
        }
        self
    }

    /// Flutter's `tagForChildren`: assigns a framework tag to nodes
    /// passing through this annotation, stopping at a child boundary.
    #[must_use]
    pub fn tag_for_children(
        mut self,
        tag: impl Into<Option<incular_semantics::SemanticsTag>>,
    ) -> Self {
        self.tag_for_children = Some(tag.into());
        self
    }

    /// Flutter's platform hit-test override for this semantic node.
    #[must_use]
    pub fn hit_test_behavior(mut self, behavior: incular_semantics::SemanticsHitTestBehavior) -> Self {
        self.state.hit_test_behavior = behavior;
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

    /// Declares the accessibility representation of input focus without
    /// requesting keyboard focus. Both true and false declare focusability.
    #[must_use]
    pub fn focused(mut self, focused: bool) -> Self {
        self.state.declared_focused = Some(focused);
        self
    }

    /// Sets the legacy focusable flag without creating a keyboard target.
    /// A declared `focused` value takes precedence if also supplied.
    #[must_use]
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.state.declared_focusable = Some(focusable);
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

    /// Sets Flutter's explicit result without conflating a valid field
    /// with one whose validity has not been established.
    #[must_use]
    pub fn validation_result(
        mut self,
        result: incular_semantics::SemanticValidationResult,
    ) -> Self {
        self.state.validation_result = result;
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
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::Header, is_header);
        if is_header {
            self.role = Some(SemanticRole::Heading);
        }
        self
    }

    #[must_use]
    pub fn button(mut self, is_button: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::Button, is_button);
        if is_button {
            self.role = Some(SemanticRole::Button);
        }
        self
    }

    #[must_use]
    pub fn slider(mut self, is_slider: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::Slider, is_slider);
        if is_slider {
            self.role = Some(SemanticRole::Slider);
        }
        self
    }

    #[must_use]
    pub fn link(mut self, is_link: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::Link, is_link);
        if is_link {
            self.role = Some(SemanticRole::Link);
        }
        self
    }

    #[must_use]
    pub fn image(mut self, is_image: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::Image, is_image);
        if is_image {
            self.role = Some(SemanticRole::Image);
        }
        self
    }

    /// Declares an editable-text role flag without creating an editor.
    #[must_use]
    pub fn text_field(mut self, is_text_field: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::TextField, is_text_field);
        if is_text_field {
            self.role = Some(SemanticRole::TextField);
        }
        self
    }

    #[must_use]
    pub fn keyboard_key(mut self, is_key: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::KeyboardKey, is_key);
        self
    }

    #[must_use]
    pub fn in_mutually_exclusive_group(mut self, exclusive: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::MutuallyExclusiveGroup, exclusive);
        self
    }

    #[must_use]
    pub fn scopes_route(mut self, scopes: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::ScopesRoute, scopes);
        self
    }

    #[must_use]
    pub fn names_route(mut self, names: bool) -> Self {
        self.state
            .annotation_flags
            .set(SemanticAnnotationFlag::NamesRoute, names);
        self
    }

    #[must_use]
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.state.expanded = Some(expanded);
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

    /// Receives the replacement text from an accessibility SetText action.
    /// This handler does not require or mutate an editable child.
    #[must_use]
    pub fn on_set_text(self, callback: impl Fn(String) + 'static) -> Self {
        self.with_bound_callback(SemanticAction::SetText(String::new()), move |action| {
            if let SemanticAction::SetText(text) = action {
                callback(text);
            }
        })
    }

    /// Receives the requested selection in the original base/extent order.
    #[must_use]
    pub fn on_set_selection(self, callback: impl Fn(TextSelection) + 'static) -> Self {
        self.with_bound_callback(
            SemanticAction::SetSelection { base: 0, extent: 0 },
            move |action| {
                if let SemanticAction::SetSelection { base, extent } = action {
                    callback(TextSelection { base, extent });
                }
            },
        )
    }

    /// Registers a callback for an accessibility copy request.
    #[must_use]
    pub fn on_copy(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::Copy, move |_| callback())
    }

    /// Registers a callback for an accessibility cut request.
    #[must_use]
    pub fn on_cut(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::Cut, move |_| callback())
    }

    /// Registers a callback for an accessibility paste request.
    #[must_use]
    pub fn on_paste(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::Paste, move |_| callback())
    }

    /// Registers a callback for an accessibility expand request.
    #[must_use]
    pub fn on_expand(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::Expand, move |_| callback())
    }

    /// Registers a callback for an accessibility collapse request.
    #[must_use]
    pub fn on_collapse(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::Collapse, move |_| callback())
    }

    /// Receives a request to move the accessibility cursor forward by
    /// a character, with the original selection-extension flag.
    #[must_use]
    pub fn on_move_cursor_forward_by_character(
        self,
        callback: impl Fn(bool) + 'static,
    ) -> Self {
        self.with_bound_callback(
            SemanticAction::MoveCursorForwardByCharacter { extend_selection: false },
            move |action| {
                if let SemanticAction::MoveCursorForwardByCharacter { extend_selection } = action {
                    callback(extend_selection);
                }
            },
        )
    }

    /// Receives a request to move the accessibility cursor backward by
    /// a character, with the original selection-extension flag.
    #[must_use]
    pub fn on_move_cursor_backward_by_character(
        self,
        callback: impl Fn(bool) + 'static,
    ) -> Self {
        self.with_bound_callback(
            SemanticAction::MoveCursorBackwardByCharacter { extend_selection: false },
            move |action| {
                if let SemanticAction::MoveCursorBackwardByCharacter { extend_selection } = action {
                    callback(extend_selection);
                }
            },
        )
    }

    /// Receives a request to move the accessibility cursor forward by
    /// a word with the original selection-extension flag.
    #[must_use]
    pub fn on_move_cursor_forward_by_word(self, callback: impl Fn(bool) + 'static) -> Self {
        self.with_bound_callback(
            SemanticAction::MoveCursorForwardByWord { extend_selection: false },
            move |action| {
                if let SemanticAction::MoveCursorForwardByWord { extend_selection } = action {
                    callback(extend_selection);
                }
            },
        )
    }

    /// Receives a request to move the accessibility cursor backward by
    /// a word with the original selection-extension flag.
    #[must_use]
    pub fn on_move_cursor_backward_by_word(self, callback: impl Fn(bool) + 'static) -> Self {
        self.with_bound_callback(
            SemanticAction::MoveCursorBackwardByWord { extend_selection: false },
            move |action| {
                if let SemanticAction::MoveCursorBackwardByWord { extend_selection } = action {
                    callback(extend_selection);
                }
            },
        )
    }

    /// Registers a request to set input focus. The handler decides how to
    /// do that; this annotation does not make its child keyboard-focusable.
    #[must_use]
    pub fn on_focus(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::Focus, move |_| callback())
    }

    /// Notifies the handler when assistive technology focuses this node.
    #[must_use]
    pub fn on_did_gain_accessibility_focus(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::DidGainAccessibilityFocus, move |_| callback())
    }

    /// Notifies the handler when assistive technology leaves this node.
    #[must_use]
    pub fn on_did_lose_accessibility_focus(self, callback: impl Fn() + 'static) -> Self {
        self.with_bound_callback(SemanticAction::DidLoseAccessibilityFocus, move |_| callback())
    }

    fn with_bound_callback(
        mut self,
        action: SemanticAction,
        callback: impl Fn(SemanticAction) + 'static,
    ) -> Self {
        let kind = action.kind();
        self.bound_callbacks
            .retain(|binding| binding.kind != kind);
        self.bound_callbacks.push(BoundSemanticAction {
            kind,
            callback: Rc::new(callback),
        });
        self.action(action)
    }

    /// Flutter's `Semantics.onDismiss`: advertises a dismiss action and
    /// invokes this callback when the framework dispatches it.
    #[must_use]
    pub fn on_dismiss(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(callback));
        self.action(SemanticAction::Dismiss)
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
        self.on_scroll_left = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollLeft)
    }

    #[must_use]
    pub fn on_scroll_right(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_right = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollRight)
    }

    #[must_use]
    pub fn on_scroll_up(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_up = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollUp)
    }

    #[must_use]
    pub fn on_scroll_down(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_down = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollDown)
    }

    /// Registers a logical forward action independently of the four arrows.
    #[must_use]
    pub fn on_scroll_forward(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_forward = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollForward)
    }

    /// Registers a logical backward action independently of the four arrows.
    #[must_use]
    pub fn on_scroll_backward(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_scroll_backward = Some(Rc::new(callback));
        self.action(SemanticAction::ScrollBackward)
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
        debug_assert!(
            self.state.annotation_flags.get(SemanticAnnotationFlag::ScopesRoute) != Some(true)
                || self.state.explicit_child_nodes,
            "explicit_child_nodes must be true if scopes_route is true",
        );
        let has_metadata = self.role.is_some()
            || self.locale_for_subtree.is_some()
            || self.label.is_some()
            || self.value.is_some()
            || self.description.is_some()
            || self.on_scroll_left.is_some()
            || self.on_scroll_right.is_some()
            || self.on_scroll_up.is_some()
            || self.on_scroll_down.is_some()
            || !self.actions.is_empty()
            || self.state != SemanticState::default();
        if !has_metadata {
            return None;
        }
        let role = self.role.unwrap_or(SemanticRole::GenericContainer);
        let attributes = |item: &Option<AttributedString>| {
            item.as_ref()
                .map_or_else(Vec::new, |item| item.attributes().to_vec())
        };
        let attributes = SemanticTextAttributes {
            label: attributes(&self.label),
            value: attributes(&self.value),
            hint: attributes(&self.description),
        };
        let mut state = self.state.clone();
        state.text_attributes = (!attributes.is_empty()).then(|| Arc::new(attributes));
        let mut explicit = crate::ExplicitSemantics::new(role)
            .state(state)
            .actions(self.actions.iter().map(SemanticAction::kind));
        if let Some(label) = &self.label {
            explicit = explicit.label(label.as_str());
        }
        if let Some(value) = &self.value {
            explicit = explicit.value(value.as_str());
        }
        if let Some(description) = &self.description {
            explicit = explicit.description(description.as_str());
        }
        Some(explicit)
    }
}

impl From<Semantics> for Widget {
    fn from(value: Semantics) -> Self {
        let explicit = value.explicit();
        let on_tap = value.on_tap.clone();
        let on_long_press = value.on_long_press.clone();
        let on_dismiss = value.on_dismiss.clone();
        let on_increase = value.on_increase.clone();
        let on_decrease = value.on_decrease.clone();
        let on_scroll_forward = value.on_scroll_forward.clone();
        let on_scroll_backward = value.on_scroll_backward.clone();
        let mut widget: Widget = value
            .child
            .unwrap_or_else(|| crate::SizedBox::shrink().into());
        let tags_children = value.tag_for_children.is_some();
        if tags_children || (explicit.is_some() && widget.has_explicit_semantics()) {
            // Keep the tagging annotation outside its child, including
            // children that contribute implicit semantics such as Text.
            widget = Widget::from_kind(WidgetKind::ProxyLayout {
                mode: ChildLayoutMode::SemanticAnnotation,
                child: Some(widget),
            });
        }
        if value.block_user_actions {
            widget = widget.with_blocked_semantic_user_actions();
        }
        if let Some(explicit) = explicit {
            widget = widget.semantics(explicit);
        }
        if let Some(Some(tag)) = value.tag_for_children {
            widget = widget.with_semantic_child_tag(tag);
        }
        if let Some(locale) = value.locale_for_subtree {
            widget = widget.with_semantic_locale(Arc::from(locale.to_string()));
        }
        if let Some(callback) = on_tap {
            widget = widget
                .with_semantic_callback(incular_semantics::SemanticActionKind::Activate, callback);
        }
        if let Some(callback) = on_long_press {
            widget = widget
                .with_semantic_callback(incular_semantics::SemanticActionKind::LongPress, callback);
        }
        if let Some(callback) = on_dismiss {
            widget = widget
                .with_semantic_callback(incular_semantics::SemanticActionKind::Dismiss, callback);
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
        for (action, callback) in [
            (
                incular_semantics::SemanticActionKind::ScrollLeft,
                value.on_scroll_left,
            ),
            (
                incular_semantics::SemanticActionKind::ScrollRight,
                value.on_scroll_right,
            ),
            (
                incular_semantics::SemanticActionKind::ScrollUp,
                value.on_scroll_up,
            ),
            (
                incular_semantics::SemanticActionKind::ScrollDown,
                value.on_scroll_down,
            ),
        ] {
            if let Some(callback) = callback {
                widget = widget.with_semantic_callback(action, callback);
            }
        }
        if !value.bound_callbacks.is_empty() {
            widget = widget.with_semantic_bound_callbacks(value.bound_callbacks);
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
        let widget = Widget::from_kind(WidgetKind::ProxyLayout {
            mode: ChildLayoutMode::SemanticBlocker,
            child: Some(
                value
                    .child
                    .unwrap_or_else(|| crate::SizedBox::shrink().into()),
            ),
        });
        if value.blocking {
            widget.block_semantics()
        } else {
            widget
        }
    }
}
