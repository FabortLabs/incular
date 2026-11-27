use std::{collections::HashSet, fmt};

use incular_core::Rect;

use crate::{SemanticNode, SemanticNodeId, SemanticsTree};

/// A semantic node with an empty rectangle, and its publishing parent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InvisibleSemanticsNode {
    pub node: SemanticNodeId,
    pub parent: Option<SemanticNodeId>,
    pub bounds: Rect,
}

/// A proposed semantics update contains independently invisible nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct InvisibleSemanticsNodes {
    nodes: Vec<InvisibleSemanticsNode>,
}

impl InvisibleSemanticsNodes {
    #[must_use]
    pub fn nodes(&self) -> &[InvisibleSemanticsNode] {
        &self.nodes
    }
}

impl fmt::Display for InvisibleSemanticsNodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Invisible semantic nodes cannot be published.")?;
        for issue in &self.nodes {
            write!(f, "\n  {:?} at {:?}", issue.node, issue.bounds)?;
            if let Some(parent) = issue.parent {
                write!(f, " is a child of {parent:?}")?;
            } else {
                f.write_str(" is a root with children")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for InvisibleSemanticsNodes {}

impl SemanticsTree {
    /// Checks an already-merged graph before publication. A root may be
    /// empty when it is alone, and visibility flags on nonempty cached
    /// viewport nodes do not make the rectangle invalid.
    ///
    /// # Errors
    /// Returns each independently invisible node in traversal order. An
    /// invisible node's descendants are not checked a second time.
    pub fn validate_visibility(&self) -> Result<(), InvisibleSemanticsNodes> {
        self.validate_visibility_with_merge_boundaries(|_| false)
    }

    /// Checks a graph before it collapses merge boundaries. Descendants of a
    /// visible merge boundary contribute to it rather than being published
    /// as independent nodes.
    ///
    /// # Errors
    /// Returns the invisible nodes that would still be published independently.
    pub fn validate_visibility_with_merge_boundaries(
        &self,
        is_merge_boundary: impl Fn(&SemanticNode) -> bool,
    ) -> Result<(), InvisibleSemanticsNodes> {
        let mut issues = Vec::new();
        let mut seen = HashSet::new();
        let mut pending: Vec<_> = self.roots().iter().rev().map(|&id| (id, None)).collect();
        while let Some((id, parent)) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(node) = self.node(id) else {
                continue;
            };
            let empty = node.bounds.size.width <= 0. || node.bounds.size.height <= 0.;
            if empty {
                if parent.is_some()
                    || node
                        .children
                        .iter()
                        .any(|&child| self.node(child).is_some())
                {
                    issues.push(InvisibleSemanticsNode {
                        node: id,
                        parent,
                        bounds: node.bounds,
                    });
                }
            } else if !is_merge_boundary(node) {
                pending.extend(node.children.iter().rev().map(|&child| (child, Some(id))));
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(InvisibleSemanticsNodes { nodes: issues })
        }
    }

    /// Locates the independently published ancestor that absorbs this
    /// node in a pre-merge graph. A merge flag on the target itself does not
    /// make it merged into its parent.
    #[must_use]
    pub fn merge_owner(
        &self,
        target: SemanticNodeId,
        is_merge_boundary: impl Fn(&SemanticNode) -> bool,
    ) -> Option<SemanticNodeId> {
        let mut seen = HashSet::new();
        let mut pending: Vec<_> = self.roots().iter().rev().map(|&id| (id, None)).collect();
        while let Some((id, owner)) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(node) = self.node(id) else {
                continue;
            };
            if id == target {
                return owner;
            }
            let owner = owner.or_else(|| is_merge_boundary(node).then_some(id));
            pending.extend(node.children.iter().rev().map(|&child| (child, owner)));
        }
        None
    }
}
