//! SafeArea and the per-edge scopes it installs for descendants.

use incular_config::{EdgeInsets, MediaQueryEdges};
use typed_builder::TypedBuilder;

use crate::{MediaQuery, Widget};

/// Insets child from the display edges to avoid platform UI, notches, and status bars.
///
/// Consumed edges are removed from the descendant MediaQuery, so ordinary
/// nested safe areas do not apply the same inset twice.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct SafeArea {
    #[builder(setter(into))]
    pub(crate) child: Widget,
    #[builder(default = EdgeInsets::ZERO)]
    pub(crate) minimum: EdgeInsets,
    #[builder(default = true)]
    pub(crate) left: bool,
    #[builder(default = true)]
    pub(crate) top: bool,
    #[builder(default = true)]
    pub(crate) right: bool,
    #[builder(default = true)]
    pub(crate) bottom: bool,
    #[builder(default = false)]
    pub(crate) maintain_bottom_view_padding: bool,
}

impl SafeArea {
    /// Creates a SafeArea protecting all 4 edges.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            minimum: EdgeInsets::ZERO,
            left: true,
            top: true,
            right: true,
            bottom: true,
            maintain_bottom_view_padding: false,
        }
    }

    /// Toggles left-edge inset.
    #[must_use]
    pub fn left(mut self, left: bool) -> Self {
        self.left = left;
        self
    }

    /// Toggles top-edge inset.
    #[must_use]
    pub fn top(mut self, top: bool) -> Self {
        self.top = top;
        self
    }

    /// Toggles right-edge inset.
    #[must_use]
    pub fn right(mut self, right: bool) -> Self {
        self.right = right;
        self
    }

    /// Toggles bottom-edge inset.
    #[must_use]
    pub fn bottom(mut self, bottom: bool) -> Self {
        self.bottom = bottom;
        self
    }

    /// Sets the minimum fallback padding for edges.
    #[must_use]
    pub fn minimum(mut self, minimum: EdgeInsets) -> Self {
        self.minimum = minimum;
        self
    }

    /// Keeps the persistent bottom obstruction margin while transient
    /// occlusion (such as the keyboard) covers it. The maintained value
    /// comes from the ambient view padding, never the occlusion height.
    #[must_use]
    pub fn maintain_bottom_view_padding(mut self, maintain: bool) -> Self {
        self.maintain_bottom_view_padding = maintain;
        self
    }

    /// Sets active sides explicitly.
    #[must_use]
    pub fn sides(mut self, left: bool, top: bool, right: bool, bottom: bool) -> Self {
        self.left = left;
        self.top = top;
        self.right = right;
        self.bottom = bottom;
        self
    }

    /// Resolves against explicit insets for standalone use without a tree.
    ///
    /// The insets are the current safe margin. Without an ambient
    /// environment there is no persistent view padding to keep, so
    /// `maintain_bottom_view_padding` has no effect here; use
    /// [`Self::resolve_with_padding`] when both signals are available.
    #[must_use]
    pub fn resolve(self, insets: EdgeInsets) -> Widget {
        let mut policy = self.policy();
        policy.maintain_bottom_view_padding = false;
        let padding = policy.padding(insets, EdgeInsets::ZERO);
        crate::Padding::new(padding, self.child).into()
    }

    /// Resolves against both environment signals for use outside the
    /// retained tree, applying the same maintained-padding policy as
    /// retained construction: persistent view padding replaces ordinary
    /// padding on the bottom edge when maintenance is enabled.
    /// Transient occlusion never participates; keyboard avoidance belongs
    /// to the padded parent.
    #[must_use]
    pub fn resolve_with_padding(self, safe_insets: EdgeInsets, view_padding: EdgeInsets) -> Widget {
        let padding = self.policy().padding(safe_insets, view_padding);
        crate::Padding::new(padding, self.child).into()
    }

    /// Reports whether the descriptor keeps the persistent bottom margin
    /// while transient occlusion covers it.
    #[must_use]
    pub fn maintains_bottom_view_padding(&self) -> bool {
        self.maintain_bottom_view_padding
    }

    fn policy(&self) -> SafeAreaPolicy {
        SafeAreaPolicy {
            minimum: self.minimum,
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
            maintain_bottom_view_padding: self.maintain_bottom_view_padding,
        }
    }
}

