use super::shared::{badge, heading, mono, text_style, workspace_tab};
use super::{APP_BACKGROUND, BORDER, PRIMARY, SURFACE, TEXT_MUTED, gap, ui_text};
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
}

impl ToolView {
    pub const ALL: [Self; 7] = [
        Self::Overview,
        Self::Widgets,
        Self::Performance,
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
        }
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
                    Widget::box_(Size::new(5., 18.), PRIMARY),
                    gap(9., 1.),
                    heading("incular", 15.),
                    gap(10., 1.),
                    mono("DEVTOOLS", 10., TEXT_MUTED),
                    gap(24., 1.),
                    Expanded::new(
                        Text::new(data.header.clone())
                            .style(text_style(11., TEXT_MUTED))
                            .max_lines(Some(1))
                            .overflow(TextOverflow::Ellipsis),
                    )
                    .into(),
                    badge(
                        if data.connected {
                            "CONNECTED"
                        } else {
                            "OFFLINE"
                        },
                        if data.connected { PRIMARY } else { TEXT_MUTED },
                    ),
                    gap(12., 1.),
                    data.refresh_action.clone(),
                    gap(6., 1.),
                    data.export_action.clone(),
                ]),
            ),
        ))
        .color(APP_BACKGROUND)
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
        let body: Widget = if data.active_view == ToolView::Widgets {
            let tree = Container::with_child(
                Column::new([
                    Padding::new(
                        EdgeInsets::symmetric(14., 10.),
                        Row::new([
                            heading("Widget tree", 12.),
                            gap(8., 1.),
                            ui_text(data.row_count.to_string(), 11., TEXT_MUTED),
                            Expanded::new(gap(1., 1.)).into(),
                            ui_text("↑ ↓ navigate", 10., TEXT_MUTED),
                        ]),
                    )
                    .into(),
                    Padding::new(EdgeInsets::symmetric(12., 0.), data.search_field.clone()).into(),
                    Padding::new(EdgeInsets::symmetric(8., 5.), data.tree_controls.clone()).into(),
                    Widget::box_(Size::new(width, 1.), BORDER),
                    Expanded::new(data.tree_list.clone()).into(),
                    Widget::box_(Size::new(width, 1.), BORDER),
                    Container::with_child(Padding::new(
                        EdgeInsets::symmetric(10., 5.),
                        data.tree_breadcrumbs.clone(),
                    ))
                    .alignment(Alignment::CENTER_LEFT)
                    .into(),
                    Padding::new(
                        EdgeInsets::symmetric(14., 5.),
                        Text::new(data.tree_status.clone())
                            .style(text_style(10., TEXT_MUTED))
                            .max_lines(Some(1))
                            .overflow(TextOverflow::Ellipsis),
                    )
                    .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Start),
            )
            .color(SURFACE);
            let details = Container::with_child(
                Column::new([
                    Padding::new(
                        EdgeInsets::symmetric(16., 12.),
                        data.inspector_header.clone(),
                    )
                    .into(),
                    data.inspector_tabs.clone(),
                    Widget::box_(Size::new(width, 1.), BORDER),
                    Expanded::new(ScrollView::vertical(
                        data.inspector_scroll.clone(),
                        Padding::all(16., data.page_content.clone()),
                    ))
                    .into(),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Start),
            )
            .color(APP_BACKGROUND);
            let split = data.tree_split.clone();
            let tree_width = (width * split.get()).clamp(280., width - 350.);
            let panes = SplitView::horizontal(tree, details)
                .first_extent(tree_width)
                .min_first(280.)
                .divider_color(BORDER)
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
                Widget::box_(Size::new(width, 1.), BORDER),
                Expanded::new(panes).into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .into()
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
                                .unwrap_or("Local diagnostics · Ctrl+Shift+C to pick a widget"),
                        )
                        .style(text_style(10., TEXT_MUTED))
                        .max_lines(Some(1))
                        .overflow(TextOverflow::Ellipsis),
                    )
                    .into(),
                    gap(12., 1.),
                    mono(format!("v{}", env!("CARGO_PKG_VERSION")), 10., TEXT_MUTED),
                ]),
            ),
        ))
        .height(24.)
        .color(SURFACE);
        SizedBox::from_size(Size::new(width, height))
            .child(
                DecoratedBox::new(
                    Column::new([
                        header.into(),
                        Container::with_child(navigation)
                            .height(38.)
                            .alignment(Alignment::CENTER_LEFT)
                            .color(SURFACE)
                            .into(),
                        Widget::box_(Size::new(width, 1.), BORDER),
                        Expanded::new(body).into(),
                        Widget::box_(Size::new(width, 1.), BORDER),
                        footer.into(),
                    ])
                    .cross_axis_alignment(CrossAxisAlignment::Start),
                )
                .background(APP_BACKGROUND),
            )
            .into()
    })
    .into()
}
