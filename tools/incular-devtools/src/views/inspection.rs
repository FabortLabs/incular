//! Semantics and layer tree views: searchable indented tree plus details pane.
//! Selecting a node highlights its bounds in the target window.

use super::shared::{
    badge, column, heading, input_style, key_value, mono, quiet_button, refresh, text_style,
    violet, warning,
};
use super::theme::accent_text;
use super::{
    background, border, control_active, gap, primary, surface, text_muted, text_primary, ui_text,
};
use crate::{
    diagnostics::{TreeNode, TreeView, VisibleRow, highlight_bounds, is_identity_transform},
    inspector::{InspectorModel, Shared},
    transport::ClientBridge,
};
use incular::widgets::internal::{SplitView, TextEditingController, icons};
use incular::{controls::TextField, material::RawMaterialButton, prelude::*};
use incular_devtools_protocol::{DevWindowId, RequestMethod};
use std::rc::Rc;

const ROW_HEIGHT: f32 = 28.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Semantics,
    Layers,
}

impl Kind {
    fn placeholder(self) -> &'static str {
        match self {
            Self::Semantics => "Search role, label, value or flag…",
            Self::Layers => "Search kind or summary…",
        }
    }

    fn empty_hint(self) -> &'static str {
        match self {
            Self::Semantics => "Select a semantics node to see its role, state and actions.",
            Self::Layers => "Select a layer to see its kind, summary and bounds.",
        }
    }
}

/// Requests a fresh snapshot of the tree behind this view.
pub(crate) fn request_refresh(bridge: &ClientBridge, kind: Kind, window: Option<DevWindowId>) {
    if let Some(window) = window {
        bridge.send_or_report(match kind {
            Kind::Semantics => RequestMethod::GetSemanticsTree { window },
            Kind::Layers => RequestMethod::GetLayerTree { window },
        });
    }
}

struct RowInfo {
    id: u64,
    depth: u16,
    has_children: bool,
    expanded: bool,
    matched: bool,
    title: String,
    detail: String,
    trailing: String,
    accent: Color,
    /// Window-space bounds previewed in the target while the row is hovered.
    rect: Option<[f32; 4]>,
}

#[derive(Clone)]
struct Details {
    title: String,
    subtitle: String,
    fields: Vec<(String, String)>,
    chips: Vec<(String, Vec<String>)>,
}

struct Prepared {
    rows: Vec<RowInfo>,
    details: Option<Details>,
    total: usize,
    truncated: bool,
    searching: bool,
    selected: Option<u64>,
    loaded: bool,
}

fn bounds_text(bounds: [f32; 4]) -> String {
    format!(
        "{:.0}, {:.0} · {:.0} × {:.0}",
        bounds[0], bounds[1], bounds[2], bounds[3]
    )
}

fn prepare<N: TreeNode>(
    view: &TreeView<N>,
    hide: &dyn Fn(&N) -> bool,
    row_info: impl Fn(&N, &VisibleRow, bool) -> RowInfo,
    details: impl Fn(&N, &TreeView<N>) -> Details,
) -> Prepared {
    let searching = view.searching();
    let rows = view
        .visible_rows_hiding(hide)
        .into_iter()
        .map(|row| {
            let node = &view.nodes()[row.index];
            let expanded = view.expanded.contains(&node.node_id());
            row_info(node, &row, expanded)
        })
        .collect();
    Prepared {
        rows,
        details: view.selected_node().map(|node| details(node, view)),
        total: view.len(),
        truncated: view.truncated,
        searching,
        selected: view.selected,
        loaded: view.loaded,
    }
}

