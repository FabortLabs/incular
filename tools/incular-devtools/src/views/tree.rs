use super::shared::{WARNING, column, input_style, quiet_button, refresh, text_style};
use super::{BORDER, CONTROL, CONTROL_ACTIVE, PRIMARY, TEXT_MUTED, TEXT_PRIMARY, gap, ui_text};
use crate::{
    inspector::{Shared, TreeNavigation},
    transport::ClientBridge,
};
use incular::widgets::internal::{TextEditingController, icons};
use incular::{controls::TextField, material::RawMaterialButton, prelude::*};
use incular_devtools_protocol::{DevWidgetId, RequestMethod};

pub(crate) const ROW_HEIGHT: f32 = 28.;

pub(crate) struct TreeWidgets {
    pub search: Widget,
    pub list: Widget,
    pub controls: Widget,
    pub breadcrumbs: Widget,
    pub status: String,
}

pub(crate) fn select(shared: &Shared, bridge: &ClientBridge, tick: &Signal<u64>, id: DevWidgetId) {
    let window = shared.lock().ok().and_then(|mut state| {
        state
            .select_node(id)
            .then_some(state.active_window)
            .flatten()
    });
    if let Some(window) = window {
        bridge.send_or_report(RequestMethod::GetNodeDetails { id });
        bridge.send_or_report(RequestMethod::HighlightNode {
            window,
            id: Some(id),
        });
    }
    refresh(tick);
}

pub(crate) fn inspect_mode(
    shared: &Shared,
    bridge: &ClientBridge,
    tick: &Signal<u64>,
    enabled: Option<bool>,
) -> bool {
    let request = shared.lock().ok().and_then(|mut state| {
        let window = state.active_window?;
        state.select_mode = enabled.unwrap_or(!state.select_mode);
        Some(if state.select_mode {
            RequestMethod::StartInspectMode { window }
        } else {
            RequestMethod::StopInspectMode { window }
        })
    });
    if let Some(request) = request {
        bridge.send_or_report(request);
        refresh(tick);
        true
    } else {
        false
    }
}

