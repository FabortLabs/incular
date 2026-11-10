//! Flutter's `CalendarDatePicker`, `DatePickerDialog`/`showDatePicker`,
//! `DateRangePickerDialog`/`showDateRangePicker` and
//! `TimePickerDialog`/`showTimePicker` with Material 3 metrics.
//!
//! Dates are proleptic Gregorian calendar dates ([`Date`]); there is no
//! time zone, as with the date-only `DateTime`s Flutter's pickers use.

use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::Rc,
};

use incular_config::{CrossAxisAlignment, EdgeInsets, MainAxisAlignment, MainAxisSize};
use incular_core::Color;
use incular_widgets::{
    Alignment, Border, BorderRadius, BorderSide, BoxDecoration, BoxShape, Center, Column,
    Container, Expanded, GestureDetector, ListView, Positioned, Row, SizedBox, Stack, Text, Widget,
};

use super::{Revision, theme_of};
use crate::material_theme::helpers::alpha;
use crate::{
    DayPeriod, Dialog, DialogResultHandle, IconButton, Icons, TextButton, TimePickerEntryMode,
    show_dialog_result,
};

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    /// 1 to 12.
    pub month: u32,
    /// 1 to the month's length.
    pub day: u32,
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const SHORT_WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
/// `MaterialLocalizations.narrowWeekdays`, Sunday first.
const NARROW_WEEKDAYS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];

impl Date {
    /// A date, with the month and day clamped into range.
    #[must_use]
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        let month = month.clamp(1, 12);
        Self {
            year,
            month,
            day: day.clamp(1, Self::days_in_month(year, month)),
        }
    }

    #[must_use]
    pub const fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    /// Flutter's `DateUtils.getDaysInMonth`.
    #[must_use]
    pub const fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            2 if Self::is_leap_year(year) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    #[must_use]
    pub fn to_days(self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let yoe = year - era * 400;
        let month = i64::from(self.month);
        let doy =
            (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01.
    #[must_use]
    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Self {
            year: year as i32,
            month,
            day,
        }
    }

    #[must_use]
    pub fn add_days(self, days: i64) -> Self {
        Self::from_days(self.to_days() + days)
    }

    /// 0 for Sunday through 6 for Saturday.
    #[must_use]
    pub fn weekday(self) -> u32 {
        (self.to_days() + 4).rem_euclid(7) as u32
    }

    /// Flutter's `DateUtils.addMonthsToMonthDate`: the first of the month
    /// `months` after this date's month.
    #[must_use]
    pub fn add_months(self, months: i32) -> Self {
        let index = self.year * 12 + self.month as i32 - 1 + months;
        Self {
            year: index.div_euclid(12),
            month: index.rem_euclid(12) as u32 + 1,
            day: 1,
        }
    }

    #[must_use]
    pub fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }

    fn months_between(self, other: Self) -> i32 {
        (other.year - self.year) * 12 + other.month as i32 - self.month as i32
    }

    /// `MaterialLocalizations.formatMonthYear`: "January 2024".
    #[must_use]
    pub fn format_month_year(self) -> String {
        format!("{} {}", MONTHS[self.month as usize - 1], self.year)
    }

    /// `formatMediumDate`: "Mon, Jan 1".
    #[must_use]
    pub fn format_medium(self) -> String {
        format!(
            "{}, {} {}",
            SHORT_WEEKDAYS[self.weekday() as usize],
            SHORT_MONTHS[self.month as usize - 1],
            self.day
        )
    }

    /// `formatShortMonthDay`: "Jan 1".
    #[must_use]
    pub fn format_short_month_day(self) -> String {
        format!("{} {}", SHORT_MONTHS[self.month as usize - 1], self.day)
    }

    /// `formatCompactDate` (en_US): "01/31/2024".
    #[must_use]
    pub fn format_compact(self) -> String {
        format!("{:02}/{:02}/{:04}", self.month, self.day, self.year)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Flutter's `DateTimeRange` over dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateTimeRange {
    pub start: Date,
    pub end: Date,
}

impl DateTimeRange {
    #[must_use]
    pub fn new(start: Date, end: Date) -> Self {
        Self {
            start: start.min(end),
            end: start.max(end),
        }
    }

    /// The number of days from start to end.
    #[must_use]
    pub fn duration_days(&self) -> i64 {
        self.end.to_days() - self.start.to_days()
    }
}

/// Flutter's `TimeOfDay`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay {
    /// 0 to 23.
    pub hour: u32,
    /// 0 to 59.
    pub minute: u32,
}

impl TimeOfDay {
    #[must_use]
    pub fn new(hour: u32, minute: u32) -> Self {
        Self {
            hour: hour % 24,
            minute: minute % 60,
        }
    }

    #[must_use]
    pub fn period(self) -> DayPeriod {
        if self.hour < 12 {
            DayPeriod::Am
        } else {
            DayPeriod::Pm
        }
    }

    /// `hourOfPeriod`: 12 for noon and midnight.
    #[must_use]
    pub fn hour_of_period(self) -> u32 {
        match self.hour % 12 {
            0 => 12,
            hour => hour,
        }
    }

    #[must_use]
    pub fn replacing(self, hour: Option<u32>, minute: Option<u32>) -> Self {
        Self::new(hour.unwrap_or(self.hour), minute.unwrap_or(self.minute))
    }

