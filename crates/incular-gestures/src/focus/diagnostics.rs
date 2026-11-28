use std::hash::{DefaultHasher, Hash, Hasher};

use incular_core::{DiagnosticsNode, DiagnosticsProperty};

use super::{FocusNode, FocusScopeNode, TraversalEdgeBehavior};

impl FocusNode {
    /// Returns an identity label and the node's current role in the focus path.
    #[must_use]
    pub fn to_string_short(&self) -> String {
        let node_type = if self.scope_owner().is_some() {
            "FocusScopeNode"
        } else {
            "FocusNode"
        };
        let mut hash = DefaultHasher::new();
        self.hash(&mut hash);
        let mut result = format!("{node_type}#{:05x}", hash.finish() & 0xfffff);
        let label = self.debug_label().filter(|label| !label.is_empty());
        let role = if self.has_primary_focus() {
            Some("[PRIMARY FOCUS]")
        } else if self.has_focus() {
            Some("[IN FOCUS PATH]")
        } else {
            None
        };
        if label.is_some() || role.is_some() {
            result.push('(');
            if let Some(label) = &label {
                result.push_str(label);
            }
            if let Some(role) = role {
                if label.is_some() {
                    result.push(' ');
                }
                result.push_str(role);
            }
            result.push(')');
        }
        result
    }

    /// Snapshots the node's ordered debug properties. A host may supply the
    /// name of the attached widget or layout context; use `None` for a node
    /// without an attached context.
    #[must_use]
    pub fn diagnostic_properties(&self, context: Option<&str>) -> Vec<DiagnosticsNode> {
        let primary = self.has_primary_focus();
        let mut properties = vec![DiagnosticsNode::property(
            &DiagnosticsProperty::new("context", context.map(str::to_owned)).default_value(None),
        )];
        for (name, value, default, if_true, if_false) in [
            (
                "descendantsAreFocusable",
                self.descendants_are_focusable(),
                true,
                None,
                Some("DESCENDANTS UNFOCUSABLE"),
            ),
            (
                "descendantsAreTraversable",
                self.descendants_are_traversable(),
                true,
                None,
                Some("DESCENDANTS UNTRAVERSABLE"),
            ),
            (
                "canRequestFocus",
                self.can_request_focus(),
                true,
                None,
                Some("NOT FOCUSABLE"),
            ),
            (
                "hasFocus",
                self.has_focus() && !primary,
                false,
                Some("IN FOCUS PATH"),
                None,
            ),
            (
                "hasPrimaryFocus",
                primary,
                false,
                Some("PRIMARY FOCUS"),
                None,
            ),
        ] {
            properties.push(DiagnosticsNode::property(
                &DiagnosticsProperty::flag(name, Some(value), if_true, if_false)
                    .default_value(Some(default)),
            ));
        }
        properties
    }
}

impl FocusScopeNode {
    /// Snapshots the scope's flags and, after it has focused a child, its
    /// live focus history in newest-first order.
    #[must_use]
    pub fn diagnostic_properties(&self, context: Option<&str>) -> Vec<DiagnosticsNode> {
        let mut properties = self.scope_focus_node().diagnostic_properties(context);
        let children: Vec<_> = self
            .state
            .hosted_history
            .borrow()
            .iter()
            .rev()
            .filter_map(|state| state.upgrade())
            .map(|state| FocusNode { state })
            .filter(|node| node.host_enclosing_scope().as_ref() == Some(self))
            .map(|node| node.to_string_short())
            .collect();
        if children.is_empty() {
            return properties;
        }
        properties.push(DiagnosticsNode::property(&DiagnosticsProperty::iterable(
            "focusedChildren",
            Some(children),
        )));
        let edge = match self.traversal_edge_behavior() {
            TraversalEdgeBehavior::ClosedLoop => "closedLoop",
            TraversalEdgeBehavior::LeaveFlutterView => "leaveFlutterView",
            TraversalEdgeBehavior::ParentScope => "parentScope",
            TraversalEdgeBehavior::Stop => "stop",
        };
        properties.push(DiagnosticsNode::property(
            &DiagnosticsProperty::new(
                "traversalEdgeBehavior",
                Some(format!("TraversalEdgeBehavior.{edge}")),
            )
            .default_value(Some("TraversalEdgeBehavior.closedLoop".into())),
        ));
        properties
    }
}
