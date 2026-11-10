//! Flutter's `DataTable` and `PaginatedDataTable` (`material/data_table.dart`,
//! `material/paginated_data_table.dart`) with their Material 3 defaults.

use std::{cell::Cell, fmt, rc::Rc};

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisAlignment, MainAxisSize};
use incular_core::Color;
use incular_widgets::{
    Align, Alignment, Border, BorderSide, BoxDecoration, Column, Container, GestureDetector, Row,
    SingleChildScrollView, SizedBox, Text, Widget,
};

use super::{Revision, theme_of};
use crate::material_theme::helpers::alpha;
use crate::{Card, Checkbox, IconButton, Icons};

type SortCallback = Rc<dyn Fn(usize, bool)>;
type SelectCallback = Rc<dyn Fn(Option<bool>)>;

/// Flutter's `DataColumn`.
#[derive(Clone)]
pub struct DataColumn {
    pub label: Widget,
    pub tooltip: Option<String>,
    pub numeric: bool,
    pub on_sort: Option<SortCallback>,
}

impl fmt::Debug for DataColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataColumn")
            .field("numeric", &self.numeric)
            .finish_non_exhaustive()
    }
}

impl DataColumn {
    #[must_use]
    pub fn new(label: impl Into<Widget>) -> Self {
        Self {
            label: label.into(),
            tooltip: None,
            numeric: false,
            on_sort: None,
        }
    }

    #[must_use]
    pub fn text(label: impl Into<String>) -> Self {
        Self::new(Text::new(label))
    }

    #[must_use]
    pub fn numeric(mut self, numeric: bool) -> Self {
        self.numeric = numeric;
        self
    }

    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Called with the column index and whether the sort should ascend.
    #[must_use]
    pub fn on_sort(mut self, callback: impl Fn(usize, bool) + 'static) -> Self {
        self.on_sort = Some(Rc::new(callback));
        self
    }
}

/// Flutter's `DataCell`.
#[derive(Clone)]
pub struct DataCell {
    pub child: Widget,
    pub placeholder: bool,
    pub show_edit_icon: bool,
    pub on_tap: Option<Rc<dyn Fn()>>,
}

impl DataCell {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            placeholder: false,
            show_edit_icon: false,
            on_tap: None,
        }
    }

    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(Text::new(text))
    }

    /// Flutter's `DataCell.empty`.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(SizedBox::shrink())
    }

    #[must_use]
    pub fn placeholder(mut self, placeholder: bool) -> Self {
        self.placeholder = placeholder;
        self
    }

    #[must_use]
    pub fn show_edit_icon(mut self, show: bool) -> Self {
        self.show_edit_icon = show;
        self
    }

    #[must_use]
    pub fn on_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self
    }
}

/// Flutter's `DataRow`.
#[derive(Clone)]
pub struct DataRow {
    pub cells: Vec<DataCell>,
    pub selected: bool,
    pub on_select_changed: Option<SelectCallback>,
    pub color: Option<Color>,
}

