//! Specialized utility widgets built from the basic layout primitives.

use incular_config::{MainAxisAlignment, TextDirection, VerticalDirection};
use typed_builder::TypedBuilder;

use super::Center;
use crate::Widget;

/// Wraps a child subtree with an explicit retained key.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct KeyedSubtree {
    #[builder(setter(into))]
    key: crate::tree::Key,
    #[builder(setter(into))]
    child: Widget,
}

impl KeyedSubtree {
    #[must_use]
    pub fn new(key: impl Into<crate::tree::Key>, child: impl Into<Widget>) -> Self {
        Self {
            key: key.into(),
            child: child.into(),
        }
    }
}

impl From<KeyedSubtree> for Widget {
    fn from(value: KeyedSubtree) -> Self {
        let mut child = value.child;
        if child.key().is_none() {
            child.set_key(Some(value.key));
            return child;
        }
        // Flutter's `KeyedSubtree` keeps the child's own key and adds the
        // wrapper key on an outer element. A transparent builder carries the
        // outer key, so the child's key is never overwritten.
        let inner = child;
        let mut wrapper: Widget = crate::Builder::new(move |_| inner.clone()).into();
        wrapper.set_key(Some(value.key));
        wrapper
    }
}

/// A cell in a Table widget.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct TableCell {
    #[builder(default, setter(strip_option))]
    vertical_alignment: Option<incular_layout::TableCellVerticalAlignment>,
    #[builder(setter(into))]
    child: Widget,
}

impl TableCell {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            vertical_alignment: None,
            child: child.into(),
        }
    }

    /// Replaces the table's vertical alignment for this cell.
    #[must_use]
    pub fn vertical_alignment(
        mut self,
        alignment: incular_layout::TableCellVerticalAlignment,
    ) -> Self {
        self.vertical_alignment = Some(alignment);
        self
    }
}

impl From<TableCell> for Widget {
    fn from(value: TableCell) -> Self {
        use crate::tree::{ChildLayoutMode, WidgetKind};

        let explicit = crate::ExplicitSemantics::new(incular_semantics::Role::Cell);
        let child = value.child;
        let simple_text =
            matches!(child.kind(), WidgetKind::Text { .. }) && !child.has_explicit_semantics();
        let wrapper = Widget::from_kind(WidgetKind::ProxyLayout {
            mode: ChildLayoutMode::TableCell(value.vertical_alignment),
            child,
        })
        .semantics(explicit);
        // A plain Text supplies the cell's label, while interactive
        // content and explicit descendant roles retain their own nodes.
        if simple_text {
            wrapper.merge_semantics()
        } else {
            wrapper
        }
    }
}

/// Horizontal alignment of children when an [`OverflowBar`] forms a column.
///
/// This is the same logical Start, End, and Center set as Wrap, preserving
/// compatibility with callers that used `WrapCrossAlignment`.
pub use incular_config::WrapCrossAlignment as OverflowBarAlignment;

/// Lays out children in one centered row, or one column if the row would not fit.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct OverflowBar {
    #[builder(default = 0.0)]
    spacing: f32,
    #[builder(default = 0.0)]
    overflow_spacing: f32,
    #[builder(default = incular_config::WrapCrossAlignment::Start)]
    overflow_alignment: OverflowBarAlignment,
    #[builder(default, setter(strip_option))]
    alignment: Option<MainAxisAlignment>,
    #[builder(default)]
    overflow_direction: VerticalDirection,
    #[builder(default, setter(strip_option))]
    text_direction: Option<TextDirection>,
    #[builder(
        default = Vec::new(),
        setter(transform = |children: impl IntoIterator<Item = impl Into<Widget>>| {
            children.into_iter().map(Into::into).collect::<Vec<Widget>>()
        })
    )]
    pub(super) children: Vec<Widget>,
}

impl Default for OverflowBar {
    fn default() -> Self {
        Self::new(Vec::<Widget>::new())
    }
}

impl OverflowBar {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            spacing: 0.0,
            overflow_spacing: 0.0,
            overflow_alignment: OverflowBarAlignment::Start,
            alignment: None,
            overflow_direction: VerticalDirection::Down,
            text_direction: None,
            children: children.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    #[must_use]
    pub fn overflow_spacing(mut self, spacing: f32) -> Self {
        self.overflow_spacing = spacing;
        self
    }

    #[must_use]
    pub fn overflow_alignment(mut self, alignment: OverflowBarAlignment) -> Self {
        self.overflow_alignment = alignment;
        self
    }

    /// Expands the horizontal layout to the available width and distributes its children.
    #[must_use]
    pub fn alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    #[must_use]
    pub fn overflow_direction(mut self, direction: VerticalDirection) -> Self {
        self.overflow_direction = direction;
        self
    }

    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }
}

impl From<OverflowBar> for Widget {
    fn from(value: OverflowBar) -> Self {
        Widget::from_kind(crate::tree::WidgetKind::OverflowBar {
            spacing: value.spacing,
            overflow_spacing: value.overflow_spacing,
            overflow_alignment: value.overflow_alignment,
            alignment: value.alignment,
            overflow_direction: value.overflow_direction,
            text_direction: value.text_direction,
            children: value.children,
        })
    }
}

/// App bar navigation toolbar with leading, middle, and trailing slots.
#[derive(Clone, Debug, PartialEq, TypedBuilder)]
pub struct NavigationToolbar {
    #[builder(default, setter(strip_option, into))]
    leading: Option<Widget>,
    #[builder(default, setter(strip_option, into))]
    middle: Option<Widget>,
    #[builder(default, setter(strip_option, into))]
    trailing: Option<Widget>,
    #[builder(default = true)]
    center_middle: bool,
    #[builder(default = 16.0)]
    middle_spacing: f32,
}

impl Default for NavigationToolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationToolbar {
    #[must_use]
    pub fn new() -> Self {
        Self {
            leading: None,
            middle: None,
            trailing: None,
            center_middle: true,
            middle_spacing: 16.0,
        }
    }

    #[must_use]
    pub fn leading(mut self, widget: impl Into<Widget>) -> Self {
        self.leading = Some(widget.into());
        self
    }

    #[must_use]
    pub fn middle(mut self, widget: impl Into<Widget>) -> Self {
        self.middle = Some(widget.into());
        self
    }

    #[must_use]
    pub fn trailing(mut self, widget: impl Into<Widget>) -> Self {
        self.trailing = Some(widget.into());
        self
    }

    #[must_use]
    pub fn center_middle(mut self, center: bool) -> Self {
        self.center_middle = center;
        self
    }
}

impl From<NavigationToolbar> for Widget {
    fn from(value: NavigationToolbar) -> Self {
        let mut row_children = Vec::new();
        if let Some(leading) = value.leading {
            row_children.push(leading);
        }
        if let Some(middle) = value.middle {
            if value.center_middle {
                row_children.push(crate::layout::Expanded::new(Center::new(middle)).into());
            } else {
                row_children.push(middle);
            }
        } else if value.center_middle {
            row_children.push(crate::layout::Spacer::new().into());
        }
        if let Some(trailing) = value.trailing {
            row_children.push(trailing);
        }
        crate::layout::Row::new(row_children)
            .main_axis_alignment(incular_config::MainAxisAlignment::SpaceBetween)
            .cross_axis_alignment(incular_config::CrossAxisAlignment::Center)
            .into()
    }
}
