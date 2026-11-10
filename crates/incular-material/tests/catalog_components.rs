//! Material components added for Flutter parity: data tables, pickers,
//! search, segmented buttons, steppers, expansion, carousels, banners,
//! bottom sheets, refresh indicators, about/license pages, grid tiles,
//! animated icons and tab page selectors.

use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_material::*;
use incular_widgets::{Directionality, Text, TextDirection, Widget, internal::WidgetTree};

fn mount(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(
        Theme::new(
            ThemeData::light(),
            Directionality::new(TextDirection::Ltr, widget.into()),
        )
        .into(),
    )
    .expect("mounts");
    tree.layout(Constraints::loose(Size::new(800., 900.)))
        .expect("lays out");
    tree
}

#[test]
fn dates_follow_the_gregorian_calendar() {
    let date = Date::new(2024, 2, 29);
    assert_eq!(date.weekday(), 4, "2024-02-29 is a Thursday");
    assert_eq!(Date::from_days(date.to_days()), date);
    assert_eq!(date.add_days(1), Date::new(2024, 3, 1));
    assert_eq!(Date::new(2023, 2, 31), Date::new(2023, 2, 28));
    assert_eq!(
        Date::new(2024, 1, 15).add_months(-1),
        Date::new(2023, 12, 1)
    );
    assert_eq!(Date::new(1970, 1, 1).to_days(), 0);
    assert_eq!(date.format_medium(), "Thu, Feb 29");
    assert_eq!(TimeOfDay::new(0, 5).format(false), "12:05 AM");
    assert_eq!(TimeOfDay::new(13, 5).format(true), "13:05");
    assert_eq!(
        DateTimeRange::new(Date::new(2024, 1, 3), Date::new(2024, 1, 1)).duration_days(),
        2
    );
}

#[test]
fn calendar_selects_only_enabled_days_and_pages_months() {
    let picked = Rc::new(RefCell::new(Vec::new()));
    let calendar = CalendarDatePicker::new(
        Some(Date::new(2024, 5, 10)),
        Date::new(2024, 1, 1),
        Date::new(2024, 12, 31),
    )
    .selectable_day_predicate(|date| date.weekday() != 0)
    .on_date_changed({
        let picked = picked.clone();
        move |date| picked.borrow_mut().push(date)
    });
    calendar.select(Date::new(2024, 5, 12)); // a Sunday
    calendar.select(Date::new(2025, 1, 2)); // past the last date
    calendar.select(Date::new(2024, 5, 13));
    assert_eq!(*picked.borrow(), [Date::new(2024, 5, 13)]);
    calendar.show_month(-10);
    assert_eq!(
        calendar.displayed_month(),
        Date::new(2024, 5, 1),
        "before January is out of range"
    );
    calendar.show_month(-4);
    assert_eq!(calendar.displayed_month(), Date::new(2024, 1, 1));
    let _ = mount(calendar);
}

#[test]
fn picker_dialogs_build() {
    let first = Date::new(2020, 1, 1);
    let last = Date::new(2030, 12, 31);
    let _ = mount(
        DatePickerDialog::new(Some(Date::new(2024, 6, 1)), first, last)
            .current_date(Date::new(2024, 6, 3)),
    );
    let _ = mount(DateRangePickerDialog::new(
        None,
        Date::new(2024, 1, 1),
        Date::new(2024, 3, 31),
    ));
    let _ = mount(TimePickerDialog::new(TimeOfDay::new(9, 30)));
    let _ = mount(TimePickerDialog::new(TimeOfDay::new(21, 0)).always_use_24_hour_format(true));
    let handle = show_date_picker(DatePickerDialog::new(None, first, last));
    assert!(!handle.is_open());
}

#[test]
fn segmented_button_selection_rules_match_flutter() {
    let single = SegmentedButton::new(
        [
            ButtonSegment::new(1).text("Day"),
            ButtonSegment::new(2).text("Week"),
        ],
        [1],
    );
    assert_eq!(single.selection_after_press(&2), Some(BTreeSet::from([2])));
    assert_eq!(
        single.selection_after_press(&1),
        None,
        "the only selection stays"
    );
    let multi = SegmentedButton::new([ButtonSegment::new(1), ButtonSegment::new(2)], [1])
        .multi_selection_enabled(true)
        .empty_selection_allowed(true);
    assert_eq!(
        multi.selection_after_press(&2),
        Some(BTreeSet::from([1, 2]))
    );
    assert_eq!(multi.selection_after_press(&1), Some(BTreeSet::new()));
    let _ = mount(single.on_selection_changed(|_| {}));
    let _ = mount(
        ToggleButtons::new([Text::new("B"), Text::new("I")], [true, false]).on_pressed(|_| {}),
    );
}

