//! Material components composed from the controls and neutral widgets:
//! data tables, pickers, search, segmented buttons, steppers, expansion
//! tiles and panels, carousels, banners, bottom sheets, refresh indicators,
//! the about dialog and license page, grid tiles, animated icons and
//! tab page selectors.

use std::{cell::Cell, rc::Rc};

use incular_widgets::BuildContext;

use crate::{Theme, ThemeData};

mod data_table;
mod flexible_space;
mod pickers;
pub use flexible_space::FlexibleSpaceBar;
mod reorderable;
pub use reorderable::ReorderableListView;
mod about;
mod animated_icon;
mod banner;
mod grid_tile;
mod refresh;
mod sheets;
pub use about::{AboutDialog, AboutListTile, LicenseEntry, LicensePage, LicenseRegistry};
pub use animated_icon::{
    AnimatedIcon, AnimatedIconData, AnimatedIcons, InkRipple, InkSparkle, InkSplash, NoSplash,
};
pub use banner::MaterialBanner;
pub use grid_tile::{GridTile, GridTileBar, TabPageSelector, TabPageSelectorIndicator};
pub use refresh::{RefreshCompleter, RefreshIndicator};
pub use sheets::{ModalBottomSheet, show_modal_bottom_sheet};
mod carousel;
mod expansion;
mod stepper;
pub use carousel::{CarouselController, CarouselView};
pub use expansion::{ExpansionPanel, ExpansionPanelList, ExpansionTile, ExpansionTileController};
pub use stepper::{Step, Stepper};
mod search;
mod segmented;
pub use data_table::{
    DataCell, DataColumn, DataRow, DataTable, DataTableSource, PaginatedDataTable,
};
pub use pickers::{
    CalendarDatePicker, Date, DatePickerDialog, DateRangePickerDialog, DateTimeRange, TimeOfDay,
    TimePickerDialog, show_date_picker, show_date_range_picker, show_time_picker,
};
pub use search::{SearchAnchor, SearchBar, SearchController, search_suggestion};
pub use segmented::{ButtonSegment, SegmentedButton, ToggleButtons};
/// The ambient Material theme, or the light baseline outside one.
pub(crate) fn theme_of(context: &BuildContext<'_>) -> ThemeData {
    Theme::of(context).unwrap_or_else(ThemeData::light)
}

/// A rebuild trigger for a retained component's local state.
#[derive(Clone, Default)]
pub(crate) struct Revision(Rc<Cell<u64>>);

impl Revision {
    pub(crate) fn bump(&self) {
        self.0.set(self.0.get().wrapping_add(1));
    }

    pub(crate) fn cell(&self) -> Rc<Cell<u64>> {
        self.0.clone()
    }
}
