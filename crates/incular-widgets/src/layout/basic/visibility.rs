//! Visibility and offstage layout primitives.

use typed_builder::TypedBuilder;

use super::SizedBox;
use crate::{BuildContext, Widget, WidgetKind};

/// Conditionally displays a child.
///
/// Hidden children are replaced unless any preservation option is enabled.
/// Each preservation option implies retention; callers need not also enable
/// `maintain_state`. Retained children are measured but occupy no space unless
/// `maintain_size` (or `maintain_interactivity`, which Flutter only allows
/// with it) is enabled (parent minimum constraints still apply). Hidden
/// children never paint, and receive pointer hits only with
/// `maintain_interactivity`. Hidden children are excluded from focus and
/// keyboard delivery unless `maintain_focusability` is set.
///
/// `maintain_animation` keeps tree-driven ticks running while hidden. Otherwise
/// ticks are muted, not paused: elapsed time catches up when shown again. A
/// controller shared with a visible subtree can still advance there.
/// `maintain_semantics` keeps the measured child's semantics available.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Visibility {
    #[builder(default = true)]
    visible: bool,
    #[builder(default)]
    maintain_state: bool,
    #[builder(default)]
    maintain_size: bool,
    #[builder(default)]
    maintain_animation: bool,
    #[builder(default)]
    maintain_semantics: bool,
    #[builder(default)]
    maintain_interactivity: bool,
    #[builder(default)]
    maintain_focusability: bool,
    #[builder(default, setter(strip_option, into))]
    replacement: Option<Widget>,
    #[builder(setter(into))]
    child: Widget,
}

impl Visibility {
    /// Creates a Visibility wrapper.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            visible: true,
            maintain_state: false,
            maintain_size: false,
            maintain_animation: false,
            maintain_semantics: false,
            maintain_interactivity: false,
            maintain_focusability: false,
            replacement: None,
            child: child.into(),
        }
    }

    /// Flutter `Visibility.maintain`: keeps the hidden child's state,
    /// animations, size, semantics, interactivity and focusability.
    #[must_use]
    pub fn maintain(child: impl Into<Widget>) -> Self {
        Self {
            maintain_state: true,
            maintain_size: true,
            maintain_animation: true,
            maintain_semantics: true,
            maintain_interactivity: true,
            maintain_focusability: true,
            ..Self::new(child)
        }
    }

    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    #[must_use]
    pub fn maintain_state(mut self, maintain: bool) -> Self {
        self.maintain_state = maintain;
        self
    }

    #[must_use]
    pub fn maintain_size(mut self, maintain: bool) -> Self {
        self.maintain_size = maintain;
        self
    }

    #[must_use]
    pub fn maintain_animation(mut self, maintain: bool) -> Self {
        self.maintain_animation = maintain;
        self
    }

    #[must_use]
    pub fn maintain_semantics(mut self, maintain: bool) -> Self {
        self.maintain_semantics = maintain;
        self
    }

    /// Flutter `maintainInteractivity`: a hidden child keeps its space and
    /// still receives pointer hits.
    #[must_use]
    pub fn maintain_interactivity(mut self, maintain: bool) -> Self {
        self.maintain_interactivity = maintain;
        self
    }

    /// Allows a retained hidden subtree to keep and request keyboard focus.
    /// This is independent of pointer hit testing.
    #[must_use]
    pub fn maintain_focusability(mut self, maintain: bool) -> Self {
        self.maintain_focusability = maintain;
        self
    }

    #[must_use]
    pub fn replacement(mut self, replacement: impl Into<Widget>) -> Self {
        self.replacement = Some(replacement.into());
        self
    }
}

impl Visibility {
    /// Returns whether every enclosing Visibility is visible, or true if this
    /// context has no Visibility ancestor. Observed ancestors invalidate this
    /// build when their visibility changes.
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> bool {
        context.visibility()
    }

    pub(crate) fn into_sliver_widget(self) -> Widget {
        // A SliverVisibility publishes only the optional ticker gate. It is
        // not an ancestor of type Visibility and does not insert ExcludeFocus.
        let ticker_gate = self.maintain_state && !self.maintain_animation;
        self.into_widget(false, ticker_gate)
    }

    fn into_widget(self, report_visibility: bool, inherited_ticker: bool) -> Widget {
        let replaces = !self.visible
            && !(self.maintain_state
                || self.maintain_size
                || self.maintain_animation
                || self.maintain_semantics
                || self.maintain_interactivity
                || self.maintain_focusability);
        let child = if replaces {
            self.replacement
                .unwrap_or_else(|| SizedBox::shrink().into())
        } else {
            self.child
        };
        Widget::from_kind(WidgetKind::Visibility {
            visible: self.visible || replaces,
            reported_visible: report_visibility.then_some(self.visible),
            inherited_ticker,
            hidden: crate::tree::HiddenVisibility {
                layout: if self.maintain_size || self.maintain_interactivity {
                    crate::tree::HiddenLayout::PreserveSpace
                } else {
                    crate::tree::HiddenLayout::Offstage
                },
                animation: self.maintain_animation,
                semantics: self.maintain_semantics,
                interactivity: self.maintain_interactivity,
            },
            child,
        })
        .with_excluded_focus(!replaces && !self.visible && !self.maintain_focusability)
    }
}

impl From<Visibility> for Widget {
    fn from(value: Visibility) -> Self {
        value.into_widget(true, false)
    }
}

/// Hides its child offstage without unmounting the retained element.
/// The child is measured, retains focus and keeps animations running, but
/// occupies no space beyond parent minimum constraints and has no paint,
/// pointer hits or semantics while offstage.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct Offstage {
    #[builder(default = true)]
    offstage: bool,
    #[builder(setter(into))]
    child: Widget,
}

impl Offstage {
    /// Creates an Offstage widget.
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            offstage: true,
            child: child.into(),
        }
    }

    #[must_use]
    pub fn offstage(mut self, offstage: bool) -> Self {
        self.offstage = offstage;
        self
    }
}

impl From<Offstage> for Widget {
    fn from(value: Offstage) -> Self {
        Visibility::new(value.child)
            .visible(!value.offstage)
            .maintain_state(true)
            .maintain_animation(true)
            .maintain_focusability(true)
            .into_widget(false, false)
    }
}