impl DataRow {
    #[must_use]
    pub fn new(cells: impl IntoIterator<Item = DataCell>) -> Self {
        Self {
            cells: cells.into_iter().collect(),
            selected: false,
            on_select_changed: None,
            color: None,
        }
    }

    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    #[must_use]
    pub fn on_select_changed(mut self, callback: impl Fn(Option<bool>) + 'static) -> Self {
        self.on_select_changed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Flutter's `DataTable`: columns size to their widest cell
/// (`IntrinsicColumnWidth`), with Material 3 metrics.
#[derive(Clone)]
pub struct DataTable {
    columns: Vec<DataColumn>,
    rows: Vec<DataRow>,
    sort_column_index: Option<usize>,
    sort_ascending: bool,
    on_select_all: Option<SelectCallback>,
    data_row_min_height: f32,
    data_row_max_height: f32,
    heading_row_height: f32,
    horizontal_margin: f32,
    column_spacing: f32,
    divider_thickness: f32,
    show_checkbox_column: bool,
    show_bottom_border: bool,
    heading_row_color: Option<Color>,
    data_row_color: Option<Color>,
}

impl DataTable {
    /// Flutter's `kMinInteractiveDimension` default row height.
    pub const DEFAULT_DATA_ROW_HEIGHT: f32 = 48.;
    pub const DEFAULT_HEADING_ROW_HEIGHT: f32 = 56.;
    pub const DEFAULT_HORIZONTAL_MARGIN: f32 = 24.;
    pub const DEFAULT_COLUMN_SPACING: f32 = 56.;
    pub const DEFAULT_DIVIDER_THICKNESS: f32 = 1.;
    const SORT_ARROW_PADDING: f32 = 2.;

    #[must_use]
    pub fn new(
        columns: impl IntoIterator<Item = DataColumn>,
        rows: impl IntoIterator<Item = DataRow>,
    ) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: rows.into_iter().collect(),
            sort_column_index: None,
            sort_ascending: true,
            on_select_all: None,
            data_row_min_height: Self::DEFAULT_DATA_ROW_HEIGHT,
            data_row_max_height: Self::DEFAULT_DATA_ROW_HEIGHT,
            heading_row_height: Self::DEFAULT_HEADING_ROW_HEIGHT,
            horizontal_margin: Self::DEFAULT_HORIZONTAL_MARGIN,
            column_spacing: Self::DEFAULT_COLUMN_SPACING,
            divider_thickness: Self::DEFAULT_DIVIDER_THICKNESS,
            show_checkbox_column: true,
            show_bottom_border: false,
            heading_row_color: None,
            data_row_color: None,
        }
    }

    #[must_use]
    pub fn sort_column_index(mut self, index: Option<usize>) -> Self {
        self.sort_column_index = index;
        self
    }

    #[must_use]
    pub fn sort_ascending(mut self, ascending: bool) -> Self {
        self.sort_ascending = ascending;
        self
    }

    #[must_use]
    pub fn on_select_all(mut self, callback: impl Fn(Option<bool>) + 'static) -> Self {
        self.on_select_all = Some(Rc::new(callback));
        self
    }

    /// Sets both the minimum and maximum data row height
    /// (Flutter's deprecated `dataRowHeight`).
    #[must_use]
    pub fn data_row_height(mut self, height: f32) -> Self {
        self.data_row_min_height = height;
        self.data_row_max_height = height;
        self
    }

    #[must_use]
    pub fn data_row_min_height(mut self, height: f32) -> Self {
        self.data_row_min_height = height;
        self
    }

    #[must_use]
    pub fn data_row_max_height(mut self, height: f32) -> Self {
        self.data_row_max_height = height;
        self
    }

    #[must_use]
    pub fn heading_row_height(mut self, height: f32) -> Self {
        self.heading_row_height = height;
        self
    }

    #[must_use]
    pub fn horizontal_margin(mut self, margin: f32) -> Self {
        self.horizontal_margin = margin;
        self
    }

    #[must_use]
    pub fn column_spacing(mut self, spacing: f32) -> Self {
        self.column_spacing = spacing;
        self
    }

    #[must_use]
    pub fn divider_thickness(mut self, thickness: f32) -> Self {
        self.divider_thickness = thickness;
        self
    }

    #[must_use]
    pub fn show_checkbox_column(mut self, show: bool) -> Self {
        self.show_checkbox_column = show;
        self
    }

    #[must_use]
    pub fn show_bottom_border(mut self, show: bool) -> Self {
        self.show_bottom_border = show;
        self
    }

    #[must_use]
    pub fn heading_row_color(mut self, color: Color) -> Self {
        self.heading_row_color = Some(color);
        self
    }

    #[must_use]
    pub fn data_row_color(mut self, color: Color) -> Self {
        self.data_row_color = Some(color);
        self
    }

    fn shows_checkboxes(&self) -> bool {
        self.show_checkbox_column && self.rows.iter().any(|row| row.on_select_changed.is_some())
    }

    /// The header checkbox state: all, none, or some rows selected.
    fn all_selected(&self) -> Option<bool> {
        let selectable = self
            .rows
            .iter()
            .filter(|row| row.on_select_changed.is_some());
        let (mut any, mut all) = (false, true);
        for row in selectable {
            any |= row.selected;
            all &= row.selected;
        }
        if all && any {
            Some(true)
        } else if any {
            None
        } else {
            Some(false)
        }
    }
}