    /// `MaterialLocalizations.formatTimeOfDay` for en_US: "9:05 AM", or
    /// "09:05" in 24-hour format.
    #[must_use]
    pub fn format(self, always_use_24_hour: bool) -> String {
        if always_use_24_hour {
            format!("{:02}:{:02}", self.hour, self.minute)
        } else {
            let period = if self.period() == DayPeriod::Am {
                "AM"
            } else {
                "PM"
            };
            format!("{}:{:02} {period}", self.hour_of_period(), self.minute)
        }
    }
}

type DateCallback = Rc<dyn Fn(Date)>;
type DatePredicate = Rc<dyn Fn(Date) -> bool>;

/// Flutter's `CalendarDatePicker`: a month grid with month navigation and a
/// year chooser.
#[derive(Clone)]
pub struct CalendarDatePicker {
    initial_date: Option<Date>,
    first_date: Date,
    last_date: Date,
    current_date: Option<Date>,
    on_date_changed: Option<DateCallback>,
    selectable_day_predicate: Option<DatePredicate>,
    state: Rc<CalendarState>,
}

struct CalendarState {
    selected: Cell<Option<Date>>,
    displayed_month: Cell<Date>,
    year_mode: Cell<bool>,
    revision: Revision,
}

/// `_monthPickerHorizontalPadding`, `_subHeaderHeight`, `_dayPickerRowHeight`.
const MONTH_PICKER_HORIZONTAL_PADDING: f32 = 8.;
const SUB_HEADER_HEIGHT: f32 = 52.;
const DAY_PICKER_ROW_HEIGHT: f32 = 42.;
const MAX_DAY_PICKER_ROW_COUNT: usize = 6;
const YEAR_PICKER_ROW_HEIGHT: f32 = 52.;

impl CalendarDatePicker {
    #[must_use]
    pub fn new(initial_date: Option<Date>, first_date: Date, last_date: Date) -> Self {
        let displayed = initial_date
            .unwrap_or(first_date)
            .clamp(first_date, last_date);
        Self {
            initial_date,
            first_date,
            last_date,
            current_date: None,
            on_date_changed: None,
            selectable_day_predicate: None,
            state: Rc::new(CalendarState {
                selected: Cell::new(initial_date),
                displayed_month: Cell::new(displayed.first_of_month()),
                year_mode: Cell::new(false),
                revision: Revision::default(),
            }),
        }
    }

    /// Today, which the grid outlines. Without it, no day is outlined.
    #[must_use]
    pub fn current_date(mut self, date: Date) -> Self {
        self.current_date = Some(date);
        self
    }

