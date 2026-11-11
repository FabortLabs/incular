//! The command palette (Ctrl+K or Ctrl+Shift+P), modelled on the Chrome
//! DevTools command menu: every action of the workbench, searchable by name,
//! group or keyword, runnable with the keyboard alone.
//!
//! The palette owns its query instead of hosting a text field, so it can take
//! focus the moment it opens and keep Up, Down, Enter and Escape for itself.

use super::ToolView;
use super::icons::IconName;
use super::shared::{
    caption, control, divider, gap, glyph, hrow, kbd, line_text, primary, shortcut_keys, spacer,
    state_surface, text_muted, text_primary, text_style,
};
use super::theme::Theme;
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular::widgets::internal::ScrollView;
use incular_devtools_protocol::{DebugOption, DevtoolsProfilerMode};
use std::{cell::RefCell, collections::HashSet, rc::Rc};

/// What a command does when it runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommandAction {
    View(ToolView),
    PickWidget,
    ToggleHoverHighlight,
    ExpandTree,
    CollapseTree,
    RevealSelection,
    ClearSelection,
    CopySelectedWidget,
    ResetOverrides,
    ToggleOverlay(DebugOption),
    ClearOverlays,
    AnimationSpeed(f32),
    ProfilerMode(DevtoolsProfilerMode),
    ToggleRecording,
    ClearRecording,
    CaptureMemory,
    MemoryBaseline,
    MemoryComparison,
    ClearConsole,
    ToggleConsolePause,
    CopyConsole,
    ClearTransport,
    ToggleTheme,
    ToggleSidebar,
    Refresh,
    Reconnect,
    Export,
    CopyTargetInfo,
    ShowShortcuts,
}

/// One entry of the palette.
#[derive(Clone, Debug)]
pub struct Command {
    pub title: String,
    pub group: &'static str,
    pub keywords: &'static str,
    pub shortcut: Option<&'static str>,
    pub icon: IconName,
    pub action: CommandAction,
}

/// The state the catalogue depends on (labels flip with toggles).
#[derive(Clone, Debug, Default)]
pub struct CatalogContext {
    pub connected: bool,
    pub recording: bool,
    pub hover_highlight: bool,
    pub overlays: HashSet<DebugOption>,
    pub theme: Theme,
    pub console_paused: bool,
    pub has_selection: bool,
    pub sidebar_collapsed: bool,
}

fn command(
    group: &'static str,
    title: impl Into<String>,
    icon: IconName,
    action: CommandAction,
) -> Command {
    Command {
        title: title.into(),
        group,
        keywords: "",
        shortcut: None,
        icon,
        action,
    }
}

impl Command {
    fn keys(mut self, shortcut: &'static str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    fn words(mut self, keywords: &'static str) -> Self {
        self.keywords = keywords;
        self
    }
}