fn prepare_for(kind: Kind, model: &InspectorModel) -> Prepared {
    let show_identity = model.show_identity_transforms;
    match kind {
        Kind::Semantics => prepare(
            &model.semantics,
            &|_| false,
            |node, row, expanded| RowInfo {
                id: node.id,
                depth: row.depth,
                has_children: row.has_children,
                expanded,
                matched: row.matched,
                title: node.role.clone(),
                detail: [node.label.as_deref(), node.value.as_deref()]
                    .into_iter()
                    .flatten()
                    .map(|text| format!("\"{}\"", text.replace(['\n', '\r'], " ")))
                    .collect::<Vec<_>>()
                    .join("  "),
                trailing: if node.children.is_empty() {
                    String::new()
                } else {
                    node.children.len().to_string()
                },
                accent: violet(),
                rect: highlight_bounds(node.bounds),
            },
            |node, _| {
                let mut fields = vec![
                    (
                        "Label".to_owned(),
                        node.label.clone().unwrap_or_else(|| "—".into()),
                    ),
                    (
                        "Value".to_owned(),
                        node.value.clone().unwrap_or_else(|| "—".into()),
                    ),
                    (
                        "Description".to_owned(),
                        node.description.clone().unwrap_or_else(|| "—".into()),
                    ),
                    ("Bounds".to_owned(), bounds_text(node.bounds)),
                    ("Children".to_owned(), node.children.len().to_string()),
                ];
                if let Some(parent) = node.parent {
                    fields.push(("Parent".to_owned(), format!("#{parent}")));
                }
                Details {
                    title: node.role.clone(),
                    subtitle: format!("semantics node #{}", node.id),
                    fields,
                    chips: vec![
                        ("STATE".to_owned(), node.flags.clone()),
                        ("ACTIONS".to_owned(), node.actions.clone()),
                    ],
                }
            },
        ),
        Kind::Layers => prepare(
            &model.layers,
            &|node| !show_identity && is_identity_transform(&node.kind, &node.summary),
            |node, row, expanded| RowInfo {
                id: node.id,
                depth: row.depth,
                has_children: row.has_children,
                expanded,
                matched: row.matched,
                title: if is_identity_transform(&node.kind, &node.summary) {
                    "Transform · identity".to_owned()
                } else {
                    node.kind.clone()
                },
                detail: if is_identity_transform(&node.kind, &node.summary) {
                    String::new()
                } else {
                    node.summary.clone()
                },
                trailing: if node.children.is_empty() {
                    String::new()
                } else {
                    node.children.len().to_string()
                },
                accent: match node.kind.as_str() {
                    "Picture" => primary(),
                    "Transform" if is_identity_transform(&node.kind, &node.summary) => text_muted(),
                    "Transform" => warning(),
                    _ => accent_text(),
                },
                rect: node.bounds.and_then(highlight_bounds),
            },
            |node, _| {
                let mut fields = vec![
                    ("Summary".to_owned(), node.summary.clone()),
                    (
                        "Bounds".to_owned(),
                        node.bounds
                            .map_or_else(|| "No geometry".into(), bounds_text),
                    ),
                    ("Children".to_owned(), node.children.len().to_string()),
                ];
                if let Some(parent) = node.parent {
                    fields.push(("Parent".to_owned(), format!("#{parent}")));
                }
                Details {
                    title: node.kind.clone(),
                    subtitle: format!("layer #{}", node.id),
                    fields,
                    chips: Vec::new(),
                }
            },
        ),
    }
}

fn select_node(kind: Kind, shared: &Shared, tick: &Signal<u64>, id: u64) {
    // Selecting never leaves an overlay in the target; only hovering a row does.
    if let Ok(mut state) = shared.lock() {
        match kind {
            Kind::Semantics => drop(state.semantics.select(id)),
            Kind::Layers => drop(state.layers.select(id)),
        }
    }
    refresh(tick);
}