impl From<DataTable> for Widget {
    fn from(table: DataTable) -> Self {
        let table = Rc::new(table);
        incular_widgets::Builder::new(move |context| build_table(&table, &theme_of(context))).into()
    }
}

fn build_table(table: &DataTable, theme: &crate::ThemeData) -> Widget {
    let scheme = &theme.core().color_scheme;
    let text = &theme.core().text_theme;
    let divider = BorderSide::solid(scheme.outline_variant, table.divider_thickness);
    let row_count = table.rows.len();
    let checkboxes = table.shows_checkboxes();
    let column_count = table.columns.len();
    let row_height = table.data_row_min_height.max(0.);
    let row_max = table.data_row_max_height.max(row_height);

    // `_buildDataRowColor`: selected rows tint with primary at 8%.
    let row_background = |row: &DataRow| -> Option<Color> {
        row.color.or_else(|| {
            if row.selected {
                Some(alpha(scheme.primary, 0.08))
            } else {
                table.data_row_color
            }
        })
    };
    let cell = |child: Widget,
                height: f32,
                max_height: f32,
                padding: EdgeInsets,
                align: Alignment,
                background: Option<Color>,
                bottom: bool,
                on_tap: Option<Rc<dyn Fn()>>| {
        let mut decoration = BoxDecoration::new();
        if let Some(color) = background {
            decoration = decoration.color(color);
        }
        if bottom {
            decoration = decoration.border(Border::only(
                BorderSide::none(),
                BorderSide::none(),
                divider,
                BorderSide::none(),
            ));
        }
        let content: Widget = Container::new()
            .decoration(decoration)
            .padding(padding)
            .constraints(incular_config::Constraints::new(
                0.,
                f32::INFINITY,
                height,
                max_height,
            ))
            .child(Align::new(align, child))
            .into();
        match on_tap {
            Some(on_tap) => GestureDetector::new(content)
                .on_tap(move || on_tap())
                .into(),
            None => content,
        }
    };

    let mut columns: Vec<Widget> = Vec::new();
    if checkboxes {
        let header_state = table.all_selected();
        let on_all = table.on_select_all.clone().or_else(|| {
            // Flutter selects or clears every selectable row itself when the
            // table has no `onSelectAll`.
            let rows = table.rows.clone();
            Some(Rc::new(move |value: Option<bool>| {
                let value = value.unwrap_or(false);
                for row in &rows {
                    if row.selected != value
                        && let Some(callback) = &row.on_select_changed
                    {
                        callback(Some(value));
                    }
                }
            }))
        });
        let padding = EdgeInsets::only(
            table.horizontal_margin,
            0.,
            table.horizontal_margin / 2.,
            0.,
        );
        let mut cells = vec![cell(
            Checkbox::new(header_state == Some(true))
                .value(header_state)
                .tristate(true)
                .on_changed(move |value| {
                    if let Some(on_all) = &on_all {
                        on_all(value);
                    }
                })
                .into(),
            table.heading_row_height,
            table.heading_row_height,
            padding,
            Alignment::CENTER,
            table.heading_row_color,
            true,
            None,
        )];
        for (index, row) in table.rows.iter().enumerate() {
            let callback = row.on_select_changed.clone();
            let selected = row.selected;
            let mut checkbox = Checkbox::new(selected).enabled(callback.is_some());
            if let Some(callback) = callback {
                checkbox = checkbox.on_changed(move |value| callback(value));
            }
            cells.push(cell(
                checkbox.into(),
                row_height,
                row_max,
                padding,
                Alignment::CENTER,
                row_background(row),
                index + 1 < row_count || table.show_bottom_border,
                None,
            ));
        }
        columns.push(
            Column::new(cells)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .into(),
        );
    }

    for (column_index, column) in table.columns.iter().enumerate() {
        let first = column_index == 0 && !checkboxes;
        let last = column_index + 1 == column_count;
        let start = if first {
            table.horizontal_margin
        } else {
            table.column_spacing / 2.
        };
        let end = if last {
            table.horizontal_margin
        } else {
            table.column_spacing / 2.
        };
        let padding = EdgeInsets::only(start, 0., end, 0.);
        let align = if column.numeric {
            Alignment::CENTER_RIGHT
        } else {
            Alignment::CENTER_LEFT
        };

        // `_buildHeadingCell`: label, then a sort arrow that only shows on
        // the sorted column.
        let sorted = table.sort_column_index == Some(column_index);
        let mut heading: Vec<Widget> = Vec::new();
        let label: Widget = incular_widgets::DefaultTextStyle::new(
            text.title_small.clone().color(scheme.on_surface),
            column.label.clone(),
        )
        .into();
        let arrow: Widget = if column.on_sort.is_some() {
            let glyph = if sorted && !table.sort_ascending {
                "↓"
            } else {
                "↑"
            };
            incular_widgets::Opacity::new(
                if sorted { 1. } else { 0. },
                Text::new(glyph)
                    .style(text.title_small.clone())
                    .color(scheme.on_surface),
            )
            .into()
        } else {
            SizedBox::shrink().into()
        };
        let spacer: Widget = SizedBox::new().width(DataTable::SORT_ARROW_PADDING).into();
        if column.numeric {
            heading.extend([arrow, spacer, label]);
        } else {
            heading.extend([label, spacer, arrow]);
        }
        let on_sort = column.on_sort.clone().map(|on_sort| {
            let ascending = if sorted { !table.sort_ascending } else { true };
            Rc::new(move || on_sort(column_index, ascending)) as Rc<dyn Fn()>
        });
        let mut cells = vec![cell(
            Row::new(heading).main_axis_size(MainAxisSize::Min).into(),
            table.heading_row_height,
            table.heading_row_height,
            padding,
            align,
            table.heading_row_color,
            true,
            on_sort,
        )];

        for (row_index, row) in table.rows.iter().enumerate() {
            let data = row
                .cells
                .get(column_index)
                .cloned()
                .unwrap_or_else(DataCell::empty);
            let mut style = text.body_medium.clone().color(scheme.on_surface);
            if data.placeholder {
                style = style.color(alpha(scheme.on_surface, 0.6));
            }
            let mut content: Widget =
                incular_widgets::DefaultTextStyle::new(style, data.child).into();
            if data.show_edit_icon {
                content = Row::new([
                    Widget::from(incular_widgets::Flexible::new(content)),
                    Widget::from(Text::new(Icons::EDIT).color(scheme.on_surface_variant)),
                ])
                .main_axis_size(MainAxisSize::Min)
                .spacing(6.)
                .into();
            }
            // A cell tap wins; otherwise tapping the row toggles selection.
            let on_tap = data.on_tap.clone().or_else(|| {
                row.on_select_changed.clone().map(|callback| {
                    let selected = row.selected;
                    Rc::new(move || callback(Some(!selected))) as Rc<dyn Fn()>
                })
            });
            cells.push(cell(
                content,
                row_height,
                row_max,
                padding,
                align,
                row_background(row),
                row_index + 1 < row_count || table.show_bottom_border,
                on_tap,
            ));
        }
        columns.push(
            Column::new(cells)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .into(),
        );
    }
    Row::new(columns)
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into()
}