/// Every command, in display order for an empty query.
pub fn catalog(context: &CatalogContext) -> Vec<Command> {
    let mut commands = Vec::new();
    for view in ToolView::ALL {
        commands.push(Command {
            title: format!("Go to {}", view.label()),
            group: "Navigation",
            keywords: view.description(),
            shortcut: None,
            icon: view.icon(),
            action: CommandAction::View(view),
        });
    }
    commands.extend([
        command(
            "Inspector",
            "Pick a widget in the app",
            IconName::Crosshair,
            CommandAction::PickWidget,
        )
        .keys("Ctrl+Shift+C")
        .words("select element inspect picker"),
        command(
            "Inspector",
            if context.hover_highlight {
                "Turn off highlight on hover"
            } else {
                "Turn on highlight on hover"
            },
            IconName::Eye,
            CommandAction::ToggleHoverHighlight,
        )
        .words("outline overlay"),
        command(
            "Inspector",
            "Expand the whole widget tree",
            IconName::ChevronDown,
            CommandAction::ExpandTree,
        )
        .words("open all branches"),
        command(
            "Inspector",
            "Collapse the whole widget tree",
            IconName::ChevronRight,
            CommandAction::CollapseTree,
        )
        .words("close all branches"),
        command(
            "Inspector",
            "Reveal the selected widget",
            IconName::Target,
            CommandAction::RevealSelection,
        )
        .words("scroll locate"),
        command(
            "Inspector",
            "Clear the selection",
            IconName::Close,
            CommandAction::ClearSelection,
        ),
        command(
            "Inspector",
            "Copy the selected widget's details",
            IconName::Copy,
            CommandAction::CopySelectedWidget,
        )
        .words("clipboard properties"),
        command(
            "Inspector",
            "Reset all property overrides",
            IconName::Refresh,
            CommandAction::ResetOverrides,
        )
        .words("undo edits"),
    ]);
    for group in &super::overlays::GROUPS {
        for &(option, label) in group.options {
            let on = context.overlays.contains(&option);
            commands.push(
                command(
                    "Overlays",
                    format!("{} {label}", if on { "Hide" } else { "Show" }),
                    IconName::Layers,
                    CommandAction::ToggleOverlay(option),
                )
                .words(group.title),
            );
        }
    }
    commands.push(
        command(
            "Overlays",
            "Turn off every overlay",
            IconName::Close,
            CommandAction::ClearOverlays,
        )
        .words("disable clear reset"),
    );
    for (scale, label) in super::overlays::ANIMATION_SPEEDS {
        commands.push(
            command(
                "Animation",
                if scale == 0. {
                    "Pause animations".to_owned()
                } else {
                    format!("Run animations at {label}")
                },
                if scale == 0. {
                    IconName::Pause
                } else {
                    IconName::Play
                },
                CommandAction::AnimationSpeed(scale),
            )
            .words("slow motion speed time dilation"),
        );
    }
    commands.extend([
        command(
            "Performance",
            if context.recording {
                "Stop recording"
            } else {
                "Start recording"
            },
            if context.recording {
                IconName::Stop
            } else {
                IconName::Record
            },
            CommandAction::ToggleRecording,
        )
        .words("profile trace capture"),
        command(
            "Performance",
            "Clear the recording",
            IconName::Trash,
            CommandAction::ClearRecording,
        ),
        command(
            "Performance",
            "Profiler mode: Basic (frame counts)",
            IconName::Activity,
            CommandAction::ProfilerMode(DevtoolsProfilerMode::Basic),
        ),
        command(
            "Performance",
            "Profiler mode: Timings (phase timings)",
            IconName::Activity,
            CommandAction::ProfilerMode(DevtoolsProfilerMode::Performance),
        ),
        command(
            "Performance",
            "Profiler mode: Deep (per-widget traces)",
            IconName::Activity,
            CommandAction::ProfilerMode(DevtoolsProfilerMode::Deep),
        )
        .words("flamegraph"),
        command(
            "Memory",
            "Capture a memory snapshot",
            IconName::Chip,
            CommandAction::CaptureMemory,
        )
        .words("heap resources"),
        command(
            "Memory",
            "Set memory baseline A",
            IconName::Chip,
            CommandAction::MemoryBaseline,
        )
        .words("compare diff"),
        command(
            "Memory",
            "Capture memory comparison B",
            IconName::Chip,
            CommandAction::MemoryComparison,
        )
        .words("compare diff"),
        command(
            "Console",
            "Clear the console",
            IconName::Trash,
            CommandAction::ClearConsole,
        )
        .words("logs"),
        command(
            "Console",
            if context.console_paused {
                "Resume the live console"
            } else {
                "Pause the console display"
            },
            if context.console_paused {
                IconName::Play
            } else {
                IconName::Pause
            },
            CommandAction::ToggleConsolePause,
        ),
        command(
            "Console",
            "Copy visible console messages",
            IconName::Copy,
            CommandAction::CopyConsole,
        )
        .words("clipboard logs"),
        command(
            "Transport",
            "Clear completed requests",
            IconName::Trash,
            CommandAction::ClearTransport,
        )
        .words("network"),
        command(
            "Appearance",
            match context.theme {
                Theme::Dark => "Switch to the light theme",
                Theme::Light => "Switch to the dark theme",
            },
            match context.theme {
                Theme::Dark => IconName::Sun,
                Theme::Light => IconName::Moon,
            },
            CommandAction::ToggleTheme,
        )
        .keys("Ctrl+Shift+L")
        .words("dark light colour color mode"),
        command(
            "Appearance",
            if context.sidebar_collapsed {
                "Expand the sidebar"
            } else {
                "Collapse the sidebar"
            },
            IconName::PanelLeft,
            CommandAction::ToggleSidebar,
        )
        .keys("Ctrl+B")
        .words("navigation compact"),
        command(
            "Session",
            if context.connected {
                "Refresh target data"
            } else {
                "Reconnect to a target"
            },
            IconName::Refresh,
            if context.connected {
                CommandAction::Refresh
            } else {
                CommandAction::Reconnect
            },
        )
        .keys("Ctrl+R")
        .words("reload"),
        command(
            "Session",
            "Export a diagnostic report",
            IconName::Download,
            CommandAction::Export,
        )
        .keys("Ctrl+E")
        .words("save json"),
        command(
            "Session",
            "Copy target information",
            IconName::Copy,
            CommandAction::CopyTargetInfo,
        )
        .words("clipboard pid platform version"),
        command(
            "Help",
            "Show keyboard shortcuts",
            IconName::Keyboard,
            CommandAction::ShowShortcuts,
        )
        .keys("Ctrl+/")
        .words("keys help reference"),
    ]);
    // Navigation shortcuts follow tab order.
    for (index, entry) in commands.iter_mut().take(ToolView::ALL.len()).enumerate() {
        entry.shortcut = Some(NAV_KEYS[index]);
    }
    commands
}

