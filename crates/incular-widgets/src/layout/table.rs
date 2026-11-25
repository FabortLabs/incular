//! Row-major table layout descriptor.

use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

pub use incular_layout::{TableCellVerticalAlignment, TableColumnWidth};

use crate::{TextDirection, Widget, WidgetKind, macros::widget_from_kind};

/// A row-major table with Flutter's column-width and cell-alignment rules.
///
/// Columns default to [`TableColumnWidth::Flex`] with factor 1.0, so a table
/// with a bounded width fills it, as Flutter's default `Table` does. Use
/// [`Table::column_width`] for per-column widths and
/// [`Table::default_column_width`] to change the default.
#[derive(Clone, Debug, PartialEq, TypedBuilder, SmartDefault)]
pub struct Table {
    #[default(1)]
    #[builder(default = 1, setter(transform = |columns: usize| columns.max(1)))]
    columns: usize,
    #[builder(default, setter(transform = |spacing: f32| spacing.max(0.0)))]
    column_spacing: f32,
    #[builder(default, setter(transform = |spacing: f32| spacing.max(0.0)))]
    row_spacing: f32,
    #[builder(default)]
    column_widths: Vec<(usize, TableColumnWidth)>,
    #[builder(default)]
    default_column_width: TableColumnWidth,
    #[builder(default)]
    default_vertical_alignment: TableCellVerticalAlignment,
    #[builder(default, setter(strip_option))]
    text_direction: Option<TextDirection>,
    #[builder(default, setter(into))]
    children: Vec<Widget>,
}

impl Table {
    /// Creates a table with a fixed number of columns and children in row-major order.
    #[must_use]
    pub fn new(columns: usize, children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            columns: columns.max(1),
            children: children.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Sets the horizontal spacing between columns.
    #[must_use]
    pub fn column_spacing(mut self, spacing: f32) -> Self {
        self.column_spacing = spacing.max(0.0);
        self
    }

    /// Sets the vertical spacing between rows.
    #[must_use]
    pub fn row_spacing(mut self, spacing: f32) -> Self {
        self.row_spacing = spacing.max(0.0);
        self
    }

    /// Sets the width of one column. The last call for a column wins.
    #[must_use]
    pub fn column_width(mut self, column: usize, width: TableColumnWidth) -> Self {
        self.column_widths.push((column, width));
        self
    }

    /// Sets the width of every column without an explicit [`Self::column_width`].
    #[must_use]
    pub fn default_column_width(mut self, width: TableColumnWidth) -> Self {
        self.default_column_width = width;
        self
    }

    /// Overrides the nearest Directionality for column ordering.
    /// With no override or ambient direction, Incular uses left-to-right.
    #[must_use]
    pub fn text_direction(mut self, direction: TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }

    /// Sets how cells are placed vertically within their row. Defaults to top.
    #[must_use]
    pub fn default_vertical_alignment(mut self, alignment: TableCellVerticalAlignment) -> Self {
        self.default_vertical_alignment = alignment;
        self
    }
}

widget_from_kind!(Table => Table {
    columns,
    column_spacing,
    row_spacing,
    column_widths,
    default_column_width,
    default_vertical_alignment,
    text_direction,
    children
});
