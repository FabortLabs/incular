//! The DevTools workbench chrome: sidebar navigation, top bar, status bar and
//! the floating layers (Overlays menu, command palette, dialogs).

use super::icons::IconName;
use super::shared::{
    CARD_RADIUS, chrome, column, count_bubble, divider, dot, floating, glyph, heading, hrow,
    line_text, mono, quiet_button, scrim, spacer, state_surface, strong, surface, surface_raised,
    text_style, warning,
};
use super::{background, danger, gap, primary, success, text_muted, text_primary, ui_text};
use crate::inspector::ConnectionState;
use incular::material::RawMaterialButton;
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

    /// Sidebar groups in tab order; together they list [`Self::ALL`] once.
    pub const GROUPS: [(&'static str, &'static [Self]); 4] = [
        ("Inspect", &[Self::Overview, Self::Widgets]),
        ("Profile", &[Self::Performance, Self::Rendering]),
        ("Trees", &[Self::Semantics, Self::Layers]),
        (
            "Runtime",
            &[
                Self::Memory,
                Self::Console,
                Self::Network,
                Self::Application,
            ],
        ),
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

    /// One-line purpose of the view, shown under its title in the top bar.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Overview => "Health, frame activity and insights for the connected app",
            Self::Widgets => "Browse the retained widget tree, edit properties and inspect layout",
            Self::Console => "Structured target logs and DevTools diagnostics",
            Self::Network => "Local DevTools requests: status, latency and payload sizes",
            Self::Performance => "Frame timings, recordings, flamegraphs and expensive widgets",
            Self::Memory => "Resource inventories, snapshot history and A/B comparison",
            Self::Application => "Target identity, protocol and native windows",
            Self::Rendering => "Text cache, pipeline, semantics and compositor counters",
            Self::Semantics => "The accessibility tree assistive technology sees",
            Self::Layers => "The retained compositor layer tree",
        }
    }

    pub const fn icon(self) -> IconName {
        match self {
            Self::Overview => IconName::Overview,
            Self::Widgets => IconName::Tree,
            Self::Console => IconName::Terminal,
            Self::Network => IconName::Transfer,
            Self::Performance => IconName::Activity,
            Self::Memory => IconName::Chip,
            Self::Application => IconName::AppWindow,
            Self::Rendering => IconName::Monitor,
            Self::Semantics => IconName::Accessibility,
            Self::Layers => IconName::Layers,
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
    /// Opens the command palette (Ctrl+K or Ctrl+Shift+P).
    CommandPalette,
    /// Shows the keyboard shortcut reference (Ctrl+/ or F1).
    ShortcutHelp,
    /// Collapses or expands the sidebar (Ctrl+B).
    ToggleSidebar,
    /// Switches between the dark and light themes (Ctrl+Shift+L).
    ToggleTheme,
}

/// Maps a key press to a command. Every command needs Ctrl (F1 aside), so
/// plain typing in a text field never triggers one.
pub fn shortcut_for(code: Code, modifiers: Modifiers) -> Option<Shortcut> {
    if code == Code::F1 && !modifiers.intersects(Modifiers::CONTROL | Modifiers::ALT) {
        return Some(Shortcut::ShortcutHelp);
    }
    if !modifiers.contains(Modifiers::CONTROL) || modifiers.contains(Modifiers::ALT) {
        return None;
    }
    if modifiers.contains(Modifiers::SHIFT) {
        return match code {
            Code::KeyC => Some(Shortcut::PickWidget),
            Code::KeyP => Some(Shortcut::CommandPalette),
            Code::KeyL => Some(Shortcut::ToggleTheme),
            _ => None,
        };
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
        Code::KeyK => Some(Shortcut::CommandPalette),
        Code::KeyB => Some(Shortcut::ToggleSidebar),
        Code::Slash => Some(Shortcut::ShortcutHelp),
        _ => None,
    }
}

/// Every global shortcut, grouped, as `(group, [(keys, action)])`, for the
/// shortcut reference and the command palette.
pub const SHORTCUT_REFERENCE: [(&str, &[(&str, &str)]); 4] = [
    (
        "General",
        &[
            ("Ctrl+K", "Open the command palette"),
            ("Ctrl+Shift+P", "Open the command palette"),
            ("Ctrl+/", "Show keyboard shortcuts"),
            ("F1", "Show keyboard shortcuts"),
            ("Ctrl+B", "Collapse or expand the sidebar"),
            ("Ctrl+Shift+L", "Switch between dark and light theme"),
            ("Esc", "Close a menu, dialog or the picker"),
        ],
    ),
    (
        "Navigation",
        &[
            ("Ctrl+1 … Ctrl+9", "Open the 1st … 9th view"),
            ("Ctrl+0", "Open Application"),
            ("Ctrl+PgDn", "Next view"),
            ("Ctrl+PgUp", "Previous view"),
        ],
    ),
    (
        "Inspector",
        &[
            ("Ctrl+Shift+C", "Pick a widget in the app"),
            ("↑ ↓", "Move through the tree or search matches"),
            ("← →", "Collapse or expand a branch"),
            ("Home / End", "First or last row"),
            ("Enter", "Select the first search match"),
        ],
    ),
    (
        "Session",
        &[
            ("Ctrl+R", "Refresh, or reconnect when offline"),
            ("Ctrl+E", "Export a diagnostic report"),
        ],
    ),
];

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

/// Contents of the open header Overlays menu.
#[derive(Clone)]
pub struct OverlaysMenu {
    pub panel: Widget,
    pub scroll: ScrollController,
    pub close: std::rc::Rc<dyn Fn()>,
}

/// Counts shown next to sidebar items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavBadges {
    pub console_errors: usize,
    pub console_warnings: usize,
    pub failed_requests: usize,
}