pub(crate) fn build(
    shared: Shared,
    bridge: ClientBridge,
    tick: Signal<u64>,
    search: TextEditingController,
    scroll: ScrollController,
    focus: (FocusNode, FocusNode),
) -> TreeWidgets {
    let (focus, workspace_focus) = focus;
    let (row_count, total, query, selected, focused_root, path, truncated) = {
        let state = shared.lock().expect("inspector state");
        (
            state.rows.len(),
            state.nodes.len(),
            state.search.clone(),
            state.selected,
            state.focused_root,
            state
                .selected
                .map(|id| state.ancestor_path(id))
                .unwrap_or_default(),
            state.tree_truncated,
        )
    };
    let search_field: Widget = TextField::new(search)
        .size(Size::new(0., 18.))
        .style(input_style())
        .placeholder("Search type, text, key or id…")
        .on_submit({
            let shared = shared.clone();
            let bridge = bridge.clone();
            let tick = tick.clone();
            let focus = focus.clone();
            let workspace_focus = workspace_focus.clone();
            move |_| {
                let id = shared
                    .lock()
                    .ok()
                    .and_then(|mut state| state.navigate_tree(TreeNavigation::First));
                if let Some(id) = id {
                    workspace_focus.unfocus();
                    focus.request_focus();
                    select(&shared, &bridge, &tick, id);
                }
            }
        })
        .on_changed({
            let shared = shared.clone();
            let tick = tick.clone();
            let scroll = scroll.clone();
            move |query| {
                if let Ok(mut state) = shared.lock() {
                    state.set_search(query);
                }
                scroll.deferred_jump_to(0.);
                refresh(&tick);
            }
        })
        .into();
    let mut controls = Vec::new();
    for (label, expand) in [("Expand tree", true), ("Collapse tree", false)] {
        let shared = shared.clone();
        let tick = tick.clone();
        controls.push(quiet_button(label, false, true, move || {
            if let Ok(mut state) = shared.lock() {
                if let Some(root) = state.focused_root {
                    state.expand_branch(root, expand);
                } else if expand {
                    state.expand_all();
                } else {
                    state.collapse_all();
                }
            }
            refresh(&tick);
        }));
    }
    {
        let shared = shared.clone();
        let tick = tick.clone();
        let scroll = scroll.clone();
        controls.push(quiet_button(
            if focused_root.is_some() {
                "Whole tree"
            } else {
                "Focus subtree"
            },
            focused_root.is_some(),
            selected.is_some() || focused_root.is_some(),
            move || {
                if let Ok(mut state) = shared.lock() {
                    state.focus_subtree(if focused_root.is_some() {
                        None
                    } else {
                        selected
                    });
                }
                scroll.deferred_jump_to(0.);
                refresh(&tick);
            },
        ));
    }
    if !query.trim().is_empty() {
        for (label, direction) in [
            ("Previous match", TreeNavigation::Previous),
            ("Next match", TreeNavigation::Next),
        ] {
            let shared = shared.clone();
            let bridge = bridge.clone();
            let tick = tick.clone();
            controls.push(quiet_button(label, false, row_count > 0, move || {
                let id = shared
                    .lock()
                    .ok()
                    .and_then(|mut state| state.navigate_tree(direction));
                if let Some(id) = id {
                    select(&shared, &bridge, &tick, id);
                }
            }));
        }
    }
    let mut breadcrumbs = vec![ui_text("SELECTION", 9., TEXT_MUTED), gap(6., 1.)];
    let visible_path = if path.len() > 5 {
        let mut short = vec![path[0]];
        short.extend_from_slice(&path[path.len() - 4..]);
        short
    } else {
        path.clone()
    };
    for (index, id) in visible_path.iter().copied().enumerate() {
        if index > 0 {
            breadcrumbs.push(ui_text(
                if index == 1 && path.len() > 5 {
                    " / … / "
                } else {
                    "/"
                },
                12.,
                TEXT_MUTED,
            ));
        }
        let label = shared
            .lock()
            .ok()
            .and_then(|state| state.nodes.get(&id).map(|node| node.type_name.clone()))
            .unwrap_or_default();
        let shared = shared.clone();
        let bridge = bridge.clone();
        let tick = tick.clone();
        breadcrumbs.push(quiet_button(label, selected == Some(id), true, move || {
            select(&shared, &bridge, &tick, id);
        }));
    }
    let breadcrumbs = Wrap::new(breadcrumbs).spacing(4.).run_spacing(4.).into();
    let list_shared = shared.clone();
    let list_bridge = bridge.clone();
    let list_tick = tick.clone();
    let list_focus = focus.clone();
    let list_workspace_focus = workspace_focus.clone();
    let list: Widget = if row_count == 0 {
        Padding::all(
            16.,
            column([
                ui_text(
                    if total == 0 {
                        "Waiting for the widget tree"
                    } else {
                        "No widgets match this search"
                    },
                    13.,
                    TEXT_PRIMARY,
                ),
                gap(1., 8.),
                ui_text("Try type:Text, key:save, or a label.", 11., TEXT_MUTED),
            ]),
        )
        .into()
    } else {
        CustomScrollView::new(vec![Box::new(SliverFixedExtentList::new(
            row_count,
            ROW_HEIGHT,
            move |index| {
                let snapshot = list_shared.lock().ok().and_then(|state| {
                    let row = state.rows.get(index)?.clone();
                    let node = state.nodes.get(&row.id)?;
                    let label = format!(
                        "{}{}{}",
                        node.type_name,
                        node.key
                            .as_ref()
                            .map(|key| format!("  #{key}"))
                            .unwrap_or_default(),
                        node.label
                            .as_ref()
                            .map(|label| format!("  \"{}\"", label.replace(['\n', '\r'], " ")))
                            .unwrap_or_default()
                    );
                    Some((
                        row,
                        label,
                        node.type_name.clone(),
                        node.key.clone(),
                        node.label.clone(),
                        node.child_ids.len(),
                        state.selected,
                        state.hovered,
                        state.expanded.contains(&node.id),
                        !node.child_ids.is_empty(),
                        state.search.trim().is_empty(),
                    ))
                });
                let Some((
                    row,
                    label,
                    kind,
                    key,
                    text,
                    children,
                    selected,
                    hovered,
                    expanded,
                    has_children,
                    is_tree,
                )) = snapshot
                else {
                    return gap(1., ROW_HEIGHT);
                };
                let id = row.id;
                let disclosure: Widget = if has_children && is_tree {
                    let icon: Widget = Icon::new(icons::chevron_right())
                        .size(11.)
                        .brush(TEXT_MUTED)
                        .into();
                    let icon: Widget = if expanded {
                        Transform::rotation(std::f32::consts::FRAC_PI_2, icon).into()
                    } else {
                        icon
                    };
                    let shared = list_shared.clone();
                    let tick = list_tick.clone();
                    RawMaterialButton::new(if expanded {
                        "Collapse branch"
                    } else {
                        "Expand branch"
                    })
                    .size(Size::new(20., ROW_HEIGHT))
                    .color(Color::TRANSPARENT)
                    .content(Align::new(Alignment::CENTER, icon))
                    .on_press(move || {
                        if let Ok(mut state) = shared.lock() {
                            state.toggle_expanded(id);
                        }
                        refresh(&tick);
                    })
                    .into()
                } else {
                    gap(20., ROW_HEIGHT)
                };
                let selected_row = selected == Some(id);
                let shared = list_shared.clone();
                let bridge = list_bridge.clone();
                let tick = list_tick.clone();
                let focus = list_focus.clone();
                let workspace_focus = list_workspace_focus.clone();
                let hover_shared = list_shared.clone();
                let hover_bridge = list_bridge.clone();
                let hover_tick = list_tick.clone();
                let exit_shared = list_shared.clone();
                let exit_bridge = list_bridge.clone();
                let exit_tick = list_tick.clone();
                let kind = if row.depth > 16 {
                    format!("…{}  {kind}", row.depth)
                } else {
                    kind
                };
                let label_content = LayoutBuilder::new(move |_, constraints| {
                    let type_color = if selected_row {
                        TEXT_PRIMARY
                    } else {
                        Color::rgba(177, 194, 237, 255)
                    };
                    let text_widget = |value: String, color| {
                        Text::new(value)
                            .style(text_style(12., color).font_family("Consolas"))
                            .max_lines(Some(1))
                            .overflow(TextOverflow::Ellipsis)
                    };
                    if key.is_none() && text.is_none() {
                        return text_widget(kind.clone(), type_color).into();
                    }
                    let mut parts: Vec<Widget> = vec![
                        ConstrainedBox::new(
                            Constraints::loose(Size::new(
                                (constraints.max_width() * 0.55).min(240.),
                                ROW_HEIGHT,
                            )),
                            text_widget(kind.clone(), type_color),
                        )
                        .into(),
                    ];
                    if let Some(key) = &key {
                        parts.push(gap(8., 1.));
                        parts.push(
                            ConstrainedBox::new(
                                Constraints::loose(Size::new(
                                    (constraints.max_width() * 0.3).min(160.),
                                    ROW_HEIGHT,
                                )),
                                text_widget(format!("#{key}"), PRIMARY),
                            )
                            .into(),
                        );
                    }
                    if let Some(text) = &text {
                        parts.push(gap(8., 1.));
                        parts.push(
                            Expanded::new(text_widget(
                                format!("\"{}\"", text.replace(['\n', '\r'], " ")),
                                WARNING,
                            ))
                            .into(),
                        );
                    }
                    Row::new(parts).into()
                });
                let content: Widget = Align::new(
                    Alignment::CENTER_LEFT,
                    Padding::new(
                        EdgeInsets::symmetric(5., 0.),
                        Row::new([
                            Expanded::new(label_content).into(),
                            gap(8., 1.),
                            ui_text(
                                if children > 0 {
                                    children.to_string()
                                } else {
                                    String::new()
                                },
                                10.,
                                TEXT_MUTED,
                            ),
                            gap(10., 1.),
                        ]),
                    ),
                )
                .into();
                let button: Widget = RawMaterialButton::new(format!("Inspect {label}"))
                    .size(Size::new(0., ROW_HEIGHT))
                    .content(content)
                    .color(Color::TRANSPARENT)
                    .hover_color(CONTROL)
                    .focused_color(CONTROL_ACTIVE)
                    .on_press(move || {
                        workspace_focus.unfocus();
                        focus.request_focus();
                        select(&shared, &bridge, &tick, id);
                    })
                    .on_hover(move || {
                        let window = hover_shared.lock().ok().and_then(|mut state| {
                            state.hovered = Some(id);
                            state.active_window
                        });
                        if let Some(window) = window {
                            hover_bridge.send_or_report(RequestMethod::HighlightNode {
                                window,
                                id: Some(id),
                            });
                        }
                        refresh(&hover_tick);
                    })
                    .on_exit(move || {
                        let selection = exit_shared.lock().ok().map(|mut state| {
                            state.hovered = None;
                            (state.active_window, state.selected)
                        });
                        if let Some((Some(window), id)) = selection {
                            exit_bridge.send_or_report(RequestMethod::HighlightNode { window, id });
                        }
                        refresh(&exit_tick);
                    })
                    .into();
                let indent = f32::from(row.depth.min(16)) * 14.;
                let mut guides = Canvas::default();
                for depth in 0..row.depth.min(16) {
                    guides.rect(
                        incular::core::Rect::from_origin_size(
                            Offset::new(f32::from(depth) * 14. + 9., 0.),
                            Size::new(1., ROW_HEIGHT),
                        ),
                        BORDER,
                    );
                }
                Container::with_child(Row::new([
                    Widget::box_(
                        Size::new(3., ROW_HEIGHT),
                        if selected_row {
                            PRIMARY
                        } else {
                            Color::TRANSPARENT
                        },
                    ),
                    gap(8., 1.),
                    CustomPaint::new(Size::new(indent, ROW_HEIGHT), guides.finish()).into(),
                    disclosure,
                    Expanded::new(button).into(),
                ]))
                .color(if selected_row {
                    CONTROL_ACTIVE
                } else if hovered == Some(id) {
                    CONTROL
                } else {
                    Color::TRANSPARENT
                })
                .into()
            },
        )) as Box<dyn Sliver>])
        .controller(scroll)
        .into()
    };
    let keyboard_shared = shared;
    let keyboard_bridge = bridge;
    let keyboard_tick = tick;
    let list = KeyboardListener::new(list)
        .focus_node(focus)
        .on_key(move |event| {
            if !event.state.is_down()
                || event
                    .modifiers
                    .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::META)
            {
                return false;
            }
            let direction = match event.code {
                Code::ArrowUp => TreeNavigation::Previous,
                Code::ArrowDown => TreeNavigation::Next,
                Code::ArrowLeft => TreeNavigation::CollapseOrParent,
                Code::ArrowRight => TreeNavigation::ExpandOrChild,
                Code::Home => TreeNavigation::First,
                Code::End => TreeNavigation::Last,
                _ => return false,
            };
            let id = keyboard_shared
                .lock()
                .ok()
                .and_then(|mut state| state.navigate_tree(direction));
            if let Some(id) = id {
                select(&keyboard_shared, &keyboard_bridge, &keyboard_tick, id);
            } else {
                refresh(&keyboard_tick);
            }
            true
        })
        .into();
    TreeWidgets {
        search: SizedBox::new().height(30.).child(search_field).into(),
        list,
        controls: Wrap::new(controls).spacing(4.).run_spacing(4.).into(),
        breadcrumbs,
        status: format!(
            "{row_count} {} · {total} loaded{}",
            if query.trim().is_empty() {
                "visible"
            } else {
                "matches"
            },
            if truncated { " · truncated" } else { "" }
        ),
    }
}
