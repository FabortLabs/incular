//! Row-major table layout descriptor.

use smart_default::SmartDefault;
use typed_builder::TypedBuilder;

pub use incular_layout::{TableCellVerticalAlignment, TableColumnWidth};

use std::rc::Rc;

use crate::{TableBorder, TextDirection, Widget, WidgetKind, macros::widget_from_kind};

/// Explicit cell grouping for a rectangular [Table].
///
/// With [Table::from_rows], every row must contain the same number of
/// cells. The flat [Table::new] constructor also supports a ragged tail.
#[derive(Clone, Debug, Default, PartialEq, TypedBuilder)]
pub struct TableRow {
    #[builder(default, setter(into))]
    children: Vec<Widget>,
}

impl TableRow {
    #[must_use]
    pub fn new(children: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        Self {
            children: children.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub fn children(&self) -> &[Widget] {
        &self.children
    }
}

/// A row in an explicitly rectangular table had a different cell count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableRowLengthError {
    /// Zero-based index of the first irregular row.
    pub row: usize,
    /// Number of cells in the first row.
    pub expected: usize,
    /// Number of cells supplied in this row.
    pub actual: usize,
}

impl std::fmt::Display for TableRowLengthError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "Table contains irregular row lengths. Row {} has {} cells; expected {}.",
            self.row, self.actual, self.expected,
        )
    }
}

impl std::error::Error for TableRowLengthError {}

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
    #[builder(default, setter(transform = |border: TableBorder| Some(Rc::new(border))))]
    border: Option<Rc<TableBorder>>,
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

    /// Creates a rectangular table from explicit rows.
    ///
    /// # Errors
    ///
    /// Returns [TableRowLengthError] for the first row that does not
    /// have the same number of cells as the first row. An empty list or
    /// a list containing only empty rows lays out as an empty table.
    pub fn from_rows(
        rows: impl IntoIterator<Item = TableRow>,
    ) -> Result<Self, TableRowLengthError> {
        let mut rows = rows.into_iter();
        let Some(TableRow { mut children }) = rows.next() else {
            return Ok(Self::default());
        };
        let columns = children.len();
        for (index, TableRow { children: row }) in rows.enumerate() {
            if row.len() != columns {
                return Err(TableRowLengthError {
                    row: index + 1,
                    expected: columns,
                    actual: row.len(),
                });
            }
            children.extend(row);
        }
        Ok(Self {
            columns: columns.max(1),
            children,
            ..Self::default()
        })
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

    /// Paints explicit grid strokes after the cells. The default is no border.
    #[must_use]
    pub fn border(mut self, border: TableBorder) -> Self {
        self.border = Some(Rc::new(border));
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
    border,
    children
});