struct Rows;

impl DataTableSource for Rows {
    fn get_row(&self, index: usize) -> Option<DataRow> {
        Some(DataRow::new([
            DataCell::text(format!("Row {index}")),
            DataCell::text(index.to_string()),
        ]))
    }

    fn row_count(&self) -> usize {
        25
    }
}

#[test]
fn data_tables_build_and_paginate() {
    let table = DataTable::new(
        [
            DataColumn::text("Name").on_sort(|_, _| {}),
            DataColumn::text("Age").numeric(true),
        ],
        [
            DataRow::new([DataCell::text("Ada"), DataCell::text("36")]).on_select_changed(|_| {}),
            DataRow::new([DataCell::text("Linus"), DataCell::text("54")]).selected(true),
        ],
    )
    .sort_column_index(Some(0));
    let _ = mount(table);
    let pages = Rc::new(RefCell::new(Vec::new()));
    let paginated = PaginatedDataTable::new(
        [
            DataColumn::text("Name"),
            DataColumn::text("Index").numeric(true),
        ],
        Rc::new(Rows),
    )
    .header(Text::new("Rows"))
    .on_page_changed({
        let pages = pages.clone();
        move |first| pages.borrow_mut().push(first)
    });
    paginated.page_to(13);
    assert_eq!(paginated.first_row_index(), 10);
    let _ = mount(paginated);
    assert_eq!(*pages.borrow(), [10]);
}

#[test]
fn expansion_tile_controller_toggles_and_reports() {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let controller = ExpansionTileController::new();
    let tile = ExpansionTile::text("More")
        .children([Text::new("Hidden")])
        .controller(controller.clone())
        .on_expansion_changed({
            let changes = changes.clone();
            move |expanded| changes.borrow_mut().push(expanded)
        });
    let mut tree = mount(tile);
    controller.expand();
    tree.layout(Constraints::loose(Size::new(800., 900.)))
        .unwrap();
    controller.toggle();
    assert!(!controller.is_expanded());
    assert_eq!(*changes.borrow(), [true, false]);
    let _ = mount(ExpansionPanelList::new([
        ExpansionPanel::new(|_| Text::new("One").into(), Text::new("Body")).is_expanded(true),
        ExpansionPanel::new(|_| Text::new("Two").into(), Text::new("Body")),
    ]));
}

#[test]
fn remaining_components_build() {
    let _ = mount(Stepper::new([
        Step::new(Text::new("Account"), Text::new("Details")).is_active(true),
        Step::new(Text::new("Confirm"), Text::new("Done")).state(StepState::Complete),
    ]));
    let _ = mount(
        Stepper::new([Step::new(Text::new("A"), Text::new("a"))])
            .stepper_type(StepperType::Horizontal),
    );
    let _ = mount(CarouselView::new(200., [Text::new("1"), Text::new("2")]).on_tap(|_| {}));
    let _ = mount(MaterialBanner::new(
        Text::new("Offline"),
        [TextButton::new("Retry")],
    ));
    let _ = mount(SearchBar::new().hint_text("Search"));
    let anchor = SearchAnchor::bar(|_| vec![Text::new("Apple").into()], "Search fruit");
    anchor.controller().open_view();
    let _ = mount(anchor);
    let _ = mount(
        GridTile::new(Text::new("Tile")).footer(GridTileBar::new().title(Text::new("Title"))),
    );
    let _ = mount(TabPageSelector::new(TabController::new(3)));
    let _ = mount(AnimatedIcon::new(AnimatedIcons::MENU_ARROW, 0.5));
    let _ = mount(AboutDialog::new("Incular").version("1.0"));
    LicenseRegistry::add_license(LicenseEntry::with_line_breaks(
        ["incular"],
        "MIT License\n\nPermission is granted.",
    ));
    let _ = mount(LicensePage::new().application_name("Incular"));
    assert_eq!(LicenseRegistry::packages(), ["incular"]);
    let sheet = show_modal_bottom_sheet(|| Text::new("Sheet").into()).show_drag_handle(true);
    assert!(sheet.is_open());
    let _ = mount(sheet.present(Text::new("Page")));
    sheet.close();
    let _ = mount(RefreshIndicator::new(
        incular_widgets::ListView::new([Text::new("Item")]),
        |done| done.complete(),
    ));
    let _ = mount(ReorderableListView::new([Text::new("A"), Text::new("B")]).on_reorder(|_, _| {}));
    let _ = mount(
        FlexibleSpaceBar::new()
            .title(Text::new("Title"))
            .expanded_height(200.),
    );
    assert_eq!(InkSparkle::SPLASH_FACTORY, SplashFactory::Sparkle);
}