const NAV_KEYS: [&str; 10] = [
    "Ctrl+1", "Ctrl+2", "Ctrl+3", "Ctrl+4", "Ctrl+5", "Ctrl+6", "Ctrl+7", "Ctrl+8", "Ctrl+9",
    "Ctrl+0",
];

/// Whether every character of `needle` appears in `haystack` in order.
fn subsequence(needle: &str, haystack: &str) -> bool {
    let mut chars = haystack.chars();
    needle.chars().all(|wanted| chars.any(|c| c == wanted))
}

/// Rank of `command` for `query` (lower is better), or `None` when a query
/// word matches nowhere. Title prefixes beat title words, which beat group
/// and keyword hits, which beat in-order letter matches.
pub fn score(command: &Command, query: &str) -> Option<u32> {
    let title = command.title.to_lowercase();
    let extra = format!("{} {}", command.group, command.keywords).to_lowercase();
    let mut total = 0;
    for word in query.split_whitespace() {
        let word = word.to_lowercase();
        total += if title.starts_with(&word) {
            0
        } else if title
            .split(|c: char| !c.is_alphanumeric())
            .any(|part| part.starts_with(&word))
        {
            1
        } else if title.contains(&word) {
            2
        } else if extra.contains(&word) {
            3
        } else if subsequence(&word, &title) {
            5
        } else {
            return None;
        };
    }
    Some(total)
}

/// Commands matching `query`, best first; the catalogue order breaks ties.
pub fn filter(commands: &[Command], query: &str) -> Vec<Command> {
    if query.trim().is_empty() {
        return commands.to_vec();
    }
    let mut ranked = commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| score(command, query).map(|rank| (rank, index, command)))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
    ranked
        .into_iter()
        .map(|(_, _, command)| command.clone())
        .collect()
}

/// Open-palette state that survives rebuilds.
#[derive(Clone)]
pub(crate) struct PaletteState {
    pub(crate) open: Signal<bool>,
    pub(crate) query: Rc<RefCell<String>>,
    pub(crate) selected: Rc<RefCell<usize>>,
    pub(crate) focus: FocusNode,
    pub(crate) scroll: ScrollController,
    /// Takes keyboard focus back when the palette closes, so global
    /// shortcuts keep working.
    return_focus: FocusNode,
}

impl PaletteState {
    pub(crate) fn new(return_focus: FocusNode) -> Self {
        Self {
            open: Signal::new(false),
            query: Rc::new(RefCell::new(String::new())),
            selected: Rc::new(RefCell::new(0)),
            focus: FocusNode::new(),
            scroll: ScrollController::new(),
            return_focus,
        }
    }

    /// Opens with an empty query and the first command selected.
    pub(crate) fn show(&self) {
        self.query.borrow_mut().clear();
        *self.selected.borrow_mut() = 0;
        self.open.set(true);
        self.focus.request_focus();
        self.scroll.deferred_jump_to(0.);
    }

    pub(crate) fn hide(&self) {
        if self.open.get() {
            self.open.set(false);
        }
        self.return_focus.request_focus();
    }
}

const ROW_HEIGHT: f32 = 38.;
const GROUP_HEIGHT: f32 = 28.;
const LIST_HEIGHT: f32 = 372.;
pub(crate) const WIDTH: f32 = 640.;