/// Flutter's `DataTableSource`: rows for a [`PaginatedDataTable`], built on
/// demand.
pub trait DataTableSource {
    fn get_row(&self, index: usize) -> Option<DataRow>;
    fn row_count(&self) -> usize;
    fn is_row_count_approximate(&self) -> bool {
        false
    }
    fn selected_row_count(&self) -> usize {
        0
    }
}

/// Flutter's `PaginatedDataTable`: a [`DataTable`] in a card showing one page
/// of a [`DataTableSource`], with a header and a page footer.
#[derive(Clone)]
pub struct PaginatedDataTable {
    header: Option<Widget>,
    actions: Vec<Widget>,
    columns: Vec<DataColumn>,
    source: Rc<dyn DataTableSource>,
    rows_per_page: usize,
    available_rows_per_page: Vec<usize>,
    on_rows_per_page_changed: Option<Rc<dyn Fn(usize)>>,
    on_page_changed: Option<Rc<dyn Fn(usize)>>,
    sort_column_index: Option<usize>,
    sort_ascending: bool,
    show_first_last_buttons: bool,
    first_row: Rc<Cell<usize>>,
    revision: Revision,
}

impl PaginatedDataTable {
    pub const DEFAULT_ROWS_PER_PAGE: usize = 10;

    #[must_use]
    pub fn new(
        columns: impl IntoIterator<Item = DataColumn>,
        source: Rc<dyn DataTableSource>,
    ) -> Self {
        Self {
            header: None,
            actions: Vec::new(),
            columns: columns.into_iter().collect(),
            source,
            rows_per_page: Self::DEFAULT_ROWS_PER_PAGE,
            available_rows_per_page: vec![10, 20, 50, 100],
            on_rows_per_page_changed: None,
            on_page_changed: None,
            sort_column_index: None,
            sort_ascending: true,
            show_first_last_buttons: false,
            first_row: Rc::new(Cell::new(0)),
            revision: Revision::default(),
        }
    }

