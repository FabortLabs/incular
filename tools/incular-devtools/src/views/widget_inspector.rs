use super::shared::{
    column, heading, input_style, key_value, mono, quiet_button, refresh, text_style, workspace_tab,
};
use super::{DANGER, PRIMARY, SUCCESS, TEXT_MUTED, TEXT_PRIMARY, compact_button, gap, ui_text};
use crate::{
    inspector::{InspectorSection, Shared, debug_value, editable_value, parse_debug_value},
    transport::ClientBridge,
};
use incular::controls::TextField;
use incular::material::RawMaterialButton;
use incular::prelude::*;
use incular::widgets::internal::{TextEditingController, icons};
use incular_devtools_protocol::{
    DebugOption, DevWidgetId, DevWindowId, NodeDetails, RequestMethod, SignalSubscriber,
    SignalSummary, WindowSummary,
};
use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};

pub(crate) struct InspectorData {
    pub(crate) windows: Vec<WindowSummary>,
    pub(crate) active_window: Option<DevWindowId>,
    pub(crate) select_mode: bool,
    pub(crate) debug_options: HashSet<DebugOption>,
    pub(crate) animation_scale: f32,
    pub(crate) details: Vec<String>,
    pub(crate) selected_details: Option<NodeDetails>,
    pub(crate) has_selection: bool,
    pub(crate) signals: Vec<SignalSummary>,
    pub(crate) selected_signal: Option<SignalSummary>,
    pub(crate) signal_subscribers: Vec<SignalSubscriber>,
}

pub(crate) struct InspectorWidgets {
    pub(crate) content: Widget,
    pub(crate) search_field: Widget,
    pub(crate) tree_list: Widget,
    pub(crate) toolbar: Widget,
    pub(crate) header: Widget,
    pub(crate) tabs: Widget,
    pub(crate) breadcrumbs: Widget,
    pub(crate) tree_controls: Widget,
    pub(crate) tree_status: String,
}

pub(crate) struct InspectorBuildContext {
    pub(crate) shared: Shared,
    pub(crate) bridge: ClientBridge,
    pub(crate) tick: Signal<u64>,
    pub(crate) search: TextEditingController,
    pub(crate) signal_value: TextEditingController,
    pub(crate) property_value: TextEditingController,
    pub(crate) property_binding: Rc<RefCell<Option<(DevWidgetId, String)>>>,
    pub(crate) inspector_section: Signal<InspectorSection>,
    pub(crate) visual_tools_open: Signal<bool>,
    pub(crate) tree_scroll: ScrollController,
    pub(crate) tree_focus: FocusNode,
    pub(crate) workspace_focus: FocusNode,
    pub(crate) property_filter: TextEditingController,
    pub(crate) collapsed_groups: Signal<HashSet<&'static str>>,
}