/// The palette panel. `run` executes a command and is responsible for
/// closing the palette.
pub(crate) fn build(
    state: &PaletteState,
    commands: Vec<Command>,
    tick: Signal<u64>,
    run: Rc<dyn Fn(CommandAction)>,
) -> Widget {
    let query = state.query.borrow().clone();
    let matches = filter(&commands, &query);
    let selected = (*state.selected.borrow()).min(matches.len().saturating_sub(1));
    *state.selected.borrow_mut() = selected;

    let grouped = query.trim().is_empty();
    let mut rows = Vec::new();
    let mut offsets = Vec::new();
    let mut y = 0.;
    let mut last_group = "";
    for (index, entry) in matches.iter().enumerate() {
        if grouped && entry.group != last_group {
            last_group = entry.group;
            rows.push(
                SizedBox::new()
                    .height(GROUP_HEIGHT)
                    .child(Align::new(
                        Alignment::BOTTOM_LEFT,
                        Padding::new(EdgeInsets::only(12., 0., 0., 6.), caption(entry.group)),
                    ))
                    .into(),
            );
            y += GROUP_HEIGHT;
        }
        offsets.push(y);
        y += ROW_HEIGHT;
        let is_selected = index == selected;
        let action = entry.action;
        let run = run.clone();
        let mut row = vec![
            glyph(
                entry.icon,
                15.,
                if is_selected { primary() } else { text_muted() },
            ),
            gap(12., 1.),
            Expanded::new(line_text(entry.title.clone(), 13., text_primary())).into(),
        ];
        if !grouped {
            row.push(line_text(entry.group, 11., text_muted()));
            row.push(gap(10., 1.));
        }
        if let Some(keys) = entry.shortcut {
            row.push(shortcut_keys(keys));
        }
        rows.push(state_surface(
            RawMaterialButton::new(entry.title.clone())
                .size(Size::new(0., ROW_HEIGHT))
                .content(Padding::new(
                    EdgeInsets::symmetric(12., 0.),
                    Row::new(row).cross_axis_alignment(CrossAxisAlignment::Center),
                ))
                .on_press(move || run(action)),
            if is_selected {
                control()
            } else {
                Color::TRANSPARENT
            },
            8.,
        ));
    }
    if matches.is_empty() {
        rows.push(super::shared::empty_state(
            IconName::Search,
            "No matching commands",
            "Try another word: view names, overlay names, “record”, “theme”…",
        ));
    }
    // Keep the selected row inside the list viewport.
    if let Some(top) = offsets.get(selected).copied() {
        let viewport = state.scroll.viewport_extent();
        let offset = state.scroll.offset();
        if viewport > 0. && (top < offset || top + ROW_HEIGHT > offset + viewport) {
            state
                .scroll
                .deferred_jump_to((top - viewport + ROW_HEIGHT + 8.).max(0.).min(top));
        }
    }

    let field: Widget = Row::new([
        glyph(IconName::Search, 16., text_muted()),
        gap(12., 1.),
        Expanded::new(if query.is_empty() {
            hrow([
                Widget::box_(Size::new(1.5, 18.), primary()),
                gap(4., 1.),
                Text::new("Type a command or search…")
                    .style(text_style(14., text_muted()))
                    .into(),
            ])
        } else {
            hrow([
                Text::new(query.clone())
                    .style(text_style(14., text_primary()))
                    .max_lines(Some(1))
                    .into(),
                gap(1., 1.),
                Widget::box_(Size::new(1.5, 18.), primary()),
            ])
        })
        .into(),
        kbd("Esc"),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .into();

    let footer: Widget = Row::new([
        kbd("↑"),
        gap(2., 1.),
        kbd("↓"),
        gap(6., 1.),
        super::shared::ui_text("navigate", 11., text_muted()),
        gap(14., 1.),
        kbd("Enter"),
        gap(6., 1.),
        super::shared::ui_text("run", 11., text_muted()),
        gap(14., 1.),
        kbd("Esc"),
        gap(6., 1.),
        super::shared::ui_text("close", 11., text_muted()),
        spacer(),
        super::shared::ui_text(
            format!(
                "{} command{}",
                matches.len(),
                if matches.len() == 1 { "" } else { "s" }
            ),
            11.,
            text_muted(),
        ),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .into();

    let panel: Widget = Column::new([
        SizedBox::new()
            .height(54.)
            .child(Padding::new(EdgeInsets::symmetric(18., 0.), field))
            .into(),
        divider(),
        SizedBox::new()
            .height(LIST_HEIGHT)
            .child(ScrollView::vertical(
                state.scroll.clone(),
                Padding::new(
                    EdgeInsets::symmetric(8., 6.),
                    Column::new(rows)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .main_axis_size(MainAxisSize::Min),
                ),
            ))
            .into(),
        divider(),
        SizedBox::new()
            .height(40.)
            .child(Padding::new(EdgeInsets::symmetric(16., 0.), footer))
            .into(),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .main_axis_size(MainAxisSize::Min)
    .into();

    let keyboard_state = state.clone();
    let count = matches.len();
    let actions = matches.iter().map(|entry| entry.action).collect::<Vec<_>>();
    KeyboardListener::new(panel)
        .focus_node(state.focus.clone())
        .autofocus(true)
        .on_key(move |event| {
            if !event.state.is_down() {
                return false;
            }
            let ctrl = event
                .modifiers
                .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::META);
            match event.code {
                Code::Escape => {
                    keyboard_state.hide();
                    return true;
                }
                Code::ArrowDown | Code::ArrowUp if count > 0 => {
                    let mut selected = keyboard_state.selected.borrow_mut();
                    *selected = if event.code == Code::ArrowDown {
                        (*selected + 1) % count
                    } else {
                        (*selected + count - 1) % count
                    };
                    drop(selected);
                    super::shared::refresh(&tick);
                    return true;
                }
                Code::PageDown | Code::PageUp if count > 0 && !ctrl => {
                    let mut selected = keyboard_state.selected.borrow_mut();
                    *selected = if event.code == Code::PageDown {
                        (*selected + 8).min(count - 1)
                    } else {
                        selected.saturating_sub(8)
                    };
                    drop(selected);
                    super::shared::refresh(&tick);
                    return true;
                }
                Code::Enter | Code::NumpadEnter => {
                    let index = *keyboard_state.selected.borrow();
                    if let Some(action) = actions.get(index).copied() {
                        run(action);
                    }
                    return true;
                }
                Code::Backspace => {
                    let mut query = keyboard_state.query.borrow_mut();
                    if event.modifiers.contains(Modifiers::CONTROL) {
                        query.clear();
                    } else {
                        query.pop();
                    }
                    drop(query);
                    *keyboard_state.selected.borrow_mut() = 0;
                    keyboard_state.scroll.deferred_jump_to(0.);
                    super::shared::refresh(&tick);
                    return true;
                }
                _ => {}
            }
            if ctrl {
                return false;
            }
            if let incular::core::KeyboardKey::Character(text) = &event.key
                && !text.chars().any(char::is_control)
            {
                let mut query = keyboard_state.query.borrow_mut();
                if query.chars().count() < 80 {
                    query.push_str(text);
                }
                drop(query);
                *keyboard_state.selected.borrow_mut() = 0;
                keyboard_state.scroll.deferred_jump_to(0.);
                super::shared::refresh(&tick);
                return true;
            }
            false
        })
        .into()
}

/// The keyboard shortcut reference dialog.
pub(crate) fn shortcuts_dialog(close: Rc<dyn Fn()>) -> Widget {
    let mut groups: Vec<Widget> = Vec::new();
    for (title, entries) in super::shell::SHORTCUT_REFERENCE {
        let mut rows = vec![caption(title), gap(1., 8.)];
        for (keys, action) in entries {
            rows.push(
                SizedBox::new()
                    .height(32.)
                    .child(
                        Row::new([
                            Expanded::new(line_text(*action, 12.5, text_primary())).into(),
                            gap(12., 1.),
                            shortcut_keys(keys),
                        ])
                        .cross_axis_alignment(CrossAxisAlignment::Center),
                    )
                    .into(),
            );
        }
        groups.push(super::shared::column(rows));
    }
    let mut groups = groups.into_iter();
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (index, group) in groups.by_ref().enumerate() {
        let target = if index % 2 == 0 {
            &mut left
        } else {
            &mut right
        };
        if !target.is_empty() {
            target.push(gap(1., 18.));
        }
        target.push(group);
    }
    Column::new([
        Padding::new(
            EdgeInsets::only(22., 18., 14., 14.),
            Row::new([
                glyph(IconName::Keyboard, 18., primary()),
                gap(10., 1.),
                Expanded::new(super::shared::column([
                    super::shared::heading("Keyboard shortcuts", 15.),
                    gap(1., 2.),
                    super::shared::ui_text(
                        "Every shortcut needs Ctrl, so typing in a field never triggers one.",
                        11.5,
                        text_muted(),
                    ),
                ]))
                .into(),
                super::shared::icon_button(IconName::Close, "Close shortcuts", false, move || {
                    close()
                }),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Center),
        )
        .into(),
        divider(),
        Padding::all(
            22.,
            Row::new([
                Expanded::new(super::shared::column(left)).into(),
                gap(32., 1.),
                Expanded::new(super::shared::column(right)).into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .into(),
    ])
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .main_axis_size(MainAxisSize::Min)
    .into()
}