/// A modal layer over the whole workspace: a dimming scrim (clicking it calls
/// `dismiss`) and a panel placed `top` pixels from the window top.
#[derive(Clone)]
pub struct Modal {
    pub panel: Widget,
    pub width: f32,
    pub top: f32,
    pub dismiss: std::rc::Rc<dyn Fn()>,
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
    pub fps_pill: Widget,
    /// Shown in every view while the target picker is on.
    pub picking_pill: Option<Widget>,
    pub overlays_action: Widget,
    /// Overlay controls for the header menu, when it is open.
    pub overlays_menu: Option<OverlaysMenu>,
    pub export_action: Widget,
    pub notice: Option<String>,
    /// Name of the connected (or sought) application in the sidebar card.
    pub target_title: String,
    /// Second line of the sidebar target card.
    pub target_detail: String,
    pub badges: NavBadges,
    pub sidebar_collapsed: Signal<bool>,
    pub sidebar_scroll: ScrollController,
    /// The top bar's "Search commands" trigger.
    pub command_action: Widget,
    /// Sidebar footer button for the shortcut reference.
    pub help_action: Widget,
    /// Command palette or dialog, when one is open.
    pub modal: Option<Modal>,
}

pub const SIDEBAR_WIDTH: f32 = 232.;
pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 60.;
pub const TOP_BAR_HEIGHT: f32 = 60.;
pub const STATUS_BAR_HEIGHT: f32 = 28.;
const BRAND_HEIGHT: f32 = 56.;
const TARGET_HEIGHT: f32 = 66.;
const GROUP_HEIGHT: f32 = 28.;
const ITEM_HEIGHT: f32 = 32.;
const ITEM_GAP: f32 = 2.;
const SIDEBAR_PADDING: f32 = 10.;

/// Where the sidebar item of `view` sits in window coordinates, for tests
/// and scripted reviews that click through the navigation.
pub fn navigation_hit_point(view: ToolView, collapsed: bool) -> Offset {
    let mut y = BRAND_HEIGHT + TARGET_HEIGHT;
    for (_, views) in ToolView::GROUPS {
        y += GROUP_HEIGHT;
        for candidate in views {
            if *candidate == view {
                let x = if collapsed {
                    SIDEBAR_COLLAPSED_WIDTH / 2.
                } else {
                    SIDEBAR_PADDING + 40.
                };
                return Offset::new(x, y + ITEM_HEIGHT / 2.);
            }
            y += ITEM_HEIGHT + ITEM_GAP;
        }
    }
    Offset::ZERO
}