    #[must_use]
    pub fn on_date_changed(mut self, callback: impl Fn(Date) + 'static) -> Self {
        self.on_date_changed = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn selectable_day_predicate(mut self, predicate: impl Fn(Date) -> bool + 'static) -> Self {
        self.selectable_day_predicate = Some(Rc::new(predicate));
        self
    }

    /// The selected date, if any.
    #[must_use]
    pub fn selected_date(&self) -> Option<Date> {
        self.state.selected.get()
    }

    /// The first day of the month on display.
    #[must_use]
    pub fn displayed_month(&self) -> Date {
        self.state.displayed_month.get()
    }

    /// Shows the month `months` away, within the first and last dates.
    pub fn show_month(&self, months: i32) {
        let target = self.state.displayed_month.get().add_months(months);
        if target >= self.first_date.first_of_month() && target <= self.last_date.first_of_month() {
            self.state.displayed_month.set(target);
            self.state.revision.bump();
        }
    }

    /// Selects `date` as a tap on its day would.
    pub fn select(&self, date: Date) {
        if !self.is_selectable(date) {
            return;
        }
        self.state.selected.set(Some(date));
        self.state.displayed_month.set(date.first_of_month());
        self.state.revision.bump();
        if let Some(callback) = &self.on_date_changed {
            callback(date);
        }
    }

    fn is_selectable(&self, date: Date) -> bool {
        date >= self.first_date
            && date <= self.last_date
            && self
                .selectable_day_predicate
                .as_ref()
                .is_none_or(|predicate| predicate(date))
    }

    fn toggle_year_mode(&self) {
        self.state.year_mode.set(!self.state.year_mode.get());
        self.state.revision.bump();
    }

    fn choose_year(&self, year: i32) {
        let month = self.state.displayed_month.get().month;
        let target = Date::new(year, month, 1).clamp(
            self.first_date.first_of_month(),
            self.last_date.first_of_month(),
        );
        self.state.displayed_month.set(target);
        self.state.year_mode.set(false);
        self.state.revision.bump();
    }
}

impl From<CalendarDatePicker> for Widget {
    fn from(picker: CalendarDatePicker) -> Self {
        let _ = picker.initial_date;
        let picker = Rc::new(picker);
        Widget::stateful_builder(picker.state.revision.cell(), move |context| {
            let theme = theme_of(context);
            let scheme = theme.core().color_scheme;
            let text = theme.core().text_theme.clone();
            let month = picker.state.displayed_month.get();
            let year_mode = picker.state.year_mode.get();

            // `_DatePickerModeToggleButton`.
            let toggle_label = Row::new([
                Widget::from(
                    Text::new(month.format_month_year())
                        .style(text.title_small.clone())
                        .color(scheme.on_surface_variant),
                ),
                Widget::from(
                    Text::new(if year_mode {
                        Icons::ARROW_DROP_UP
                    } else {
                        Icons::ARROW_DROP_DOWN
                    })
                    .color(scheme.on_surface_variant),
                ),
            ])
            .main_axis_size(MainAxisSize::Min)
            .spacing(4.);
            let toggle_picker = picker.clone();
            let mut header = vec![
                SizedBox::new().width(16.).into(),
                Widget::from(Expanded::new(incular_widgets::Align::new(
                    Alignment::CENTER_LEFT,
                    GestureDetector::new(toggle_label)
                        .on_tap(move || toggle_picker.toggle_year_mode()),
                ))),
            ];
            if !year_mode {
                let can_previous = month > picker.first_date.first_of_month();
                let can_next = month < picker.last_date.first_of_month();
                let previous = picker.clone();
                let next = picker.clone();
                header.push(
                    IconButton::new(Icons::CHEVRON_LEFT)
                        .tooltip("Previous month")
                        .enabled(can_previous)
                        .on_pressed(move || previous.show_month(-1))
                        .into(),
                );
                header.push(
                    IconButton::new(Icons::CHEVRON_RIGHT)
                        .tooltip("Next month")
                        .enabled(can_next)
                        .on_pressed(move || next.show_month(1))
                        .into(),
                );
            }
            let header: Widget = SizedBox::new()
                .height(SUB_HEADER_HEIGHT)
                .child(Row::new(header).cross_axis_alignment(CrossAxisAlignment::Center))
                .into();

            let body = if year_mode {
                year_grid(&picker, &scheme, &text)
            } else {
                day_grid(&picker, month, &scheme, &text)
            };
            Column::new([header, body])
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .into()
        })
    }
}

fn day_cell(
    label: String,
    text_style: incular_text::TextStyle,
    foreground: Color,
    background: Option<Color>,
    outline: Option<Color>,
    on_tap: Option<Rc<dyn Fn()>>,
) -> Widget {
    // `_DayItem`: a 40px circle centered in its cell.
    let mut decoration = BoxDecoration::new().shape(BoxShape::Circle);
    if let Some(color) = background {
        decoration = decoration.color(color);
    }
    if let Some(color) = outline {
        decoration = decoration.border(Border::all(BorderSide::solid(color, 1.)));
    }
    let circle: Widget = Container::new()
        .width(40.)
        .height(40.)
        .decoration(decoration)
        .child(Center::new(
            Text::new(label).style(text_style).color(foreground),
        ))
        .into();
    let cell: Widget = SizedBox::new()
        .height(DAY_PICKER_ROW_HEIGHT)
        .child(Center::new(circle))
        .into();
    let cell = match on_tap {
        Some(on_tap) => GestureDetector::new(cell).on_tap(move || on_tap()).into(),
        None => cell,
    };
    Expanded::new(cell).into()
}

fn weekday_header(scheme: &crate::ColorScheme, text: &crate::TextTheme) -> Widget {
    Row::new(NARROW_WEEKDAYS.iter().map(|day| {
        Widget::from(Expanded::new(
            SizedBox::new()
                .height(DAY_PICKER_ROW_HEIGHT)
                .child(Center::new(
                    Text::new(*day)
                        .style(text.body_large.clone())
                        .color(scheme.on_surface),
                )),
        ))
    }))
    .into()
}

fn day_grid(
    picker: &Rc<CalendarDatePicker>,
    month: Date,
    scheme: &crate::ColorScheme,
    text: &crate::TextTheme,
) -> Widget {
    let selected = picker.state.selected.get();
    let offset = month.weekday() as usize;
    let days = Date::days_in_month(month.year, month.month) as usize;
    let mut rows = vec![weekday_header(scheme, text)];
    let mut cells: Vec<Widget> = (0..offset)
        .map(|_| Widget::from(Expanded::new(SizedBox::new().height(DAY_PICKER_ROW_HEIGHT))))
        .collect();
    for day in 1..=days {
        let date = Date::new(month.year, month.month, day as u32);
        let enabled = picker.is_selectable(date);
        let is_selected = selected == Some(date);
        let is_today = picker.current_date == Some(date);
        // `_DayItem` colors: selected days fill with primary; today is
        // outlined in primary; disabled days fade to 38%.
        let (foreground, background, outline) = if is_selected {
            (scheme.on_primary, Some(scheme.primary), None)
        } else if !enabled {
            (alpha(scheme.on_surface, 0.38), None, None)
        } else if is_today {
            (scheme.primary, None, Some(scheme.primary))
        } else {
            (scheme.on_surface, None, None)
        };
        let on_tap = enabled.then(|| {
            let picker = picker.clone();
            Rc::new(move || picker.select(date)) as Rc<dyn Fn()>
        });
        cells.push(day_cell(
            day.to_string(),
            text.body_large.clone(),
            foreground,
            background,
            outline,
            on_tap,
        ));
    }
    while !cells.len().is_multiple_of(7) {
        cells.push(Expanded::new(SizedBox::new().height(DAY_PICKER_ROW_HEIGHT)).into());
    }
    let mut week = Vec::new();
    for cell in cells {
        week.push(cell);
        if week.len() == 7 {
            rows.push(Row::new(std::mem::take(&mut week)).into());
        }
    }
    // The grid always reserves six weeks, so the dialog never resizes.
    while rows.len() < MAX_DAY_PICKER_ROW_COUNT + 1 {
        rows.push(SizedBox::new().height(DAY_PICKER_ROW_HEIGHT).into());
    }
    Container::new()
        .padding(EdgeInsets::symmetric(MONTH_PICKER_HORIZONTAL_PADDING, 0.))
        .child(Column::new(rows).main_axis_size(MainAxisSize::Min))
        .into()
}

fn year_grid(
    picker: &Rc<CalendarDatePicker>,
    scheme: &crate::ColorScheme,
    text: &crate::TextTheme,
) -> Widget {
    // `YearPicker`: three columns of 72x36 pills in 52px rows.
    let first = picker.first_date.year;
    let last = picker.last_date.year;
    let selected_year = picker.state.selected.get().map(|date| date.year);
    let current_year = picker.current_date.map(|date| date.year);
    let years = (first..=last).collect::<Vec<_>>();
    let rows = years
        .chunks(3)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    let picker = picker.clone();
    let scheme = *scheme;
    let style = text.body_large.clone();
    let list: Widget = ListView::builder(rows.len(), move |row| {
        let cells = rows[row].iter().map(|&year| {
            let selected = selected_year == Some(year);
            let (foreground, background, outline) = if selected {
                (scheme.on_primary, Some(scheme.primary), None)
            } else if current_year == Some(year) {
                (scheme.primary, None, Some(scheme.primary))
            } else {
                (scheme.on_surface_variant, None, None)
            };
            let mut decoration = BoxDecoration::new().border_radius(BorderRadius::circular(18.));
            if let Some(color) = background {
                decoration = decoration.color(color);
            }
            if let Some(color) = outline {
                decoration = decoration.border(Border::all(BorderSide::solid(color, 1.)));
            }
            let pill: Widget = Container::new()
                .width(72.)
                .height(36.)
                .decoration(decoration)
                .child(Center::new(
                    Text::new(year.to_string())
                        .style(style.clone())
                        .color(foreground),
                ))
                .into();
            let picker = picker.clone();
            Widget::from(Expanded::new(
                GestureDetector::new(
                    SizedBox::new()
                        .height(YEAR_PICKER_ROW_HEIGHT)
                        .child(Center::new(pill)),
                )
                .on_tap(move || picker.choose_year(year)),
            ))
        });
        let mut cells = cells.collect::<Vec<_>>();
        while cells.len() < 3 {
            cells.push(Expanded::new(SizedBox::shrink()).into());
        }
        Row::new(cells)
    })
    .into();
    SizedBox::new()
        .height(DAY_PICKER_ROW_HEIGHT * (MAX_DAY_PICKER_ROW_COUNT + 1) as f32)
        .child(
            Container::new()
                .padding(EdgeInsets::symmetric(24., 0.))
                .child(list),
        )
        .into()
}

/// `_buildPickerHeader` for the Material 3 dialogs: help text over the
/// headline.
fn picker_header(theme: &crate::ThemeData, help: &str, headline: String) -> Widget {
    let scheme = &theme.core().color_scheme;
    let text = &theme.core().text_theme;
    Container::new()
        .padding(EdgeInsets::only(24., 16., 12., 12.))
        .child(
            Column::new([
                Widget::from(
                    Text::new(help)
                        .style(text.label_large.clone())
                        .color(scheme.on_surface_variant),
                ),
                SizedBox::new().height(36.).into(),
                Widget::from(
                    Text::new(headline)
                        .style(text.headline_large.clone())
                        .color(scheme.on_surface_variant),
                ),
            ])
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .into()
}

fn picker_actions(
    cancel: &str,
    confirm: &str,
    on_cancel: Rc<dyn Fn()>,
    on_confirm: Rc<dyn Fn()>,
) -> Widget {
    Container::new()
        .padding(EdgeInsets::only(8., 0., 12., 12.))
        .child(
            Row::new([
                Widget::from(TextButton::new(cancel).on_pressed(move || on_cancel())),
                Widget::from(TextButton::new(confirm).on_pressed(move || on_confirm())),
            ])
            .main_axis_alignment(MainAxisAlignment::End)
            .spacing(8.),
        )
        .into()
}

/// The shell every picker dialog shares: `surfaceContainerHigh`, radius 28.
fn picker_dialog(theme: &crate::ThemeData, width: f32, child: Widget) -> Widget {
    Container::new()
        .width(width)
        .decoration(
            BoxDecoration::new()
                .color(theme.core().color_scheme.surface_container_high)
                .border_radius(BorderRadius::circular(28.)),
        )
        .child(child)
        .into()
}

type Slot<T> = Rc<RefCell<Option<DialogResultHandle<T>>>>;

/// Flutter's `DatePickerDialog`: the calendar under a header, with Cancel
/// and OK.
#[derive(Clone)]
pub struct DatePickerDialog {
    pub initial_date: Option<Date>,
    pub first_date: Date,
    pub last_date: Date,
    pub current_date: Option<Date>,
    pub help_text: String,
    pub cancel_text: String,
    pub confirm_text: String,
    pub selectable_day_predicate: Option<Rc<dyn Fn(Date) -> bool>>,
    pub on_cancel: Option<Rc<dyn Fn()>>,
    pub on_confirm: Option<Rc<dyn Fn(Date)>>,
}

impl DatePickerDialog {
    /// `_calendarPortraitDialogSizeM3.width`.
    pub const WIDTH: f32 = 360.;

    #[must_use]
    pub fn new(initial_date: Option<Date>, first_date: Date, last_date: Date) -> Self {
        Self {
            initial_date,
            first_date,
            last_date,
            current_date: None,
            help_text: "Select date".to_owned(),
            cancel_text: "Cancel".to_owned(),
            confirm_text: "OK".to_owned(),
            selectable_day_predicate: None,
            on_cancel: None,
            on_confirm: None,
        }
    }

    #[must_use]
    pub fn current_date(mut self, date: Date) -> Self {
        self.current_date = Some(date);
        self
    }

    #[must_use]
    pub fn help_text(mut self, text: impl Into<String>) -> Self {
        self.help_text = text.into();
        self
    }

    #[must_use]
    pub fn on_confirm(mut self, callback: impl Fn(Date) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_cancel = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn selectable_day_predicate(mut self, predicate: impl Fn(Date) -> bool + 'static) -> Self {
        self.selectable_day_predicate = Some(Rc::new(predicate));
        self
    }
}

impl From<DatePickerDialog> for Widget {
    fn from(dialog: DatePickerDialog) -> Self {
        let selected = Rc::new(Cell::new(dialog.initial_date));
        let revision = Revision::default();
        let mut calendar =
            CalendarDatePicker::new(dialog.initial_date, dialog.first_date, dialog.last_date)
                .on_date_changed({
                    let (selected, revision) = (selected.clone(), revision.clone());
                    move |date| {
                        selected.set(Some(date));
                        revision.bump();
                    }
                });
        if let Some(today) = dialog.current_date {
            calendar = calendar.current_date(today);
        }
        if let Some(predicate) = dialog.selectable_day_predicate.clone() {
            calendar = calendar.selectable_day_predicate(move |date| predicate(date));
        }
        let calendar: Widget = calendar.into();
        let dialog = Rc::new(dialog);
        Widget::stateful_builder(revision.cell(), move |context| {
            let theme = theme_of(context);
            let headline = selected
                .get()
                .map_or_else(|| "Select date".to_owned(), Date::format_medium);
            let on_cancel = dialog.on_cancel.clone().unwrap_or_else(|| Rc::new(|| {}));
            let on_confirm = {
                let (dialog, selected) = (dialog.clone(), selected.clone());
                Rc::new(move || {
                    if let (Some(date), Some(callback)) = (selected.get(), &dialog.on_confirm) {
                        callback(date);
                    }
                }) as Rc<dyn Fn()>
            };
            picker_dialog(
                &theme,
                DatePickerDialog::WIDTH,
                Column::new([
                    picker_header(&theme, &dialog.help_text, headline),
                    crate::Divider::new().into(),
                    calendar.clone(),
                    picker_actions(
                        &dialog.cancel_text,
                        &dialog.confirm_text,
                        on_cancel,
                        on_confirm,
                    ),
                ])
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .into(),
            )
        })
    }
}

fn result_dialog<T: Clone + 'static>(
    build: impl FnOnce(Slot<T>) -> Widget,
) -> DialogResultHandle<T> {
    let slot: Slot<T> = Rc::new(RefCell::new(None));
    let content = build(slot.clone());
    let handle = show_dialog_result::<T>(
        Dialog::new(content)
            .background_color(Color::TRANSPARENT)
            .elevation(0.),
    );
    *slot.borrow_mut() = Some(handle.clone());
    handle
}

fn complete<T: Clone + 'static>(slot: &Slot<T>, value: Option<T>) {
    let handle = slot.borrow().clone();
    if let Some(handle) = handle {
        match value {
            Some(value) => handle.complete(value),
            None => handle.close(),
        }
    }
}

/// Flutter's `showDatePicker`: a [`DatePickerDialog`] whose result handle
/// receives the confirmed date. Present it with
/// [`DialogResultHandle::present`] and call `open`.
#[must_use]
pub fn show_date_picker(dialog: DatePickerDialog) -> DialogResultHandle<Date> {
    result_dialog(|slot| {
        let confirm = dialog.on_confirm.clone();
        let cancel = dialog.on_cancel.clone();
        let cancel_slot = slot.clone();
        dialog
            .on_confirm(move |date| {
                if let Some(confirm) = &confirm {
                    confirm(date);
                }
                complete(&slot, Some(date));
            })
            .on_cancel(move || {
                if let Some(cancel) = &cancel {
                    cancel();
                }
                complete(&cancel_slot, None);
            })
            .into()
    })
}

/// Flutter's `DateRangePickerDialog` in calendar mode: a scrolling list of
/// months where the first tap picks the start and the second the end.
#[derive(Clone)]
pub struct DateRangePickerDialog {
    pub initial_range: Option<DateTimeRange>,
    pub first_date: Date,
    pub last_date: Date,
    pub current_date: Option<Date>,
    pub help_text: String,
    pub save_text: String,
    pub on_confirm: Option<Rc<dyn Fn(DateTimeRange)>>,
    pub on_cancel: Option<Rc<dyn Fn()>>,
}

impl DateRangePickerDialog {
    #[must_use]
    pub fn new(initial_range: Option<DateTimeRange>, first_date: Date, last_date: Date) -> Self {
        Self {
            initial_range,
            first_date,
            last_date,
            current_date: None,
            help_text: "Select range".to_owned(),
            save_text: "Save".to_owned(),
            on_confirm: None,
            on_cancel: None,
        }
    }

