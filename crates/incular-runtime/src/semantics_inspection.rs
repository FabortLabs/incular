//! JSON diagnostics for retained accessibility data. These reads never
//! mutate the semantic graph, force a frame, or expose password contents.

use incular_semantics::{
    CheckedState, Role, SemanticActionKind as Action, SemanticAnnotationFlag as Flag, SemanticNode,
    SemanticNodeId,
};
use serde_json::{Value, json};

use crate::Runtime;

/// A Flutter-style data map from one retained semantic node.
///
/// Retained rectangles are already in window coordinates, so no separate
/// transform is serialized. Obscured values are withheld from diagnostics.
#[must_use]
pub fn flutter_semantics_data_json(node: &SemanticNode) -> Value {
    let state = &node.state;
    let annotation = |flag, fallback| state.annotation_flags.get(flag).unwrap_or(fallback);
    let flags: Vec<_> = [
        (state.checked.is_some(), "hasCheckedState"),
        (state.checked == Some(CheckedState::Checked), "isChecked"),
        (state.has_selected_state, "hasSelectedState"),
        (state.selected, "isSelected"),
        (
            annotation(Flag::Button, node.role == Role::Button),
            "isButton",
        ),
        (
            annotation(
                Flag::TextField,
                matches!(
                    node.role,
                    Role::TextField | Role::TextArea | Role::SearchField
                ),
            ),
            "isTextField",
        ),
        (state.focused, "isFocused"),
        (state.has_enabled_state, "hasEnabledState"),
        (state.has_enabled_state && state.enabled, "isEnabled"),
        (
            annotation(Flag::MutuallyExclusiveGroup, false),
            "isInMutuallyExclusiveGroup",
        ),
        (
            annotation(Flag::Header, node.role == Role::Heading),
            "isHeader",
        ),
        (state.obscured, "isObscured"),
        (annotation(Flag::ScopesRoute, false), "scopesRoute"),
        (annotation(Flag::NamesRoute, false), "namesRoute"),
        (state.hidden, "isHidden"),
        (annotation(Flag::Image, node.role == Role::Image), "isImage"),
        (state.live_region, "isLiveRegion"),
        (state.toggled.is_some(), "hasToggledState"),
        (state.toggled == Some(true), "isToggled"),
        (state.has_implicit_scrolling, "hasImplicitScrolling"),
        (state.multiline, "isMultiline"),
        (state.read_only, "isReadOnly"),
        (state.focusable, "isFocusable"),
        (annotation(Flag::Link, node.role == Role::Link), "isLink"),
        (
            annotation(Flag::Slider, node.role == Role::Slider),
            "isSlider",
        ),
        (annotation(Flag::KeyboardKey, false), "isKeyboardKey"),
        (
            state.checked == Some(CheckedState::Indeterminate),
            "isCheckStateMixed",
        ),
        (state.expanded.is_some(), "hasExpandedState"),
        (state.expanded == Some(true), "isExpanded"),
        (state.required, "hasRequiredState"),
        (state.required, "isRequired"),
    ]
    .into_iter()
    .filter_map(|(set, name)| set.then_some(name))
    .collect();
    let actions: Vec<_> = [
        (Action::Activate, "tap"),
        (Action::LongPress, "longPress"),
        (Action::ScrollLeft, "scrollLeft"),
        (Action::ScrollRight, "scrollRight"),
        (Action::ScrollUp, "scrollUp"),
        (Action::ScrollDown, "scrollDown"),
        (Action::ScrollToOffset, "scrollToOffset"),
        (Action::Increment, "increase"),
        (Action::Decrement, "decrease"),
        (Action::ShowOnScreen, "showOnScreen"),
        (
            Action::MoveCursorForwardByCharacter,
            "moveCursorForwardByCharacter",
        ),
        (
            Action::MoveCursorBackwardByCharacter,
            "moveCursorBackwardByCharacter",
        ),
        (Action::SetSelection, "setSelection"),
        (Action::Copy, "copy"),
        (Action::Cut, "cut"),
        (Action::Paste, "paste"),
        (
            Action::DidGainAccessibilityFocus,
            "didGainAccessibilityFocus",
        ),
        (
            Action::DidLoseAccessibilityFocus,
            "didLoseAccessibilityFocus",
        ),
        (Action::Custom, "customAction"),
        (Action::Dismiss, "dismiss"),
        (Action::MoveCursorForwardByWord, "moveCursorForwardByWord"),
        (Action::MoveCursorBackwardByWord, "moveCursorBackwardByWord"),
        (Action::SetText, "setText"),
        (Action::Focus, "focus"),
        (Action::Expand, "expand"),
        (Action::Collapse, "collapse"),
    ]
    .into_iter()
    .filter_map(|(action, name)| node.actions.contains(&action).then_some(name))
    .collect();
    let value = if state.obscured {
        ""
    } else {
        node.value.as_deref().unwrap_or_default()
    };
    json!({
        "label": node.label.as_deref().unwrap_or_default(),
        "value": value,
        "hint": node.description.as_deref().unwrap_or_default(),
        "tooltip": state.tooltip.as_deref().unwrap_or_default(),
        "increasedValue": state.increased_value.as_deref().unwrap_or_default(),
        "decreasedValue": state.decreased_value.as_deref().unwrap_or_default(),
        "flags": flags,
        "actions": actions,
        "rect": {
            "left": node.bounds.origin.x,
            "top": node.bounds.origin.y,
            "width": node.bounds.size.width,
            "height": node.bounds.size.height,
        },
    })
}

fn semantic_key(id: SemanticNodeId) -> u64 {
    (u64::from(id.0.generation()) << 32) | u64::from(id.0.index())
}

/// Adds retained identity and the two independent child orders to the data map.
/// Like Flutter's JSON field, the hit-test array stores inverse hit order,
/// with the last child being the first one visited by point dispatch.
#[must_use]
pub fn flutter_semantics_node_json(
    node: &SemanticNode,
    inverse_hit_order: &[SemanticNodeId],
) -> Value {
    let mut json = flutter_semantics_data_json(node);
    json["id"] = json!(semantic_key(node.id));
    json["childrenInTraversalOrder"] = json!(
        node.children
            .iter()
            .copied()
            .map(semantic_key)
            .collect::<Vec<_>>()
    );
    json["childrenInHitTestOrder"] = json!(
        inverse_hit_order
            .iter()
            .copied()
            .map(semantic_key)
            .collect::<Vec<_>>()
    );
    json
}

impl Runtime {
    /// Reads one live semantic node and both its traversal and paint orders.
    #[must_use]
    pub fn flutter_semantics_node_json(&self, id: SemanticNodeId) -> Option<Value> {
        let node = self.tree().semantics().node(id)?;
        let inverse_hit_order = self.tree().semantic_inverse_hit_test_children(id)?;
        Some(flutter_semantics_node_json(node, &inverse_hit_order))
    }
}