pub(crate) fn build_inspector(
    data: InspectorData,
    context: InspectorBuildContext,
) -> InspectorWidgets {
    let InspectorBuildContext {
        shared,
        bridge,
        tick,
        search,
        signal_value,
        property_value,
        property_binding,
        inspector_section,
        visual_tools_open,
        tree_scroll,
        tree_focus,
        workspace_focus,
        property_filter,
        collapsed_groups,
    } = context;
    let InspectorData {
        windows,
        active_window,
        select_mode,
        debug_options,
        animation_scale,
        details,
        selected_details,
        has_selection,
        signals,
        selected_signal,
        signal_subscribers,
    } = data;
    let mut window_controls = Vec::new();
    let mut selection_controls = Vec::new();
    let mut debug_controls = Vec::new();
    let mut animation_controls = Vec::new();
    let mut signal_body = Vec::new();

    for window in windows {
        let window_bridge = bridge.clone();
        let window_shared = Arc::clone(&shared);
        let window_tick = tick.clone();
        window_controls.push(quiet_button(
            format!(
                "{} · {:.0}×{:.0}",
                window.title, window.logical_size[0], window.logical_size[1]
            ),
            active_window == Some(window.id),
            true,
            move || {
                if let Ok(mut state) = window_shared.lock() {
                    if state.active_window == Some(window.id) {
                        return;
                    }
                    state.active_window = Some(window.id);
                    state.nodes.clear();
                    state.rows.clear();
                    state.selected = None;
                    state.focused_root = None;
                    state.selection_history.clear();
                    state.history_cursor = None;
                    state.hovered = None;
                    state.details = None;
                    state.selected_frame = None;
                    state.selected_range = None;
                    state.range_anchor = None;
                    state.timeline_offset = 0;
                    state.tree_retry_sent = false;
                }
                window_bridge.send_or_report(RequestMethod::GetWidgetTree { window: window.id });
                window_tick.update(|value| {
                    *value = value
                        .checked_add(1)
                        .expect("DevTools UI revision exhausted")
                });
            },
        ));
    }
    let selection_bridge = bridge.clone();
    let selection_shared = Arc::clone(&shared);
    let selection_tick = tick.clone();
    let picker = RawMaterialButton::new(if select_mode {
        "Cancel picking"
    } else {
        "Pick widget"
    })
    .size(Size::new(0., 28.))
    .padding(EdgeInsets::symmetric(10., 5.))
    .label_style(text_style(12., super::APP_BACKGROUND))
    .color(if select_mode { DANGER } else { PRIMARY })
    .enabled(active_window.is_some())
    .on_press(move || {
        super::tree::inspect_mode(&selection_shared, &selection_bridge, &selection_tick, None);
    })
    .into();
    for (label, forward) in [("← Back", false), ("Forward →", true)] {
        let shared = shared.clone();
        let bridge = bridge.clone();
        let tick = tick.clone();
        let enabled = shared.lock().is_ok_and(|state| {
            state.history_cursor.is_some_and(|cursor| {
                if forward {
                    state
                        .selection_history
                        .iter()
                        .skip(cursor + 1)
                        .any(|id| state.nodes.contains_key(id))
                } else {
                    state
                        .selection_history
                        .iter()
                        .take(cursor)
                        .any(|id| state.nodes.contains_key(id))
                }
            })
        });
        selection_controls.push(quiet_button(label, false, enabled, move || {
            let id = shared
                .lock()
                .ok()
                .and_then(|mut state| state.selection_history_step(forward));
            if let Some(id) = id {
                super::tree::select(&shared, &bridge, &tick, id);
            }
        }));
    }
    {
        let shared = shared.clone();
        let bridge = bridge.clone();
        let tick = tick.clone();
        selection_controls.push(quiet_button(
            "Reveal selection",
            false,
            has_selection,
            move || {
                let id = shared.lock().ok().and_then(|mut state| {
                    let selected = state.selected?;
                    state.focused_root = None;
                    state.set_search("");
                    Some(selected)
                });
                if let Some(id) = id {
                    super::tree::select(&shared, &bridge, &tick, id);
                }
            },
        ));
    }
    let clear_bridge = bridge.clone();
    let clear_shared = Arc::clone(&shared);
    let clear_tick = tick.clone();
    selection_controls.push(quiet_button(
        "Clear selection",
        false,
        has_selection,
        move || {
            let window = clear_shared.lock().ok().and_then(|mut state| {
                state.selected = None;
                state.hovered = None;
                state.details = None;
                state.active_window
            });
            if let Some(window) = window {
                clear_bridge.send_or_report(RequestMethod::HighlightNode { window, id: None });
            }
            clear_tick.update(|value| {
                *value = value
                    .checked_add(1)
                    .expect("DevTools UI revision exhausted")
            });
        },
    ));
    for (option, label) in [
        (DebugOption::LayoutBounds, "Bounds: selected"),
        (DebugOption::LayoutBoundsSubtree, "Bounds: subtree"),
        (DebugOption::LayoutBoundsWholeWindow, "Bounds: whole window"),
        (DebugOption::PaddingContent, "Padding/content"),
        (DebugOption::Baselines, "Baselines"),
        (DebugOption::Clips, "Clips"),
        (DebugOption::HitTestRegions, "Hit test"),
        (DebugOption::SemanticsBounds, "Semantics bounds"),
        (DebugOption::ScrollViewports, "Scroll viewports"),
        (DebugOption::LayerBoundaries, "Layer boundaries"),
        (DebugOption::HighlightBuild, "BUILD flash"),
        (DebugOption::HighlightLayout, "LAYOUT flash"),
        (DebugOption::HighlightPaint, "PAINT flash"),
        (DebugOption::HighlightSemantics, "SEMANTICS flash"),
        (DebugOption::HighlightComposite, "COMPOSITE flash"),
        (DebugOption::RepaintRainbow, "Repaint rainbow"),
    ] {
        let option_bridge = bridge.clone();
        let option_shared = Arc::clone(&shared);
        let option_tick = tick.clone();
        let enabled = debug_options.contains(&option);
        debug_controls.push(
            RawMaterialButton::new(label)
                .size(Size::new(0., 30.))
                .content(Align::new(
                    Alignment::CENTER_LEFT,
                    Row::new([
                        Widget::from(
                            Container::new()
                                .width(10.)
                                .height(10.)
                                .color(if enabled { PRIMARY } else { Color::TRANSPARENT })
                                .border(Border::new(
                                    1.,
                                    if enabled { PRIMARY } else { super::TEXT_MUTED },
                                ))
                                .radius(2.),
                        ),
                        gap(10., 1.),
                        ui_text(label, 12., if enabled { TEXT_PRIMARY } else { TEXT_MUTED }),
                        Expanded::new(gap(1., 1.)).into(),
                        ui_text(
                            if enabled { "ON" } else { "OFF" },
                            9.,
                            if enabled { PRIMARY } else { TEXT_MUTED },
                        ),
                        gap(8., 1.),
                    ]),
                ))
                .color(Color::TRANSPARENT)
                .hover_color(super::CONTROL)
                .on_press(move || {
                    let enabled = if let Ok(mut state) = option_shared.lock() {
                        if !state.debug_options.insert(option) {
                            state.debug_options.remove(&option);
                            false
                        } else {
                            true
                        }
                    } else {
                        false
                    };
                    option_bridge.send_or_report(RequestMethod::SetDebugOption {
                        name: option,
                        enabled,
                    });
                    option_tick.update(|value| {
                        *value = value
                            .checked_add(1)
                            .expect("DevTools UI revision exhausted")
                    });
                })
                .into(),
        );
    }
    for (scale, label) in [
        (1., "Animations 1×"),
        (0.5, "Animations 0.5×"),
        (0.25, "Animations 0.25×"),
        (0.1, "Animations 0.1×"),
        (0., "Pause animations"),
    ] {
        let animation_bridge = bridge.clone();
        animation_controls.push(quiet_button(
            label,
            scale == animation_scale,
            true,
            move || animation_bridge.send_or_report(RequestMethod::SetAnimationSpeed { scale }),
        ));
    }

    let signals_bridge = bridge.clone();
    signal_body.push(compact_button("Refresh signal list", false, move || {
        signals_bridge.send_or_report(RequestMethod::ListSignals)
    }));
    for signal in signals {
        let label = format!(
            "{} · {} · generation {} · writes {} · subscribers {}{}",
            signal.name.as_deref().unwrap_or("<unnamed>"),
            signal.type_name,
            signal.generation,
            signal.write_count,
            signal.subscriber_count,
            signal
                .last_write_summary
                .as_ref()
                .map(|value| format!(" · last {value}"))
                .unwrap_or_default(),
        );
        let signal_bridge = bridge.clone();
        let signal_shared = Arc::clone(&shared);
        let signal_tick = tick.clone();
        signal_body.push(compact_button(
            label,
            selected_signal
                .as_ref()
                .is_some_and(|item| item.id == signal.id),
            move || {
                if let Ok(mut state) = signal_shared.lock() {
                    state.selected_signal = Some(signal.id);
                    state.signal_subscribers.clear();
                }
                signal_bridge.send_or_report(RequestMethod::GetSignalSubscribers { id: signal.id });
                signal_tick.update(|value| {
                    *value = value
                        .checked_add(1)
                        .expect("DevTools UI revision exhausted")
                });
            },
        ));
    }
    if let Some(signal) = selected_signal {
        signal_body.push(ui_text(
            format!(
                "Signal {} subscribers:",
                signal.name.as_deref().unwrap_or("<unnamed>")
            ),
            14.,
            TEXT_PRIMARY,
        ));
        if signal_subscribers.is_empty() {
            signal_body.push(ui_text("No retained subscribers.", 13., TEXT_MUTED));
        } else {
            signal_body.extend(
                signal_subscribers
                    .into_iter()
                    .map(|subscriber| ui_text(subscriber.path, 13., TEXT_MUTED)),
            );
        }
        if signal.editable {
            let signal_bridge = bridge.clone();
            let signal_tick = tick.clone();
            signal_body.push(
                TextField::new(signal_value)
                    .placeholder("New signal value; press Enter")
                    .on_submit(move |input| {
                        if let Some(value) = editable_value(&signal, &input) {
                            signal_bridge.send_or_report(RequestMethod::EditSignal {
                                id: signal.id,
                                value,
                            });
                            signal_tick.update(|value| {
                                *value = value
                                    .checked_add(1)
                                    .expect("DevTools UI revision exhausted")
                            });
                        }
                    })
                    .into(),
            );
        }
    }
    if signal_body.len() == 1 {
        signal_body.push(ui_text(
            "No debug-enabled signals are registered by this target.",
            13.,
            TEXT_MUTED,
        ));
    }

    let tree = super::tree::build(
        Arc::clone(&shared),
        bridge.clone(),
        tick.clone(),
        search,
        tree_scroll,
        (tree_focus, workspace_focus),
    );
    let mut property_editor = Vec::new();
    if inspector_section.get() == InspectorSection::Properties
        && let Some(details) = selected_details.as_ref()
        && let Some(property) = details
            .properties
            .iter()
            .filter(|property| property.editable)
            .find(|property| {
                property_binding
                    .borrow()
                    .as_ref()
                    .is_some_and(|(id, name)| *id == details.id && *name == property.name)
            })
            .or_else(|| details.properties.iter().find(|property| property.editable))
    {
        let mut editable_properties = Vec::new();
        for editable in details
            .properties
            .iter()
            .filter(|property| property.editable)
        {
            let binding_state = property_binding.clone();
            let editor = property_value.clone();
            let editor_tick = tick.clone();
            let id = details.id;
            let name = editable.name.clone();
            let value = debug_value(&editable.value);
            editable_properties.push(compact_button(
                format!("Edit {}", editable.name),
                editable.name == property.name,
                move || {
                    *binding_state.borrow_mut() = Some((id, name.clone()));
                    editor.set_text(value.clone());
                    super::shared::refresh(&editor_tick);
                },
            ));
        }
        property_editor.push(
            Wrap::new(editable_properties)
                .spacing(6.)
                .run_spacing(6.)
                .into(),
        );
        property_editor.push(gap(1., 10.));
        property_editor.push(ui_text(
            format!(
                "Edit {}{}",
                property.name,
                if property.overridden {
                    " · DEV OVERRIDE"
                } else {
                    ""
                }
            ),
            13.,
            if property.overridden {
                SUCCESS
            } else {
                TEXT_PRIMARY
            },
        ));
        let property_bridge = bridge.clone();
        let property_tick = tick.clone();
        let id = details.id;
        let name = property.name.clone();
        let template = property.value.clone();
        property_editor.push(
            TextField::new(property_value)
                .style(input_style())
                .placeholder("Enter a value and press Enter")
                .on_submit(move |input| {
                    if let Some(value) = parse_debug_value(&template, &input) {
                        property_bridge.send_or_report(RequestMethod::EditProperty {
                            id,
                            name: name.clone(),
                            value,
                        });
                        property_tick.update(|value| {
                            *value = value
                                .checked_add(1)
                                .expect("DevTools UI revision exhausted")
                        });
                    }
                })
                .into(),
        );
        if property.overridden {
            let reset_bridge = bridge.clone();
            property_editor.push(compact_button(
                "Reset property overrides",
                false,
                move || reset_bridge.send_or_report(RequestMethod::ResetOverrides),
            ));
        }
        property_editor.push(gap(1., 10.));
    }

    let selected_node = shared
        .lock()
        .ok()
        .and_then(|state| state.selected.and_then(|id| state.nodes.get(&id).cloned()));
    let tools_open = visual_tools_open.get();
    let mut tabs = Vec::new();
    for (section, label) in [
        (InspectorSection::Properties, "Properties"),
        (InspectorSection::Layout, "Layout"),
        (InspectorSection::Signals, "Signals"),
        (InspectorSection::Why, "Why"),
        (InspectorSection::Semantics, "Semantics"),
    ] {
        let section_signal = inspector_section.clone();
        let tools = visual_tools_open.clone();
        tabs.push(workspace_tab(
            label,
            !tools_open && inspector_section.get() == section,
            true,
            move || {
                tools.set(false);
                section_signal.set(section);
            },
        ));
    }
    let tools = visual_tools_open.clone();
    tabs.push(workspace_tab("Overlays", tools_open, true, move || {
        tools.set(true);
    }));

    let details_content = if tools_open {
        let invalidations = debug_controls.split_off(10);
        column([
            ui_text(
                "Draw diagnostics in the target application.",
                12.,
                TEXT_MUTED,
            ),
            gap(1., 12.),
            property_group(
                "Geometry overlays",
                &collapsed_groups,
                column(debug_controls),
            ),
            gap(1., 12.),
            property_group("Work & repaint", &collapsed_groups, column(invalidations)),
            gap(1., 12.),
            property_group(
                "Animation speed",
                &collapsed_groups,
                Wrap::new(animation_controls)
                    .spacing(4.)
                    .run_spacing(4.)
                    .into(),
            ),
        ])
    } else if has_selection {
        if inspector_section.get() == InspectorSection::Properties {
            if let Some(details) = selected_details.as_ref() {
                let filter = property_filter.text().to_lowercase();
                let filter_tick = tick.clone();
                let mut properties = vec![
                    SizedBox::new()
                        .height(30.)
                        .child(
                            TextField::new(property_filter.clone())
                                .size(Size::new(0., 18.))
                                .style(input_style())
                                .placeholder("Filter properties…")
                                .on_changed(move |_| {
                                    refresh(&filter_tick);
                                }),
                        )
                        .into(),
                    gap(1., 8.),
                ];
                let mut matched = 0;
                for property in details.properties.iter().filter(|property| {
                    property.name.to_lowercase().contains(&filter)
                        || debug_value(&property.value)
                            .to_lowercase()
                            .contains(&filter)
                }) {
                    matched += 1;
                    properties.push(key_value(
                        format!(
                            "{}{}",
                            property.name,
                            if property.overridden { " *" } else { "" }
                        ),
                        debug_value(&property.value),
                    ));
                }
                if matched == 0 {
                    properties.push(ui_text("No properties match this filter.", 12., TEXT_MUTED));
                }
                if !property_editor.is_empty() {
                    properties.push(gap(1., 12.));
                    properties.push(column(property_editor));
                }
                let mut identity = vec![key_value("Widget id", details.id.to_string())];
                if let Some(key) = &details.key {
                    identity.push(key_value("Key", key.clone()));
                }
                if let Some(source) = &details.source {
                    identity.push(key_value(
                        "Source",
                        format!("{}:{}", source.file, source.line),
                    ));
                }
                identity.push(key_value(
                    "Local offset",
                    details.state.offset.map_or_else(
                        || "Unavailable".into(),
                        |offset| format!("{:.1}, {:.1}", offset[0], offset[1]),
                    ),
                ));
                let work: Vec<Widget> = [
                    ("BUILD", details.state.builds),
                    ("LAYOUT", details.state.layouts),
                    ("PAINT", details.state.paints),
                    ("COMPOSITE", details.state.composites),
                ]
                .into_iter()
                .map(|(label, count)| {
                    Expanded::new(column([
                        mono(count.to_string(), 20., TEXT_PRIMARY),
                        gap(1., 4.),
                        ui_text(label, 9., TEXT_MUTED),
                    ]))
                    .into()
                })
                .collect();
                column([
                    property_group("Widget properties", &collapsed_groups, column(properties)),
                    gap(1., 18.),
                    property_group("Identity & position", &collapsed_groups, column(identity)),
                    gap(1., 18.),
                    property_group(
                        "Retained work",
                        &collapsed_groups,
                        Padding::new(EdgeInsets::symmetric(0., 8.), Row::new(work)).into(),
                    ),
                ])
            } else if let Some(node) = selected_node.as_ref() {
                let mut identity = vec![
                    key_value("Widget id", node.id.to_string()),
                    key_value("Children", node.child_ids.len().to_string()),
                ];
                if let Some(key) = &node.key {
                    identity.push(key_value("Key", key.clone()));
                }
                if let Some(label) = &node.label {
                    identity.push(key_value("Label", label.clone()));
                }
                column([
                    property_group("Identity & position", &collapsed_groups, column(identity)),
                    gap(1., 18.),
                    ui_text(
                        "Detailed properties are not available yet.",
                        12.,
                        TEXT_MUTED,
                    ),
                ])
            } else {
                ui_text(
                    "The selected widget is no longer retained.",
                    12.,
                    TEXT_MUTED,
                )
            }
        } else if inspector_section.get() == InspectorSection::Layout {
            let mut body = Vec::new();
            if let Some(layout) = selected_details
                .as_ref()
                .and_then(|details| details.layout.as_ref())
            {
                body.push(super::geometry::box_model(layout));
                body.push(gap(1., 16.));
                let constraints = vec![
                    key_value(
                        "Incoming",
                        layout
                            .incoming_constraints
                            .as_ref()
                            .map_or_else(|| "Unavailable".into(), debug_value),
                    ),
                    key_value(
                        "Baseline",
                        layout
                            .baseline
                            .map_or_else(|| "—".into(), |value| format!("{value:.1}")),
                    ),
                    key_value(
                        "Clip",
                        layout.clip.map_or_else(
                            || "None".into(),
                            |rect| {
                                format!(
                                    "{:.1}, {:.1} · {:.1} × {:.1}",
                                    rect[0], rect[1], rect[2], rect[3]
                                )
                            },
                        ),
                    ),
                ];
                body.push(property_group(
                    "Constraints & clipping",
                    &collapsed_groups,
                    column(constraints),
                ));
                body.push(gap(1., 16.));
            }
            body.push(property_group(
                "Layout decisions",
                &collapsed_groups,
                column(details.into_iter().skip(2).map(|line| {
                    Padding::new(
                        EdgeInsets::symmetric(0., 4.),
                        ui_text(line, 11., TEXT_MUTED),
                    )
                    .into()
                })),
            ));
            column(body)
        } else {
            let title = match inspector_section.get() {
                InspectorSection::Signals => "Consumed signals",
                InspectorSection::Why => "Invalidation causes",
                InspectorSection::Semantics => "Accessibility semantics",
                _ => "Diagnostics",
            };
            let mut body = vec![property_group(
                title,
                &collapsed_groups,
                column(details.into_iter().skip(1).map(|line| {
                    Padding::new(
                        EdgeInsets::symmetric(0., 4.),
                        ui_text(line, 12., TEXT_MUTED),
                    )
                    .into()
                })),
            )];
            if inspector_section.get() == InspectorSection::Signals {
                body.push(gap(1., 16.));
                body.push(property_group(
                    "Signal registry",
                    &collapsed_groups,
                    column(signal_body),
                ));
            }
            column(body)
        }
    } else {
        Padding::new(
            EdgeInsets::symmetric(6., 36.),
            column([
                heading("Inspect your application", 20.),
                gap(1., 12.),
                ui_text(
                    "Pick a widget in the target window, or select one from the tree.",
                    13.,
                    TEXT_MUTED,
                ),
                gap(1., 24.),
                mono("Ctrl + Shift + C", 12., PRIMARY),
                gap(1., 6.),
                ui_text("Toggle the widget picker", 11., TEXT_MUTED),
                gap(1., 20.),
                mono("↑ ↓   ← →", 12., TEXT_PRIMARY),
                gap(1., 6.),
                ui_text("Move through the tree and expand branches", 11., TEXT_MUTED),
            ]),
        )
        .into()
    };
    let identity = selected_details
        .as_ref()
        .map(|details| {
            (
                details.type_name.clone(),
                details.id,
                details.key.clone(),
                details
                    .state
                    .size
                    .map(|size| format!("{:.0} × {:.0}", size[0], size[1])),
            )
        })
        .or_else(|| {
            selected_node
                .as_ref()
                .map(|node| (node.type_name.clone(), node.id, node.key.clone(), None))
        });
    let selected_header = if let Some((kind, id, key, size)) = identity {
        column([
            Row::new([
                Expanded::new(
                    Text::new(kind)
                        .style(text_style(17., TEXT_PRIMARY).font_weight(FontWeight::W600))
                        .max_lines(Some(1))
                        .overflow(TextOverflow::Ellipsis),
                )
                .into(),
                gap(12., 1.),
                mono(size.unwrap_or_else(|| "—".into()), 12., PRIMARY),
            ])
            .into(),
            gap(1., 5.),
            mono(
                format!(
                    "{id}{}",
                    key.map(|key| format!("   #{key}")).unwrap_or_default()
                ),
                10.,
                TEXT_MUTED,
            ),
            gap(1., 7.),
            Wrap::new(selection_controls)
                .spacing(2.)
                .run_spacing(2.)
                .into(),
        ])
    } else {
        column([
            heading("Inspector", 16.),
            gap(1., 5.),
            ui_text("Select a widget to see its details", 11., TEXT_MUTED),
        ])
    };
    let mut toolbar = vec![
        picker,
        gap(8., 1.),
        mono(
            if select_mode {
                "Esc to cancel"
            } else {
                "Ctrl+Shift+C"
            },
            10.,
            TEXT_MUTED,
        ),
        gap(16., 1.),
        ui_text("TARGET", 9., TEXT_MUTED),
        gap(4., 1.),
    ];
    toolbar.extend(window_controls);
    if select_mode {
        toolbar.push(gap(12., 1.));
        toolbar.push(ui_text("Hover to preview · Click to inspect", 11., PRIMARY));
    }
    InspectorWidgets {
        content: details_content,
        search_field: tree.search,
        tree_list: tree.list,
        toolbar: Wrap::new(toolbar).spacing(2.).run_spacing(4.).into(),
        header: selected_header,
        tabs: Wrap::new(tabs).into(),
        breadcrumbs: tree.breadcrumbs,
        tree_controls: tree.controls,
        tree_status: tree.status,
    }
}