    #[must_use]
    pub fn current_date(mut self, date: Date) -> Self {
        self.current_date = Some(date);
        self
    }

    #[must_use]
    pub fn on_confirm(mut self, callback: impl Fn(DateTimeRange) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_cancel = Some(Rc::new(callback));
        self
    }
}

struct RangeState {
    start: Cell<Option<Date>>,
    end: Cell<Option<Date>>,
    revision: Revision,
}

impl RangeState {
    /// `_CalendarDateRangePickerState._updateSelection`.
    fn tap(&self, date: Date) {
        match (self.start.get(), self.end.get()) {
            (Some(start), None) if date >= start => self.end.set(Some(date)),
            _ => {
                self.start.set(Some(date));
                self.end.set(None);
            }
        }
        self.revision.bump();
    }
}

impl From<DateRangePickerDialog> for Widget {
    fn from(dialog: DateRangePickerDialog) -> Self {
        let state = Rc::new(RangeState {
            start: Cell::new(dialog.initial_range.map(|range| range.start)),
            end: Cell::new(dialog.initial_range.map(|range| range.end)),
            revision: Revision::default(),
        });
        let dialog = Rc::new(dialog);
        Widget::stateful_builder(state.revision.cell(), move |context| {
            let theme = theme_of(context);
            let scheme = theme.core().color_scheme;
            let text = theme.core().text_theme.clone();
            let (start, end) = (state.start.get(), state.end.get());
            let headline = format!(
                "{} – {}",
                start.map_or_else(|| "Start date".to_owned(), Date::format_short_month_day),
                end.map_or_else(|| "End date".to_owned(), Date::format_short_month_day)
            );
            let save = {
                let (dialog, state) = (dialog.clone(), state.clone());
                move || {
                    if let (Some(start), Some(callback)) = (state.start.get(), &dialog.on_confirm) {
                        callback(DateTimeRange::new(start, state.end.get().unwrap_or(start)));
                    }
                }
            };
            let close = {
                let dialog = dialog.clone();
                move || {
                    if let Some(callback) = &dialog.on_cancel {
                        callback();
                    }
                }
            };
            let header = Row::new([
                Widget::from(
                    IconButton::new(Icons::CLOSE)
                        .tooltip("Close")
                        .on_pressed(close),
                ),
                Widget::from(Expanded::new(picker_header(
                    &theme,
                    &dialog.help_text,
                    headline,
                ))),
                Widget::from(
                    TextButton::new(dialog.save_text.clone())
                        .enabled(start.is_some())
                        .on_pressed(save),
                ),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start);

            let first_month = dialog.first_date.first_of_month();
            let months = first_month
                .months_between(dialog.last_date.first_of_month())
                .max(0) as usize
                + 1;
            let (state_for_list, dialog_for_list) = (state.clone(), dialog.clone());
            let list: Widget = ListView::builder(months, move |index| {
                range_month(
                    &dialog_for_list,
                    &state_for_list,
                    first_month.add_months(index as i32),
                    &scheme,
                    &text,
                )
            })
            .into();
            picker_dialog(
                &theme,
                DatePickerDialog::WIDTH,
                Column::new([
                    Widget::from(header),
                    weekday_header(&theme.core().color_scheme, &theme.core().text_theme),
                    crate::Divider::new().into(),
                    SizedBox::new()
                        .height(DAY_PICKER_ROW_HEIGHT * 8.)
                        .child(list)
                        .into(),
                ])
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .into(),
            )
        })
    }
}

fn range_month(
    dialog: &DateRangePickerDialog,
    state: &Rc<RangeState>,
    month: Date,
    scheme: &crate::ColorScheme,
    text: &crate::TextTheme,
) -> Widget {
    let (start, end) = (state.start.get(), state.end.get());
    let offset = month.weekday() as usize;
    let days = Date::days_in_month(month.year, month.month);
    let mut cells: Vec<Widget> = (0..offset)
        .map(|_| Widget::from(Expanded::new(SizedBox::new().height(DAY_PICKER_ROW_HEIGHT))))
        .collect();
    for day in 1..=days {
        let date = Date::new(month.year, month.month, day);
        let enabled = date >= dialog.first_date && date <= dialog.last_date;
        let is_end = start == Some(date) || end == Some(date);
        let in_range =
            matches!((start, end), (Some(start), Some(end)) if date > start && date < end);
        // M3 `rangeSelectionBackgroundColor` is `secondaryContainer`.
        let (foreground, background, outline) = if is_end {
            (scheme.on_primary, Some(scheme.primary), None)
        } else if !enabled {
            (alpha(scheme.on_surface, 0.38), None, None)
        } else if in_range {
            (
                scheme.on_secondary_container,
                Some(scheme.secondary_container),
                None,
            )
        } else if dialog.current_date == Some(date) {
            (scheme.primary, None, Some(scheme.primary))
        } else {
            (scheme.on_surface, None, None)
        };
        let on_tap = enabled.then(|| {
            let state = state.clone();
            Rc::new(move || state.tap(date)) as Rc<dyn Fn()>
        });
        cells.push(day_cell(
            day.to_string(),
            text.body_large.clone(),
            foreground,
            background,
            outline,
            on_tap,
        ));
    }
    while !cells.len().is_multiple_of(7) {
        cells.push(Expanded::new(SizedBox::new().height(DAY_PICKER_ROW_HEIGHT)).into());
    }
    let mut rows = vec![Widget::from(
        Container::new()
            .padding(EdgeInsets::only(24., 16., 0., 8.))
            .child(
                Text::new(month.format_month_year())
                    .style(text.title_small.clone())
                    .color(scheme.on_surface_variant),
            ),
    )];
    let mut week = Vec::new();
    for cell in cells {
        week.push(cell);
        if week.len() == 7 {
            rows.push(Row::new(std::mem::take(&mut week)).into());
        }
    }
    Container::new()
        .padding(EdgeInsets::symmetric(MONTH_PICKER_HORIZONTAL_PADDING, 0.))
        .child(
            Column::new(rows)
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch),
        )
        .into()
}

/// Flutter's `showDateRangePicker`.
#[must_use]
pub fn show_date_range_picker(dialog: DateRangePickerDialog) -> DialogResultHandle<DateTimeRange> {
    result_dialog(|slot| {
        let confirm = dialog.on_confirm.clone();
        let cancel = dialog.on_cancel.clone();
        let cancel_slot = slot.clone();
        dialog
            .on_confirm(move |range| {
                if let Some(confirm) = &confirm {
                    confirm(range);
                }
                complete(&slot, Some(range));
            })
            .on_cancel(move || {
                if let Some(cancel) = &cancel {
                    cancel();
                }
                complete(&cancel_slot, None);
            })
            .into()
    })
}

/// Flutter's `TimePickerDialog`: hour and minute fields over a dial of
/// hours or minutes, with an AM/PM selector unless the 24-hour format is
/// used.
#[derive(Clone)]
pub struct TimePickerDialog {
    pub initial_time: TimeOfDay,
    pub entry_mode: TimePickerEntryMode,
    pub always_use_24_hour_format: bool,
    pub help_text: String,
    pub cancel_text: String,
    pub confirm_text: String,
    pub on_confirm: Option<Rc<dyn Fn(TimeOfDay)>>,
    pub on_cancel: Option<Rc<dyn Fn()>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TimeMode {
    Hour,
    Minute,
}

struct TimeState {
    time: Cell<TimeOfDay>,
    mode: Cell<TimeMode>,
    revision: Revision,
}

/// `_kTimePickerDialSize` and number layout radii for Material 3.
const DIAL_SIZE: f32 = 256.;
const DIAL_LABEL_SIZE: f32 = 48.;

impl TimePickerDialog {
    #[must_use]
    pub fn new(initial_time: TimeOfDay) -> Self {
        Self {
            initial_time,
            entry_mode: TimePickerEntryMode::Dial,
            always_use_24_hour_format: false,
            help_text: "Select time".to_owned(),
            cancel_text: "Cancel".to_owned(),
            confirm_text: "OK".to_owned(),
            on_confirm: None,
            on_cancel: None,
        }
    }