/// Edge toggles and floors shared by every SafeArea construction path.
#[derive(Clone, Copy, Debug)]
struct SafeAreaPolicy {
    minimum: EdgeInsets,
    left: bool,
    top: bool,
    right: bool,
    bottom: bool,
    maintain_bottom_view_padding: bool,
}

impl SafeAreaPolicy {
    fn padding(self, safe_insets: EdgeInsets, view_padding: EdgeInsets) -> EdgeInsets {
        let ambient = safe_insets.normalized();
        // A maintained bottom edge keeps the persistent obstruction
        // margin, which the shell reports separately from transient
        // occlusion and never reduces under it. The occlusion height is
        // not persistent padding. Disabled edges stay at their minimum.
        let bottom = if self.maintain_bottom_view_padding {
            view_padding.normalized().bottom
        } else {
            ambient.bottom
        };
        EdgeInsets::only(
            if self.left {
                ambient.left.max(self.minimum.left)
            } else {
                self.minimum.left
            },
            if self.top {
                ambient.top.max(self.minimum.top)
            } else {
                self.minimum.top
            },
            if self.right {
                ambient.right.max(self.minimum.right)
            } else {
                self.minimum.right
            },
            if self.bottom {
                bottom.max(self.minimum.bottom)
            } else {
                self.minimum.bottom
            },
        )
    }
}

impl From<SafeArea> for Widget {
    fn from(value: SafeArea) -> Self {
        let policy = value.policy();
        let edges = MediaQueryEdges {
            left: policy.left,
            top: policy.top,
            right: policy.right,
            bottom: policy.bottom,
        };
        let child = MediaQuery::remove_padding(value.child, edges);
        crate::Builder::new(move |context| {
            let view_padding = if policy.maintain_bottom_view_padding && policy.bottom {
                context.view_padding()
            } else {
                EdgeInsets::ZERO
            };
            let padding = policy.padding(context.safe_insets(), view_padding);
            crate::Padding::new(padding, child.clone()).into()
        })
        .into()
    }
}

/// Flutter's `MediaQuery.removePadding`: hides the chosen safe-area sides from
/// `child`'s subtree, so a nested [`SafeArea`] or bar does not pad them again.
/// Sides that are not chosen keep the ambient safe insets.
#[derive(Clone)]
pub struct RemovePadding {
    child: Widget,
    left: bool,
    top: bool,
    right: bool,
    bottom: bool,
}

impl RemovePadding {
    /// Creates a scope that removes no padding until sides are chosen.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            left: false,
            top: false,
            right: false,
            bottom: false,
        }
    }

    /// Removes the left safe inset from `child`'s subtree.
    #[must_use]
    pub fn left(mut self, value: bool) -> Self {
        self.left = value;
        self
    }

    /// Removes the top safe inset from `child`'s subtree.
    #[must_use]
    pub fn top(mut self, value: bool) -> Self {
        self.top = value;
        self
    }

    /// Removes the right safe inset from `child`'s subtree.
    #[must_use]
    pub fn right(mut self, value: bool) -> Self {
        self.right = value;
        self
    }

    /// Removes the bottom safe inset from `child`'s subtree.
    #[must_use]
    pub fn bottom(mut self, value: bool) -> Self {
        self.bottom = value;
        self
    }
}

impl From<RemovePadding> for Widget {
    fn from(value: RemovePadding) -> Self {
        MediaQuery::remove_padding(
            value.child,
            MediaQueryEdges {
                left: value.left,
                top: value.top,
                right: value.right,
                bottom: value.bottom,
            },
        )
    }
}