fn tree_list(
    kind: Kind,
    prepared: Rc<Prepared>,
    shared: Shared,
    bridge: ClientBridge,
    tick: Signal<u64>,
    scroll: ScrollController,
) -> Widget {
    let count = prepared.rows.len();
    if count == 0 {
        let (title, hint) = if !prepared.loaded {
            (
                "Waiting for the snapshot",
                "Use Refresh if nothing arrives.",
            )
        } else if prepared.searching {
            (
                "Nothing matches this search",
                "Clear the search to see the whole tree.",
            )
        } else {
            (
                "The tree is empty",
                "The target has nothing retained here yet.",
            )
        };
        return Padding::all(
            16.,
            column([
                ui_text(title, 13., text_primary()),
                gap(1., 8.),
                ui_text(hint, 11., text_muted()),
            ]),
        )
        .into();
    }
    CustomScrollView::new(vec![Box::new(SliverFixedExtentList::new(
        count,
        ROW_HEIGHT,
        move |index| {
            let Some(row) = prepared.rows.get(index) else {
                return gap(1., ROW_HEIGHT);
            };
            let id = row.id;
            let selected = prepared.selected == Some(id);
            let disclosure: Widget = if row.has_children && !prepared.searching {
                let icon: Widget = Icon::new(icons::chevron_right())
                    .size(11.)
                    .brush(text_muted())
                    .into();
                let icon: Widget = if row.expanded {
                    Transform::rotation(std::f32::consts::FRAC_PI_2, icon).into()
                } else {
                    icon
                };
                let shared = shared.clone();
                let tick = tick.clone();
                RawMaterialButton::new(if row.expanded {
                    "Collapse branch"
                } else {
                    "Expand branch"
                })
                .size(Size::new(20., ROW_HEIGHT))
                .color(Color::TRANSPARENT)
                .content(Align::new(Alignment::CENTER, icon))
                .on_press(move || {
                    if let Ok(mut state) = shared.lock() {
                        match kind {
                            Kind::Semantics => state.semantics.toggle(id),
                            Kind::Layers => state.layers.toggle(id),
                        }
                    }
                    refresh(&tick);
                })
                .into()
            } else {
                gap(20., ROW_HEIGHT)
            };
            let title = row.title.clone();
            let detail = row.detail.clone();
            let accent = if selected { text_primary() } else { row.accent };
            let matched = row.matched && prepared.searching;
            let label = LayoutBuilder::new(move |_, constraints| {
                let text = |value: String, color: Color| {
                    Text::new(value)
                        .style(text_style(12., color).font_family("Consolas"))
                        .max_lines(Some(1))
                        .overflow(TextOverflow::Ellipsis)
                };
                let mut parts: Vec<Widget> = vec![
                    ConstrainedBox::new(
                        Constraints::loose(Size::new(
                            (constraints.max_width() * 0.45).min(200.),
                            ROW_HEIGHT,
                        )),
                        text(title.clone(), if matched { primary() } else { accent }),
                    )
                    .into(),
                ];
                if !detail.is_empty() {
                    parts.push(gap(8., 1.));
                    parts.push(Expanded::new(text(detail.clone(), warning())).into());
                }
                Row::new(parts).into()
            });
            let content: Widget = Align::new(
                Alignment::CENTER_LEFT,
                Padding::new(
                    EdgeInsets::symmetric(5., 0.),
                    Row::new([
                        Expanded::new(label).into(),
                        gap(8., 1.),
                        ui_text(row.trailing.clone(), 10., text_muted()),
                        gap(10., 1.),
                    ]),
                ),
            )
            .into();
            let press_shared = shared.clone();
            let press_tick = tick.clone();
            let enter_shared = shared.clone();
            let enter_bridge = bridge.clone();
            let exit_shared = shared.clone();
            let exit_bridge = bridge.clone();
            let rect = row.rect;
            let button: Widget = RawMaterialButton::new(format!("Select {}", row.title))
                .size(Size::new(0., ROW_HEIGHT))
                .content(content)
                .color(Color::TRANSPARENT)
                .hover_color(super::theme::hover_overlay())
                .pressed_color(super::theme::pressed_overlay())
                .focused_color(super::theme::focus_ring())
                .on_press(move || {
                    select_node(kind, &press_shared, &press_tick, id);
                })
                .on_hover(move || super::hover::enter_rect(&enter_shared, &enter_bridge, id, rect))
                .on_exit(move || super::hover::leave_rect(&exit_shared, &exit_bridge, id))
                .into();
            let depth = row.depth.min(16);
            let mut guides = Canvas::default();
            if !prepared.searching {
                for level in 0..depth {
                    guides.rect(
                        incular::core::Rect::from_origin_size(
                            Offset::new(f32::from(level) * 14. + 9., 0.),
                            Size::new(1., ROW_HEIGHT),
                        ),
                        border(),
                    );
                }
            }
            let indent = if prepared.searching {
                f32::from(depth.min(6)) * 8.
            } else {
                f32::from(depth) * 14.
            };
            Container::with_child(Row::new([
                Widget::box_(
                    Size::new(3., ROW_HEIGHT),
                    if selected {
                        primary()
                    } else {
                        Color::TRANSPARENT
                    },
                ),
                gap(8., 1.),
                CustomPaint::new(Size::new(indent, ROW_HEIGHT), guides.finish()).into(),
                disclosure,
                Expanded::new(button).into(),
            ]))
            .color(if selected {
                control_active()
            } else {
                Color::TRANSPARENT
            })
            .into()
        },
    )) as Box<dyn Sliver>])
    .controller(scroll)
    .into()
}

