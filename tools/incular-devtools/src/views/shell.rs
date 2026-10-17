use super::shared::{badge, heading, mono, text_style, warning, workspace_tab};
use super::{background, border, danger, gap, primary, surface, text_muted, ui_text};
use crate::inspector::ConnectionState;
use incular::prelude::*;
use incular::widgets::internal::{ScrollView, SplitView};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolView {
    Overview,
    #[default]
    Widgets,
    Console,
    Network,
    Performance,
    Memory,
    Application,
    Rendering,
    Semantics,
    Layers,
}

impl ToolView {
    pub const ALL: [Self; 10] = [
        Self::Overview,
        Self::Widgets,
        Self::Performance,
        Self::Rendering,
        Self::Semantics,
        Self::Layers,
        Self::Memory,
        Self::Console,
        Self::Network,
        Self::Application,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Widgets => "Widget inspector",
            Self::Console => "Console",
            Self::Network => "Transport",
            Self::Performance => "Performance",
            Self::Memory => "Memory",
            Self::Application => "Application",
            Self::Rendering => "Rendering",
            Self::Semantics => "Semantics",
            Self::Layers => "Layers",
        }
    }
}

impl ToolView {
    /// Shortcut caption for the view, e.g. `Ctrl+3`; the tenth view is `Ctrl+0`.
    pub fn shortcut_label(self) -> String {
        let index = Self::ALL.iter().position(|view| *view == self).unwrap_or(0);
        format!("Ctrl+{}", (index + 1) % 10)
    }

    /// The view `offset` places after this one in tab order, wrapping around.
    pub fn cycled(self, offset: isize) -> Self {
        let count = Self::ALL.len() as isize;
        let index = Self::ALL.iter().position(|view| *view == self).unwrap_or(0) as isize;
        Self::ALL[(index + offset).rem_euclid(count) as usize]
    }
}

/// Application-wide keyboard command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    View(ToolView),
    NextView,
    PreviousView,
    Refresh,
    Export,
    PickWidget,
}

/// Maps a key press to a command. Every command needs Ctrl, so plain typing
/// in a text field never triggers one.
pub fn shortcut_for(code: Code, modifiers: Modifiers) -> Option<Shortcut> {
    if !modifiers.contains(Modifiers::CONTROL) || modifiers.contains(Modifiers::ALT) {
        return None;
    }
    if modifiers.contains(Modifiers::SHIFT) {
        return (code == Code::KeyC).then_some(Shortcut::PickWidget);
    }
    let digit = match code {
        Code::Digit1 | Code::Numpad1 => Some(0),
        Code::Digit2 | Code::Numpad2 => Some(1),
        Code::Digit3 | Code::Numpad3 => Some(2),
        Code::Digit4 | Code::Numpad4 => Some(3),
        Code::Digit5 | Code::Numpad5 => Some(4),
        Code::Digit6 | Code::Numpad6 => Some(5),
        Code::Digit7 | Code::Numpad7 => Some(6),
        Code::Digit8 | Code::Numpad8 => Some(7),
        Code::Digit9 | Code::Numpad9 => Some(8),
        Code::Digit0 | Code::Numpad0 => Some(9),
        _ => None,
    };
    if let Some(index) = digit {
        return ToolView::ALL.get(index).copied().map(Shortcut::View);
    }
    match code {
        Code::PageDown => Some(Shortcut::NextView),
        Code::PageUp => Some(Shortcut::PreviousView),
        Code::KeyR => Some(Shortcut::Refresh),
        Code::KeyE => Some(Shortcut::Export),
        _ => None,
    }
}

pub(crate) fn initial_tool_view() -> ToolView {
    match std::env::var("INCULAR_DEVTOOLS_VIEW")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "console" => ToolView::Console,
        "network" | "transport" => ToolView::Network,
        "performance" => ToolView::Performance,
        "memory" => ToolView::Memory,
        "application" => ToolView::Application,
        "rendering" => ToolView::Rendering,
        "semantics" => ToolView::Semantics,
        "layers" => ToolView::Layers,
        "inspector" | "signals" | "widgets" => ToolView::Widgets,
        "overview" => ToolView::Overview,
        _ => ToolView::Widgets,
    }
}

/// Content and persistent navigation for the native DevTools workspace.
pub struct ShellData {
    pub active_view: ToolView,
    pub tool_view: Signal<ToolView>,
    pub header: String,
    pub connected: bool,
    pub connection: ConnectionState,
    /// Page content fills the workspace itself (it owns its scrolling).
    pub full_bleed: bool,
    pub row_count: usize,
    pub search_field: Widget,
    pub tree_list: Widget,
    pub inspector_toolbar: Widget,
    pub inspector_header: Widget,
    pub inspector_tabs: Widget,
    pub tree_breadcrumbs: Widget,
    pub tree_controls: Widget,
    pub tree_status: String,
    pub tree_split: Signal<f32>,
    pub inspector_scroll: ScrollController,
    pub page_content: Widget,
    pub refresh_action: Widget,
    pub theme_action: Widget,
    pub export_action: Widget,
    pub notice: Option<String>,
}