fn connection_color(state: ConnectionState) -> Color {
    match state {
        ConnectionState::Connected => success(),
        ConnectionState::Busy | ConnectionState::TakenOver => warning(),
        ConnectionState::Disconnected => danger(),
        _ => text_muted(),
    }
}

fn nav_item(view: ToolView, data: &ShellData, collapsed: bool, width: f32) -> Widget {
    let selected = data.active_view == view;
    let icon_color = if selected { primary() } else { text_muted() };
    let label_color = if selected {
        text_primary()
    } else {
        text_muted()
    };
    let badge: Option<Widget> = match view {
        ToolView::Console if data.badges.console_errors > 0 => {
            Some(count_bubble(data.badges.console_errors, danger()))
        }
        ToolView::Console if data.badges.console_warnings > 0 => {
            Some(count_bubble(data.badges.console_warnings, warning()))
        }
        ToolView::Network if data.badges.failed_requests > 0 => {
            Some(count_bubble(data.badges.failed_requests, danger()))
        }
        _ => None,
    };
    let content: Widget = if collapsed {
        let mut layers: Vec<Widget> =
            vec![Align::new(Alignment::CENTER, glyph(view.icon(), 17., icon_color)).into()];
        if badge.is_some() {
            let color = if view == ToolView::Console && data.badges.console_errors == 0 {
                warning()
            } else {
                danger()
            };
            layers.push(Positioned::new(dot(color, 7.)).right(7.).top(6.).into());
        }
        Stack::new(layers).into()
    } else {
        let mut row = vec![
            glyph(view.icon(), 16., icon_color),
            gap(11., 1.),
            Expanded::new(
                Text::new(view.label())
                    .style(text_style(13., label_color).font_weight(if selected {
                        FontWeight::W600
                    } else {
                        FontWeight::W500
                    }))
                    .max_lines(Some(1))
                    .overflow(TextOverflow::Ellipsis),
            )
            .into(),
        ];
        if let Some(badge) = badge {
            row.push(badge);
        }
        Padding::new(
            EdgeInsets::symmetric(10., 0.),
            Row::new(row).cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .into()
    };
    let signal = data.tool_view.clone();
    let button = state_surface(
        RawMaterialButton::new(view.label())
            .size(Size::new(width, ITEM_HEIGHT))
            .content(content)
            .on_press(move || {
                signal.set(view);
            }),
        if selected {
            super::shared::control_active()
        } else {
            Color::TRANSPARENT
        },
        8.,
    );
    let item: Widget = SizedBox::from_size(Size::new(width, ITEM_HEIGHT))
        .child(button)
        .into();
    item
}

fn sidebar(data: &ShellData, height: f32) -> Widget {
    let collapsed = data.sidebar_collapsed.get();
    let width = if collapsed {
        SIDEBAR_COLLAPSED_WIDTH
    } else {
        SIDEBAR_WIDTH
    };
    let inner = width - SIDEBAR_PADDING * 2.;
    let brand: Widget = SizedBox::from_size(Size::new(width, BRAND_HEIGHT))
        .child(Align::new(
            if collapsed {
                Alignment::CENTER
            } else {
                Alignment::CENTER_LEFT
            },
            Padding::new(
                EdgeInsets::symmetric(if collapsed { 0. } else { 18. }, 0.),
                if collapsed {
                    crate::logo::mark(24., crate::logo::BRAND_VIOLET)
                } else {
                    hrow([
                        crate::logo::mark(24., crate::logo::BRAND_VIOLET),
                        gap(10., 1.),
                        column([
                            strong("Incular", 14., text_primary()),
                            ui_text("DevTools", 11., text_muted()),
                        ]),
                    ])
                },
            ),
        ))
        .into();
    let status = connection_color(data.connection);
    let target: Widget = SizedBox::from_size(Size::new(width, TARGET_HEIGHT))
        .child(Padding::new(
            EdgeInsets::only(SIDEBAR_PADDING, 4., SIDEBAR_PADDING, 10.),
            if collapsed {
                Widget::from(
                    Container::with_child(Align::new(Alignment::CENTER, dot(status, 9.)))
                        .color(surface_raised())
                        .radius(10.),
                )
            } else {
                Container::with_child(Padding::new(
                    EdgeInsets::symmetric(12., 0.),
                    Row::new([
                        dot(status, 8.),
                        gap(10., 1.),
                        Expanded::new(
                            Column::new([
                                line_text(data.target_title.clone(), 12.5, text_primary()),
                                gap(1., 2.),
                                line_text(data.target_detail.clone(), 11., text_muted()),
                            ])
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .main_axis_alignment(MainAxisAlignment::Center),
                        )
                        .into(),
                    ])
                    .cross_axis_alignment(CrossAxisAlignment::Center),
                ))
                .color(surface_raised())
                .radius(10.)
                .into()
            },
        ))
        .into();
    let mut nav = Vec::new();
    for (group, views) in ToolView::GROUPS {
        nav.push(
            SizedBox::from_size(Size::new(width, GROUP_HEIGHT))
                .child(Align::new(
                    Alignment::BOTTOM_LEFT,
                    Padding::new(
                        EdgeInsets::only(SIDEBAR_PADDING + 10., 0., 0., 7.),
                        if collapsed {
                            Widget::box_(Size::new(inner - 20., 1.), super::border())
                        } else {
                            super::shared::caption(group)
                        },
                    ),
                ))
                .into(),
        );
        for view in views.iter() {
            nav.push(
                Padding::new(
                    EdgeInsets::only(SIDEBAR_PADDING, 0., SIDEBAR_PADDING, ITEM_GAP),
                    nav_item(*view, data, collapsed, inner),
                )
                .into(),
            );
        }
    }
    let toggle = {
        let signal = data.sidebar_collapsed.clone();
        super::shared::icon_button(
            IconName::PanelLeft,
            if collapsed {
                "Expand sidebar (Ctrl+B)"
            } else {
                "Collapse sidebar (Ctrl+B)"
            },
            false,
            move || signal.update(|value| *value = !*value),
        )
    };
    let footer: Widget = if collapsed {
        Column::new([
            data.help_action.clone(),
            gap(1., 4.),
            data.theme_action.clone(),
            gap(1., 4.),
            toggle,
        ])
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into()
    } else {
        Row::new([
            data.theme_action.clone(),
            gap(4., 1.),
            data.help_action.clone(),
            spacer(),
            toggle,
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into()
    };
    let nav_height = (height - BRAND_HEIGHT - TARGET_HEIGHT - 56.).max(80.);
    Container::with_child(
        Column::new([
            brand,
            target,
            SizedBox::from_size(Size::new(width, nav_height))
                .child(ScrollView::vertical(
                    data.sidebar_scroll.clone(),
                    column(nav),
                ))
                .into(),
            spacer(),
            Padding::new(
                EdgeInsets::symmetric(SIDEBAR_PADDING, 12.),
                Align::new(Alignment::CENTER, footer),
            )
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    )
    .width(width)
    .height(height)
    .color(chrome())
    .into()
}

fn top_bar(data: &ShellData) -> Widget {
    let view = data.active_view;
    let mut row = vec![
        Container::with_child(Align::new(
            Alignment::CENTER,
            glyph(view.icon(), 17., primary()),
        ))
        .width(34.)
        .height(34.)
        .color(super::shared::control_active())
        .radius(9.)
        .into(),
        gap(12., 1.),
        Expanded::new(column([
            strong(view.label(), 15., text_primary()),
            gap(1., 2.),
            line_text(view.description(), 11.5, text_muted()),
        ]))
        .into(),
    ];
    if let Some(pill) = data.picking_pill.clone() {
        row.extend([pill, gap(8., 1.)]);
    }
    row.extend([
        data.fps_pill.clone(),
        gap(8., 1.),
        data.overlays_action.clone(),
        gap(8., 1.),
        data.command_action.clone(),
        gap(8., 1.),
        data.refresh_action.clone(),
        gap(2., 1.),
        data.export_action.clone(),
    ]);
    SizedBox::new()
        .height(TOP_BAR_HEIGHT)
        .child(Padding::new(
            EdgeInsets::symmetric(20., 0.),
            Row::new(row).cross_axis_alignment(CrossAxisAlignment::Center),
        ))
        .into()
}

/// A pane card that fills the space it is given and clips its content to
/// the card's rounded corners.
fn pane(child: Widget) -> Widget {
    let mut decoration = BoxDecoration::new()
        .color(surface())
        .border_radius(BorderRadius::circular(CARD_RADIUS));
    let shadow = super::shared::card_shadow();
    if shadow.alpha > 0 {
        decoration = decoration.box_shadow([BoxShadow::new(shadow, Offset::new(0., 1.), 3., 0.)]);
    }
    DecoratedBox::new(ClipRRect::new(CornerRadii::uniform(CARD_RADIUS), child))
        .decoration(decoration)
        .into()
}

fn inspector_body(data: &ShellData, width: f32) -> Widget {
    let tree = pane(
        Column::new([
            Padding::new(
                EdgeInsets::only(16., 14., 14., 10.),
                Row::new([
                    heading("Widget tree", 13.),
                    gap(8., 1.),
                    count_bubble(data.row_count, text_muted()),
                    spacer(),
                    super::shared::kbd("↑↓"),
                    gap(3., 1.),
                    super::shared::kbd("←→"),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            )
            .into(),
            Padding::new(EdgeInsets::symmetric(12., 0.), data.search_field.clone()).into(),
            Padding::new(EdgeInsets::symmetric(10., 6.), data.tree_controls.clone()).into(),
            divider(),
            Expanded::new(data.tree_list.clone()).into(),
            divider(),
            Container::with_child(Padding::new(
                EdgeInsets::symmetric(10., 6.),
                data.tree_breadcrumbs.clone(),
            ))
            .alignment(Alignment::CENTER_LEFT)
            .into(),
            Padding::new(
                EdgeInsets::only(16., 0., 16., 8.),
                line_text(data.tree_status.clone(), 10.5, text_muted()),
            )
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .into(),
    );
    let details = pane(
        Column::new([
            Padding::new(
                EdgeInsets::only(20., 16., 20., 12.),
                data.inspector_header.clone(),
            )
            .into(),
            Padding::new(EdgeInsets::symmetric(16., 0.), data.inspector_tabs.clone()).into(),
            gap(1., 10.),
            divider(),
            Expanded::new(ScrollView::vertical(
                data.inspector_scroll.clone(),
                Padding::all(20., data.page_content.clone()),
            ))
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .into(),
    );
    let split = data.tree_split.clone();
    let available = (width - 32.).max(640.);
    let tree_width = (available * split.get()).clamp(300., available - 360.);
    let panes = SplitView::horizontal(tree, details)
        .first_extent(tree_width)
        .min_first(300.)
        .divider_color(Color::TRANSPARENT)
        .divider_visual_extent(12.)
        .divider_hit_extent(12.)
        .on_split_changed(move |delta| {
            split.set(((tree_width + delta).clamp(300., available - 360.)) / available);
        });
    Padding::new(
        EdgeInsets::only(16., 0., 16., 16.),
        Column::new([
            Widget::from(Padding::new(
                EdgeInsets::only(2., 0., 2., 10.),
                data.inspector_toolbar.clone(),
            )),
            Expanded::new(panes).into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Stretch),
    )
    .into()
}

fn status_bar(data: &ShellData) -> Widget {
    let message = data
        .notice
        .clone()
        .or_else(|| {
            data.picking_pill.is_some().then(|| {
                "Picking: click a widget in the app to inspect it, or press Esc to cancel"
                    .to_owned()
            })
        })
        .unwrap_or_else(|| data.header.clone());
    SizedBox::new()
        .height(STATUS_BAR_HEIGHT)
        .child(
            Container::with_child(Padding::new(
                EdgeInsets::symmetric(16., 0.),
                Row::new([
                    dot(connection_color(data.connection), 7.),
                    gap(8., 1.),
                    Expanded::new(line_text(message, 11., text_muted())).into(),
                    gap(12., 1.),
                    ui_text("Ctrl+K commands · Ctrl+/ shortcuts", 10.5, text_muted()),
                    gap(14., 1.),
                    mono(
                        format!(
                            "{} · {}",
                            data.active_view.label(),
                            data.active_view.shortcut_label()
                        ),
                        10.5,
                        text_muted(),
                    ),
                    gap(14., 1.),
                    mono(
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                        10.5,
                        text_muted(),
                    ),
                ])
                .cross_axis_alignment(CrossAxisAlignment::Center),
            ))
            .color(chrome()),
        )
        .into()
}

/// Builds a bounded native workbench with persistent navigation and inspector chrome.
pub fn build_shell(data: ShellData) -> Widget {
    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().max(720.);
        let height = constraints.max_height().max(480.);
        let side = if data.sidebar_collapsed.get() {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_WIDTH
        };
        let main_width = width - side;
        let body: Widget = if data.active_view == ToolView::Widgets && data.connected {
            inspector_body(&data, main_width)
        } else if data.full_bleed {
            Padding::new(
                EdgeInsets::only(16., 0., 16., 16.),
                data.page_content.clone(),
            )
            .into()
        } else {
            ScrollView::vertical(
                data.inspector_scroll.clone(),
                Padding::new(
                    EdgeInsets::only(20., 2., 20., 28.),
                    data.page_content.clone(),
                ),
            )
        };
        let main: Widget = Container::with_child(
            Column::new([
                top_bar(&data),
                Expanded::new(body).into(),
                status_bar(&data),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Stretch),
        )
        .width(main_width)
        .height(height)
        .color(background())
        .into();
        let workspace: Widget = Row::new([sidebar(&data, height), main])
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .into();
        let mut layers = vec![workspace];
        if let Some(menu) = data.overlays_menu.clone() {
            let menu_height = (height - TOP_BAR_HEIGHT - 24.).clamp(240., 680.);
            let close = menu.close.clone();
            layers.push(
                Positioned::new(
                    SizedBox::new()
                        .width(440.)
                        .height(menu_height)
                        .child(floating(
                            Column::new([
                                Padding::new(
                                    EdgeInsets::only(18., 14., 12., 12.),
                                    Row::new([
                                        heading("Overlays", 14.),
                                        spacer(),
                                        super::shared::kbd("Esc"),
                                        gap(8., 1.),
                                        quiet_button("Close", false, true, move || close()),
                                    ])
                                    .cross_axis_alignment(CrossAxisAlignment::Center),
                                )
                                .into(),
                                divider(),
                                Expanded::new(ScrollView::vertical(
                                    menu.scroll.clone(),
                                    Padding::all(18., menu.panel.clone()),
                                ))
                                .into(),
                            ])
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .into(),
                        )),
                )
                .top(TOP_BAR_HEIGHT - 4.)
                .right(16.)
                .into(),
            );
        }
        if let Some(modal) = data.modal.clone() {
            let dismiss = modal.dismiss.clone();
            layers.push(
                Positioned::new(
                    RawMaterialButton::new("Dismiss dialog")
                        .size(Size::new(width, height))
                        .content(gap(1., 1.))
                        .color(scrim())
                        .hover_color(scrim())
                        .pressed_color(scrim())
                        .on_press(move || dismiss()),
                )
                .left(0.)
                .top(0.)
                .into(),
            );
            let panel_width = modal.width.min(width - 48.);
            layers.push(
                Positioned::new(
                    SizedBox::new()
                        .width(panel_width)
                        .child(floating(modal.panel.clone())),
                )
                .left((width - panel_width) / 2.)
                .top(modal.top.min(height * 0.2))
                .into(),
            );
        }
        SizedBox::from_size(Size::new(width, height))
            .child(Stack::new(layers))
            .into()
    })
    .into()
}