fn property_group(
    title: &'static str,
    collapsed: &Signal<HashSet<&'static str>>,
    body: Widget,
) -> Widget {
    let closed = collapsed.get().contains(&title);
    let state = collapsed.clone();
    let chevron: Widget = Icon::new(icons::chevron_right())
        .size(9.)
        .brush(TEXT_MUTED)
        .into();
    let chevron = if closed {
        chevron
    } else {
        Transform::rotation(std::f32::consts::FRAC_PI_2, chevron).into()
    };
    let header: Widget = RawMaterialButton::new(title)
        .size(Size::new(0., 30.))
        .content(Align::new(
            Alignment::CENTER_LEFT,
            Row::new([
                chevron,
                gap(7., 1.),
                heading(title, 12.),
                Expanded::new(gap(1., 1.)).into(),
            ]),
        ))
        .color(Color::TRANSPARENT)
        .hover_color(super::CONTROL)
        .on_press(move || {
            state.update(|groups| {
                if !groups.insert(title) {
                    groups.remove(title);
                }
            });
        })
        .into();
    let mut children = vec![header];
    if !closed {
        children.push(
            LayoutBuilder::new(|_, constraints| {
                Widget::box_(Size::new(constraints.max_width(), 1.), super::BORDER)
            })
            .into(),
        );
        children.push(gap(1., 8.));
        children.push(body);
    }
    column(children)
}