    #[must_use]
    pub fn header(mut self, header: impl Into<Widget>) -> Self {
        self.header = Some(header.into());
        self
    }

    #[must_use]
    pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<Widget>>) -> Self {
        self.actions = actions.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn rows_per_page(mut self, rows: usize) -> Self {
        self.rows_per_page = rows.max(1);
        self
    }

    #[must_use]
    pub fn available_rows_per_page(mut self, options: impl IntoIterator<Item = usize>) -> Self {
        self.available_rows_per_page = options.into_iter().collect();
        self
    }

    #[must_use]
    pub fn on_rows_per_page_changed(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_rows_per_page_changed = Some(Rc::new(callback));
        self
    }

    /// Called with the index of the first row of the new page.
    #[must_use]
    pub fn on_page_changed(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_page_changed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn sort_column_index(mut self, index: Option<usize>) -> Self {
        self.sort_column_index = index;
        self
    }

    #[must_use]
    pub fn sort_ascending(mut self, ascending: bool) -> Self {
        self.sort_ascending = ascending;
        self
    }

    #[must_use]
    pub fn show_first_last_buttons(mut self, show: bool) -> Self {
        self.show_first_last_buttons = show;
        self
    }

    /// Flutter's `initialFirstRowIndex`.
    #[must_use]
    pub fn initial_first_row_index(self, index: usize) -> Self {
        self.first_row.set(index);
        self
    }

    /// Flutter's `PaginatedDataTableState.pageTo`: shows the page holding
    /// `row_index`.
    pub fn page_to(&self, row_index: usize) {
        let first = (row_index / self.rows_per_page) * self.rows_per_page;
        if self.first_row.replace(first) != first {
            if let Some(callback) = &self.on_page_changed {
                callback(first);
            }
            self.revision.bump();
        }
    }

    /// The index of the first row on the current page.
    #[must_use]
    pub fn first_row_index(&self) -> usize {
        self.first_row.get()
    }
}

impl From<PaginatedDataTable> for Widget {
    fn from(table: PaginatedDataTable) -> Self {
        let table = Rc::new(table);
        Widget::stateful_builder(table.revision.cell(), move |context| {
            let theme = theme_of(context);
            let scheme = &theme.core().color_scheme;
            let text = &theme.core().text_theme;
            let count = table.source.row_count();
            let first = table.first_row.get().min(count.saturating_sub(1));
            let rows = (first..(first + table.rows_per_page).min(count))
                .map(|index| {
                    table.source.get_row(index).unwrap_or_else(|| {
                        DataRow::new(table.columns.iter().map(|_| DataCell::empty()))
                    })
                })
                .collect::<Vec<_>>();
            let data_table = DataTable::new(table.columns.clone(), rows)
                .sort_column_index(table.sort_column_index)
                .sort_ascending(table.sort_ascending)
                .show_checkbox_column(true);

            let mut body: Vec<Widget> = Vec::new();
            if table.header.is_some() || !table.actions.is_empty() {
                let selected = table.source.selected_row_count();
                let title: Widget = if selected > 0 {
                    Text::new(format!(
                        "{selected} item{} selected",
                        if selected == 1 { "" } else { "s" }
                    ))
                    .style(text.title_large.clone())
                    .color(scheme.on_surface)
                    .into()
                } else {
                    incular_widgets::DefaultTextStyle::new(
                        text.title_large.clone().color(scheme.on_surface),
                        table
                            .header
                            .clone()
                            .unwrap_or_else(|| SizedBox::shrink().into()),
                    )
                    .into()
                };
                let mut header = vec![Widget::from(incular_widgets::Expanded::new(title))];
                header.extend(table.actions.iter().cloned());
                body.push(
                    Container::new()
                        .height(64.)
                        .padding(EdgeInsets::only(24., 0., 14., 0.))
                        .child(Row::new(header).cross_axis_alignment(CrossAxisAlignment::Center))
                        .into(),
                );
            }
            body.push(
                SingleChildScrollView::new(data_table)
                    .scroll_direction(incular_config::Axis::Horizontal)
                    .into(),
            );

            // `_getFooter`.
            let footer_style = text.body_small.clone().color(scheme.on_surface_variant);
            let last = (first + table.rows_per_page).min(count);
            let range = if count == 0 {
                "0–0 of 0".to_owned()
            } else if table.source.is_row_count_approximate() {
                format!("{}–{} of about {count}", first + 1, last)
            } else {
                format!("{}–{} of {count}", first + 1, last)
            };
            let page = |target: usize| {
                let table = table.clone();
                move || table.page_to(target)
            };
            let per_page = table.rows_per_page;
            let mut footer: Vec<Widget> = Vec::new();
            if let Some(on_change) = table.on_rows_per_page_changed.clone()
                && !table.available_rows_per_page.is_empty()
            {
                let options = table.available_rows_per_page.clone();
                let next = options
                    .iter()
                    .copied()
                    .find(|option| *option > per_page)
                    .unwrap_or(options[0]);
                footer.push(
                    Text::new("Rows per page:")
                        .style(footer_style.clone())
                        .into(),
                );
                footer.push(
                    crate::TextButton::new(format!("{per_page} ⌄"))
                        .on_pressed(move || on_change(next))
                        .into(),
                );
                footer.push(SizedBox::new().width(32.).into());
            }
            footer.push(Text::new(range).style(footer_style).into());
            footer.push(SizedBox::new().width(32.).into());
            let has_previous = first > 0;
            let has_next = last < count;
            if table.show_first_last_buttons {
                footer.push(
                    IconButton::new("⏮")
                        .tooltip("First page")
                        .enabled(has_previous)
                        .on_pressed(page(0))
                        .into(),
                );
            }
            footer.push(
                IconButton::new(Icons::CHEVRON_LEFT)
                    .tooltip("Previous page")
                    .enabled(has_previous)
                    .on_pressed(page(first.saturating_sub(per_page)))
                    .into(),
            );
            footer.push(SizedBox::new().width(24.).into());
            footer.push(
                IconButton::new(Icons::CHEVRON_RIGHT)
                    .tooltip("Next page")
                    .enabled(has_next)
                    .on_pressed(page(first + per_page))
                    .into(),
            );
            if table.show_first_last_buttons {
                footer.push(
                    IconButton::new("⏭")
                        .tooltip("Last page")
                        .enabled(has_next)
                        .on_pressed(page(count.saturating_sub(1)))
                        .into(),
                );
            }
            footer.push(SizedBox::new().width(14.).into());
            body.push(
                Container::new()
                    .height(56.)
                    .decoration(BoxDecoration::new().border(Border::only(
                        BorderSide::solid(scheme.outline_variant, 1.),
                        BorderSide::none(),
                        BorderSide::none(),
                        BorderSide::none(),
                    )))
                    .child(
                        Row::new(footer)
                            .main_axis_alignment(MainAxisAlignment::End)
                            .cross_axis_alignment(CrossAxisAlignment::Center),
                    )
                    .into(),
            );
            Card::new(
                Column::new(body)
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch),
            )
            .into()
        })
    }
}