fn chips(title: &str, values: &[String]) -> Widget {
    let body: Widget = if values.is_empty() {
        ui_text("none", 12., text_muted())
    } else {
        Wrap::new(
            values
                .iter()
                .map(|value| badge(value.clone(), text_primary())),
        )
        .spacing(6.)
        .run_spacing(6.)
        .into()
    };
    column([
        ui_text(title, 10., text_muted()),
        gap(1., 6.),
        body,
        gap(1., 16.),
    ])
}

fn details_pane(kind: Kind, details: Option<Details>) -> Widget {
    let Some(details) = details else {
        return Padding::all(
            20.,
            column([
                heading("Nothing selected", 14.),
                gap(1., 6.),
                ui_text(kind.empty_hint(), 12., text_muted()),
                gap(1., 10.),
                ui_text(
                    "Hover a row to outline its bounds in the target window.",
                    11.,
                    text_muted(),
                ),
            ]),
        )
        .into();
    };
    let mut body = vec![
        heading(details.title, 18.),
        gap(1., 2.),
        mono(details.subtitle, 11., text_muted()),
        gap(1., 16.),
    ];
    for (label, value) in details.fields {
        body.push(key_value(label, value));
    }
    body.push(gap(1., 14.));
    for (title, values) in &details.chips {
        body.push(chips(title, values));
    }
    Padding::all(20., column(body)).into()
}

pub(crate) struct InspectionContext {
    pub(crate) shared: Shared,
    pub(crate) bridge: ClientBridge,
    pub(crate) tick: Signal<u64>,
    pub(crate) search: TextEditingController,
    pub(crate) scroll: ScrollController,
    pub(crate) details_scroll: ScrollController,
    pub(crate) split: Signal<f32>,
    pub(crate) window: Option<DevWindowId>,
}

