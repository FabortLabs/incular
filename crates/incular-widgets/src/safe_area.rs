//! Declarative SafeArea widget consuming ambient runtime/window safe insets.

use incular_config::EdgeInsets;
use typed_builder::TypedBuilder;

use crate::{Widget, WidgetKind};

/// Insets child from the display edges to avoid platform UI, notches, and status bars.
///
/// Nested instances accumulate: each applies the full ambient padding, so
/// an inner `SafeArea` pads inside the outer one's padding. This matches
/// the platform convention the option names follow. Avoid double-padding
/// by disabling already-handled edges on one level with [`Self::sides`]
/// (for example an inner bar with only the bottom edge enabled inside a
/// full outer guard), or by keeping separate subtrees on separate edges.
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
        let padding = self.policy().padding(insets, EdgeInsets::ZERO);
        crate::Padding::new(padding, self.child).into()
    }

    /// Resolves against both environment signals for use outside the
    /// retained tree, applying the same maintained-padding policy as
    /// retained construction: the bottom edge keeps
    /// `max(safe, minimum, persistent)` when maintenance is enabled.
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
///
/// One policy feeds the retained render kind, the explicit-insets
/// [`SafeArea::resolve`], and the environment-aware
/// [`SafeArea::resolve_with_padding`], so the three paths cannot drift on
/// the max expressions. Passing zero view padding disables maintenance,
/// which is exactly the documented [`SafeArea::resolve`] contract.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SafeAreaPolicy {
    pub(crate) minimum: EdgeInsets,
    pub(crate) left: bool,
    pub(crate) top: bool,
    pub(crate) right: bool,
    pub(crate) bottom: bool,
    pub(crate) maintain_bottom_view_padding: bool,
}

impl SafeAreaPolicy {
    pub(crate) fn padding(self, safe_insets: EdgeInsets, view_padding: EdgeInsets) -> EdgeInsets {
        let ambient = safe_insets.normalized();
        // A maintained bottom edge keeps the persistent obstruction
        // margin, which the shell reports separately from transient
        // occlusion and never reduces under it. The occlusion height is
        // not persistent padding. Disabled edges stay at their minimum.
        let persistent_bottom = if self.maintain_bottom_view_padding {
            view_padding.normalized().bottom.max(0.0)
        } else {
            0.0
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
                ambient
                    .bottom
                    .max(self.minimum.bottom)
                    .max(persistent_bottom)
            } else {
                self.minimum.bottom
            },
        )
    }
}

impl From<SafeArea> for Widget {
    fn from(value: SafeArea) -> Self {
        if value.maintain_bottom_view_padding {
            // Maintained bottom padding needs the persistent view padding,
            // which only the retained render arm reads. Descendants of that
            // arm do not see reduced safe insets (see the layout note below).
            return Widget::from_kind(WidgetKind::SafeArea {
                minimum: value.minimum,
                left: value.left,
                top: value.top,
                right: value.right,
                bottom: value.bottom,
                maintain_bottom_view_padding: true,
                child: value.child,
            });
        }
        let SafeArea {
            child,
            minimum,
            left,
            top,
            right,
            bottom,
            ..
        } = value;
        // Flutter's `SafeArea.build`: the padding comes from the ambient
        // safe insets, and `MediaQuery.removePadding` hides the consumed
        // sides from descendants so a nested SafeArea does not pad twice.
        crate::Builder::new(move |context| {
            let policy = SafeAreaPolicy {
                minimum,
                left,
                top,
                right,
                bottom,
                maintain_bottom_view_padding: false,
            };
            let ambient = context.safe_insets().normalized();
            let padding = policy.padding(ambient, EdgeInsets::ZERO);
            let reduced = EdgeInsets {
                left: if left { 0.0 } else { ambient.left },
                top: if top { 0.0 } else { ambient.top },
                right: if right { 0.0 } else { ambient.right },
                bottom: if bottom { 0.0 } else { ambient.bottom },
            };
            crate::Padding::new(
                padding,
                crate::tree::safe_insets_scope(reduced, child.clone()),
            )
            .into()
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
        let RemovePadding {
            child,
            left,
            top,
            right,
            bottom,
        } = value;
        crate::Builder::new(move |context| {
            let ambient = context.safe_insets().normalized();
            let reduced = EdgeInsets {
                left: if left { 0.0 } else { ambient.left },
                top: if top { 0.0 } else { ambient.top },
                right: if right { 0.0 } else { ambient.right },
                bottom: if bottom { 0.0 } else { ambient.bottom },
            };
            crate::tree::safe_insets_scope(reduced, child.clone())
        })
        .into()
    }
}