/// Builds a bounded native workbench with persistent navigation and inspector chrome.
pub fn build_shell(data: ShellData) -> Widget {
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().max(720.);
        let height = constraints.max_height().max(480.);
        let header = Container::with_child(Align::new(
            Alignment::CENTER_LEFT,
            Padding::new(
                EdgeInsets::symmetric(16., 0.),
                Row::new([
                    Widget::box_(Size::new(5., 18.), primary()),
                    gap(9., 1.),
                    heading("incular", 15.),
                    gap(10., 1.),
                    mono("DEVTOOLS", 10., text_muted()),
                    gap(24., 1.),
                    Expanded::new(
                        Text::new(data.header.clone())
                            .style(text_style(11., text_muted()))
                            .max_lines(Some(1))
                            .overflow(TextOverflow::Ellipsis),
                    )
                    .into(),
                    badge(
                        data.connection.badge(),
                        match data.connection {
                            ConnectionState::Connected => primary(),
                            ConnectionState::Busy | ConnectionState::TakenOver => warning(),
                            ConnectionState::Disconnected => danger(),
                            _ => text_muted(),
                        },
                    ),
                    gap(12., 1.),
                    data.theme_action.clone(),
                    gap(6., 1.),
                    data.refresh_action.clone(),
                    gap(6., 1.),
                    data.export_action.clone(),
                ]),
            ),
        ))
        .color(background())
        .height(42.);
        let navigation = Padding::new(
            EdgeInsets::symmetric(8., 0.),
            Row::new(ToolView::ALL.into_iter().map(|view| {
                let signal = data.tool_view.clone();
                workspace_tab(view.label(), data.active_view == view, false, move || {
                    signal.set(view);
                })
            })),
        );
        let body: Widget = if data.active_view == ToolView::Widgets && data.connected {
            let tree = Container::with_child(
                Column::new([
                    Padding::new(
                        EdgeInsets::symmetric(14., 10.),
                        Row::new([
                            heading("Widget tree", 12.),
                            gap(8., 1.),
                            ui_text(data.row_count.to_string(), 11., text_muted()),
                            Expanded::new(gap(1., 1.)).into(),
                            ui_text("↑ ↓ navigate", 10., text_muted()),
                        ]),
                    )
                    .into(),
                    Padding::new(EdgeInsets::symmetric(12., 0.), data.search_field.clone()).into(),
                    Padding::new(EdgeInsets::symmetric(8., 5.), data.tree_controls.clone()).into(),
                    Widget::box_(Size::new(width, 1.), border()),
                    Expanded::new(data.tree_list.clone()).into(),
                    Widget::box_(Size::new(width, 1.), border()),
                    Container::with_child(Padding::new(
                        EdgeInsets::symmetric(10., 5.),
                        data.tree_breadcrumbs.clone(),
                    ))
                    .alignment(Alignment::CENTER_LEFT)
                    .into(),
                    Padding::new(
                        EdgeInsets::symmetric(14., 5.),
                        Text::new(data.tree_status.clone())
                            .style(text_style(10., text_muted()))
                            .max_lines(Some(1))
                            .overflow(TextOverflow::Ellipsis),
                    )
                    .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Start),
            )
            .color(surface());
            let details = Container::with_child(
                Column::new([
                    Padding::new(
                        EdgeInsets::symmetric(16., 12.),
                        data.inspector_header.clone(),
                    )
                    .into(),
                    data.inspector_tabs.clone(),
                    Widget::box_(Size::new(width, 1.), border()),
                    Expanded::new(ScrollView::vertical(
                        data.inspector_scroll.clone(),
                        Padding::all(16., data.page_content.clone()),
                    ))
                    .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Start),
            )
            .color(background());
            let split = data.tree_split.clone();
            let tree_width = (width * split.get()).clamp(280., width - 350.);
            let panes = SplitView::horizontal(tree, details)
                .first_extent(tree_width)
                .min_first(280.)
                .divider_color(border())
                .divider_visual_extent(1.)
                .divider_hit_extent(8.)
                .on_split_changed(move |delta| {
                    split.set(((tree_width + delta).clamp(280., width - 350.)) / width);
                });
            Column::new([
                Container::with_child(Padding::new(
                    EdgeInsets::symmetric(12., 7.),
                    data.inspector_toolbar.clone(),
                ))
                .alignment(Alignment::CENTER_LEFT)
                .into(),
                Widget::box_(Size::new(width, 1.), border()),
                Expanded::new(panes).into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .into()
        } else if data.full_bleed {
            data.page_content.clone()
        } else {
            ScrollView::vertical(
                data.inspector_scroll.clone(),
                Padding::all(24., data.page_content.clone()),
            )
        };
        let footer = Container::with_child(Align::new(
            Alignment::CENTER_LEFT,
            Padding::new(
                EdgeInsets::symmetric(16., 0.),
                Row::new([
                    Expanded::new(
                        Text::new(
                            data.notice
                                .as_deref()
                                .unwrap_or(
                                "Local diagnostics · Ctrl+Shift+C pick widget · Ctrl+R refresh · Ctrl+E export · Ctrl+PgUp/PgDn cycle views",
                            ),
                        )
                        .style(text_style(10., text_muted()))
                        .max_lines(Some(1))
                        .overflow(TextOverflow::Ellipsis),
                    )
                    .into(),
                    gap(12., 1.),
                    mono(
                        format!(
                            "{} · {}",
                            data.active_view.label(),
                            data.active_view.shortcut_label()
                        ),
                        10.,
                        text_muted(),
                    ),
                    gap(12., 1.),
                    mono(format!("v{}", env!("CARGO_PKG_VERSION")), 10., text_muted()),
                ]),
            ),
        ))
        .height(24.)
        .color(surface());
        SizedBox::from_size(Size::new(width, height))
            .child(
                DecoratedBox::new(
                    Column::new([
                        header.into(),
                        Container::with_child(navigation)
                            .height(38.)
                            .alignment(Alignment::CENTER_LEFT)
                            .color(surface())
                            .into(),
                        Widget::box_(Size::new(width, 1.), border()),
                        Expanded::new(body).into(),
                        Widget::box_(Size::new(width, 1.), border()),
                        footer.into(),
                    ])
                    .cross_axis_alignment(CrossAxisAlignment::Start),
                )
                .background(background()),
            )
            .into()
    })
    .into()
}