    #[must_use]
    pub fn entry_mode(mut self, mode: TimePickerEntryMode) -> Self {
        self.entry_mode = mode;
        self
    }

    /// `MediaQuery.alwaysUse24HourFormat`.
    #[must_use]
    pub fn always_use_24_hour_format(mut self, value: bool) -> Self {
        self.always_use_24_hour_format = value;
        self
    }

    #[must_use]
    pub fn on_confirm(mut self, callback: impl Fn(TimeOfDay) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(callback));
        self
    }

    #[must_use]
    pub fn on_cancel(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_cancel = Some(Rc::new(callback));
        self
    }
}

impl From<TimePickerDialog> for Widget {
    fn from(dialog: TimePickerDialog) -> Self {
        let state = Rc::new(TimeState {
            time: Cell::new(dialog.initial_time),
            mode: Cell::new(TimeMode::Hour),
            revision: Revision::default(),
        });
        let dialog = Rc::new(dialog);
        Widget::stateful_builder(state.revision.cell(), move |context| {
            let theme = theme_of(context);
            let scheme = theme.core().color_scheme;
            let text = theme.core().text_theme.clone();
            let time = state.time.get();
            let mode = state.mode.get();
            let use_24 = dialog.always_use_24_hour_format;

            // `_HourMinuteControl`: 96x80, radius 8; the selected one uses
            // `primaryContainer`.
            let control = |label: String, active: bool, target: TimeMode| -> Widget {
                let state = state.clone();
                let (background, foreground) = if active {
                    (scheme.primary_container, scheme.on_primary_container)
                } else {
                    (scheme.surface_container_highest, scheme.on_surface)
                };
                GestureDetector::new(
                    Container::new()
                        .width(96.)
                        .height(80.)
                        .decoration(
                            BoxDecoration::new()
                                .color(background)
                                .border_radius(BorderRadius::circular(8.)),
                        )
                        .child(Center::new(
                            Text::new(label)
                                .style(text.display_large.clone())
                                .color(foreground),
                        )),
                )
                .on_tap(move || {
                    state.mode.set(target);
                    state.revision.bump();
                })
                .into()
            };
            let hour_label = if use_24 {
                format!("{:02}", time.hour)
            } else {
                time.hour_of_period().to_string()
            };
            let mut fields = vec![
                control(hour_label, mode == TimeMode::Hour, TimeMode::Hour),
                SizedBox::new()
                    .width(24.)
                    .child(Center::new(
                        Text::new(":")
                            .style(text.display_large.clone())
                            .color(scheme.on_surface),
                    ))
                    .into(),
                control(
                    format!("{:02}", time.minute),
                    mode == TimeMode::Minute,
                    TimeMode::Minute,
                ),
            ];
            if !use_24 {
                // `_DayPeriodControl`: a 52x80 outlined pair; the selected
                // half uses `tertiaryContainer`.
                let period = |label: &str, value: DayPeriod| -> Widget {
                    let selected = time.period() == value;
                    let state = state.clone();
                    let mut cell = Container::new().height(40.).child(Center::new(
                        Text::new(label)
                            .style(text.title_medium.clone())
                            .color(if selected {
                                scheme.on_tertiary_container
                            } else {
                                scheme.on_surface_variant
                            }),
                    ));
                    if selected {
                        cell = cell.color(scheme.tertiary_container);
                    }
                    GestureDetector::new(cell)
                        .on_tap(move || {
                            let time = state.time.get();
                            if time.period() != value {
                                state
                                    .time
                                    .set(TimeOfDay::new((time.hour + 12) % 24, time.minute));
                                state.revision.bump();
                            }
                        })
                        .into()
                };
                fields.push(SizedBox::new().width(12.).into());
                fields.push(
                    Container::new()
                        .width(52.)
                        .height(80.)
                        .decoration(
                            BoxDecoration::new()
                                .border(Border::all(BorderSide::solid(scheme.outline, 1.)))
                                .border_radius(BorderRadius::circular(8.)),
                        )
                        .child(Column::new([
                            period("AM", DayPeriod::Am),
                            period("PM", DayPeriod::Pm),
                        ]))
                        .into(),
                );
            }
            let fields: Widget = Row::new(fields)
                .main_axis_alignment(MainAxisAlignment::Center)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .into();

            let mut body = vec![
                Widget::from(
                    Container::new()
                        .padding(EdgeInsets::only(24., 24., 24., 20.))
                        .child(
                            Text::new(dialog.help_text.clone())
                                .style(text.label_medium.clone())
                                .color(scheme.on_surface_variant),
                        ),
                ),
                fields,
            ];
            if !matches!(
                dialog.entry_mode,
                TimePickerEntryMode::Input | TimePickerEntryMode::InputOnly
            ) {
                body.push(SizedBox::new().height(36.).into());
                body.push(Center::new(dial(&state, use_24, &scheme, &text)).into());
            }
            let on_cancel = dialog.on_cancel.clone().unwrap_or_else(|| Rc::new(|| {}));
            let on_confirm = {
                let (dialog, state) = (dialog.clone(), state.clone());
                Rc::new(move || {
                    if let Some(callback) = &dialog.on_confirm {
                        callback(state.time.get());
                    }
                }) as Rc<dyn Fn()>
            };
            body.push(SizedBox::new().height(24.).into());
            body.push(picker_actions(
                &dialog.cancel_text,
                &dialog.confirm_text,
                on_cancel,
                on_confirm,
            ));
            picker_dialog(
                &theme,
                328.,
                Column::new(body)
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .into(),
            )
        })
    }
}

/// `_Dial`: twelve labels on a circle; the selected one sits on a primary
/// disc.
fn dial(
    state: &Rc<TimeState>,
    use_24: bool,
    scheme: &crate::ColorScheme,
    text: &crate::TextTheme,
) -> Widget {
    let time = state.time.get();
    let mode = state.mode.get();
    let radius = DIAL_SIZE / 2.;
    let label_radius = radius - DIAL_LABEL_SIZE / 2. - 4.;
    let mut children: Vec<Widget> = vec![
        Container::new()
            .width(DIAL_SIZE)
            .height(DIAL_SIZE)
            .decoration(
                BoxDecoration::new()
                    .shape(BoxShape::Circle)
                    .color(scheme.surface_container_highest),
            )
            .into(),
    ];
    for index in 0..12_u32 {
        let (label, value, selected) = match mode {
            TimeMode::Hour => {
                let shown = if index == 0 { 12 } else { index };
                let hour = if use_24 {
                    // The 24-hour dial's outer ring (0, 13–23 are reached by
                    // toggling the period).
                    if time.hour >= 12 {
                        if index == 0 { 0 } else { index + 12 }
                    } else {
                        index
                    }
                } else if time.period() == DayPeriod::Pm {
                    index + 12
                } else {
                    index
                };
                let label = if use_24 {
                    format!("{hour:02}")
                } else {
                    shown.to_string()
                };
                (label, hour, time.hour == hour)
            }
            TimeMode::Minute => {
                let minute = index * 5;
                (format!("{minute:02}"), minute, time.minute == minute)
            }
        };
        // Twelve o'clock at the top, clockwise.
        let angle = std::f32::consts::TAU * index as f32 / 12. - std::f32::consts::FRAC_PI_2;
        let x = radius + label_radius * angle.cos() - DIAL_LABEL_SIZE / 2.;
        let y = radius + label_radius * angle.sin() - DIAL_LABEL_SIZE / 2.;
        let mut decoration = BoxDecoration::new().shape(BoxShape::Circle);
        if selected {
            decoration = decoration.color(scheme.primary);
        }
        let state = state.clone();
        children.push(
            Positioned::new(
                GestureDetector::new(
                    Container::new()
                        .width(DIAL_LABEL_SIZE)
                        .height(DIAL_LABEL_SIZE)
                        .decoration(decoration)
                        .child(Center::new(
                            Text::new(label)
                                .style(text.body_large.clone())
                                .color(if selected {
                                    scheme.on_primary
                                } else {
                                    scheme.on_surface
                                }),
                        )),
                )
                .on_tap(move || {
                    let time = state.time.get();
                    match mode {
                        TimeMode::Hour => {
                            state.time.set(time.replacing(Some(value), None));
                            // Flutter moves on to the minutes after an hour
                            // is chosen.
                            state.mode.set(TimeMode::Minute);
                        }
                        TimeMode::Minute => state.time.set(time.replacing(None, Some(value))),
                    }
                    state.revision.bump();
                }),
            )
            .left(x)
            .top(y)
            .into(),
        );
    }
    SizedBox::new()
        .width(DIAL_SIZE)
        .height(DIAL_SIZE)
        .child(Stack::new(children))
        .into()
}

/// Flutter's `showTimePicker`.
#[must_use]
pub fn show_time_picker(dialog: TimePickerDialog) -> DialogResultHandle<TimeOfDay> {
    result_dialog(|slot| {
        let confirm = dialog.on_confirm.clone();
        let cancel = dialog.on_cancel.clone();
        let cancel_slot = slot.clone();
        dialog
            .on_confirm(move |time| {
                if let Some(confirm) = &confirm {
                    confirm(time);
                }
                complete(&slot, Some(time));
            })
            .on_cancel(move || {
                if let Some(cancel) = &cancel {
                    cancel();
                }
                complete(&cancel_slot, None);
            })
            .into()
    })
}