/// Full-height tree + details workspace for one inspection kind.
pub(crate) fn build_inspection(kind: Kind, context: InspectionContext) -> Widget {
    let InspectionContext {
        shared,
        bridge,
        tick,
        search,
        scroll,
        details_scroll,
        split,
        window,
    } = context;
    let (mut prepared_data, show_identity) = {
        let model = shared.lock().expect("inspector state");
        (prepare_for(kind, &model), model.show_identity_transforms)
    };
    let details_data = prepared_data.details.take();
    let prepared = Rc::new(prepared_data);
    let total = prepared.total;
    let visible = prepared.rows.len();
    let truncated = prepared.truncated;
    let searching = prepared.searching;

    let search_field: Widget = TextField::new(search)
        .size(Size::new(0., 18.))
        .style(input_style())
        .placeholder(kind.placeholder())
        .on_changed({
            let shared = shared.clone();
            let tick = tick.clone();
            let scroll = scroll.clone();
            move |query| {
                if let Ok(mut state) = shared.lock() {
                    match kind {
                        Kind::Semantics => state.semantics.set_query(query),
                        Kind::Layers => state.layers.set_query(query),
                    }
                }
                scroll.deferred_jump_to(0.);
                refresh(&tick);
            }
        })
        .into();

    let mut controls: Vec<Widget> = Vec::new();
    {
        let bridge = bridge.clone();
        controls.push(quiet_button("Refresh", false, true, move || {
            request_refresh(&bridge, kind, window);
        }));
    }
    for (label, expand) in [("Expand all", true), ("Collapse all", false)] {
        let shared = shared.clone();
        let tick = tick.clone();
        controls.push(quiet_button(label, false, !searching, move || {
            if let Ok(mut state) = shared.lock() {
                match (kind, expand) {
                    (Kind::Semantics, true) => state.semantics.expand_all(),
                    (Kind::Semantics, false) => state.semantics.collapse_all(),
                    (Kind::Layers, true) => state.layers.expand_all(),
                    (Kind::Layers, false) => state.layers.collapse_all(),
                }
            }
            refresh(&tick);
        }));
    }
    {
        let shared = shared.clone();
        let tick = tick.clone();
        controls.push(quiet_button(
            "Clear selection",
            false,
            prepared.selected.is_some(),
            move || {
                if let Ok(mut state) = shared.lock() {
                    match kind {
                        Kind::Semantics => state.semantics.clear_selection(),
                        Kind::Layers => state.layers.clear_selection(),
                    }
                }
                refresh(&tick);
            },
        ));
    }
    if kind == Kind::Layers {
        let shared = shared.clone();
        let tick = tick.clone();
        let hiding = !show_identity;
        controls.push(quiet_button(
            "Hide identity transforms",
            hiding,
            true,
            move || {
                if let Ok(mut state) = shared.lock() {
                    state.show_identity_transforms = !state.show_identity_transforms;
                }
                refresh(&tick);
            },
        ));
    }
    let status = format!(
        "{visible} {} · {total} loaded{}",
        if searching { "matches" } else { "visible" },
        if truncated {
            " · truncated at 4096"
        } else {
            ""
        }
    );
    let list = tree_list(kind, prepared, shared, bridge, tick, scroll);

    LayoutBuilder::new(move |_, constraints| {
        let width = constraints.max_width().max(720.);
        let tree_width = (width * split.get()).clamp(320., width - 320.);
        let tree = Container::with_child(
            Column::new([
                Padding::new(
                    EdgeInsets::symmetric(12., 10.),
                    SizedBox::new().height(30.).child(search_field.clone()),
                )
                .into(),
                Padding::new(
                    EdgeInsets::symmetric(8., 0.),
                    Wrap::new(controls.clone()).spacing(4.).run_spacing(4.),
                )
                .into(),
                gap(1., 6.),
                Widget::box_(Size::new(width, 1.), border()),
                Expanded::new(list.clone()).into(),
                Widget::box_(Size::new(width, 1.), border()),
                Padding::new(
                    EdgeInsets::symmetric(14., 6.),
                    Text::new(status.clone())
                        .style(text_style(10., text_muted()))
                        .max_lines(Some(1))
                        .overflow(TextOverflow::Ellipsis),
                )
                .into(),
            ])
            .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .color(surface());
        let pane = Container::with_child(incular::widgets::internal::ScrollView::vertical(
            details_scroll.clone(),
            details_pane(kind, details_data.clone()),
        ))
        .alignment(Alignment::TOP_LEFT)
        .color(background());
        let split = split.clone();
        SplitView::horizontal(tree, pane)
            .first_extent(tree_width)
            .min_first(320.)
            .divider_color(border())
            .divider_visual_extent(1.)
            .divider_hit_extent(8.)
            .on_split_changed(move |delta| {
                split.set(((tree_width + delta).clamp(320., width - 320.)) / width);
            })
            .into()
    })
    .into()
}
