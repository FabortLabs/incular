//! Semantic frame scenarios. Each one has a Flutter twin in
//! harness/flutter/test/frames/semantics_test.dart; keep them in sync.

use std::cell::Cell;
use std::rc::Rc;

use incular_core::Color;
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_semantics::SemanticAction;
use incular_widgets::{
    BlockSemantics, ColoredBox, ExcludeSemantics, IndexedSemantics, ListView, MergeSemantics, Row,
    ScrollController, Semantics, SizedBox, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("semantics__properties_state", properties_state),
        ("semantics__enabled_default", enabled_default),
        ("semantics__exclude", exclude),
        ("semantics__merge", merge),
        ("semantics__block_previous", block_previous),
        ("semantics__indexed", indexed),
        ("semantics__scrollable", scrollable),
        ("semantics__action_dispatch", action_dispatch),
        ("semantics__stale_action", stale_action),
    ]
}

fn properties_state(h: &mut FrameHarness) {
    let label_length = Rc::new(Cell::new(0.0));
    let is_button = Rc::new(Cell::new(0.0));
    let is_enabled = Rc::new(Cell::new(0.0));
    let is_selected = Rc::new(Cell::new(0.0));
    let is_checked = Rc::new(Cell::new(0.0));
    let has_tap = Rc::new(Cell::new(0.0));
    h.value("semantics.label_length", {
        let value = label_length.clone();
        move || value.get()
    });
    h.value("semantics.button", {
        let value = is_button.clone();
        move || value.get()
    });
    h.value("semantics.enabled", {
        let value = is_enabled.clone();
        move || value.get()
    });
    h.value("semantics.selected", {
        let value = is_selected.clone();
        move || value.get()
    });
    h.value("semantics.checked", {
        let value = is_checked.clone();
        move || value.get()
    });
    h.value("semantics.tap_action", {
        let value = has_tap.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Widget::from(
                Semantics::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .label("Ready")
                .button(true)
                .enabled(true)
                .selected(true)
                .checked(true)
                .on_tap(|| {}),
            ),
        )
    });

    if let Some((_, node)) = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Ready"))
    {
        label_length.set(node.label.as_ref().map_or(0, |label| label.len()) as f64);
        is_button.set((format!("{:?}", node.role) == "Button") as u8 as f64);
        is_enabled.set(node.state.enabled as u8 as f64);
        is_selected.set(node.state.selected as u8 as f64);
        is_checked.set(
            node.state
                .checked
                .is_some_and(|checked| format!("{checked:?}") == "Checked") as u8
                as f64,
        );
        has_tap.set(
            node.actions
                .iter()
                .any(|action| format!("{action:?}") == "Activate") as u8 as f64,
        );
    }
    h.frames(1);
}

fn enabled_default(h: &mut FrameHarness) {
    let enabled_state = Rc::new(Cell::new(0.0));
    let has_tap = Rc::new(Cell::new(0.0));
    h.value("semantics.enabled_state", {
        let value = enabled_state.clone();
        move || value.get()
    });
    h.value("semantics.tap_action", {
        let value = has_tap.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Widget::from(
                Semantics::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .label("Action")
                .button(true)
                .on_tap(|| {}),
            ),
        )
    });

    if let Some((_, node)) = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Action"))
    {
        enabled_state.set(node.state.enabled as u8 as f64);
        has_tap.set(
            node.actions
                .iter()
                .any(|action| format!("{action:?}") == "Activate") as u8 as f64,
        );
    }
    h.frames(1);
}

fn exclude(h: &mut FrameHarness) {
    let visible_label = Rc::new(Cell::new(0.0));
    let hidden_label = Rc::new(Cell::new(0.0));
    h.value("semantics.visible_label", {
        let value = visible_label.clone();
        move || value.get()
    });
    h.value("semantics.hidden_label", {
        let value = hidden_label.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Row::new([
                Widget::from(
                    Semantics::new(
                        SizedBox::new()
                            .width(60.)
                            .height(40.)
                            .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                    )
                    .label("Visible")
                    .button(true),
                ),
                Widget::from(ExcludeSemantics::new(
                    Semantics::new(
                        SizedBox::new()
                            .width(60.)
                            .height(40.)
                            .child(ColoredBox::new(GREEN, SizedBox::shrink())),
                    )
                    .label("Hidden")
                    .button(true),
                )),
            ]),
        )
    });

    let semantics = h.runtime().tree().semantics();
    visible_label.set(
        semantics
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some("Visible"))
            .count() as f64,
    );
    hidden_label.set(
        semantics
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some("Hidden"))
            .count() as f64,
    );
    h.frames(1);
}

fn merge(h: &mut FrameHarness) {
    let semantic_node_count = Rc::new(Cell::new(0.0));
    let merged_label_length = Rc::new(Cell::new(0.0));
    let merged_label_count = Rc::new(Cell::new(0.0));
    h.value("semantics.node_count", {
        let value = semantic_node_count.clone();
        move || value.get()
    });
    h.value("semantics.merged_label_length", {
        let value = merged_label_length.clone();
        move || value.get()
    });
    h.value("semantics.merged_label_count", {
        let value = merged_label_count.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            MergeSemantics::new(Row::new([
                Widget::from(
                    Semantics::new(
                        SizedBox::new()
                            .width(60.)
                            .height(40.)
                            .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                    )
                    .label("North")
                    .button(true),
                ),
                Widget::from(
                    Semantics::new(
                        SizedBox::new()
                            .width(60.)
                            .height(40.)
                            .child(ColoredBox::new(GREEN, SizedBox::shrink())),
                    )
                    .label("South")
                    .button(true),
                ),
            ])),
        )
    });

    let semantics = h.runtime().tree().semantics();
    semantic_node_count.set(semantics.iter().count() as f64);
    let mut merged_count = 0;
    for (_, node) in semantics.iter() {
        match node.label.as_deref() {
            Some("North\nSouth") => merged_count += 1,
            _ => {}
        }
    }
    merged_label_length.set(if merged_count > 0 {
        "North\nSouth".len() as f64
    } else {
        0.0
    });
    merged_label_count.set(merged_count as f64);
    h.frames(1);
}

fn block_previous(h: &mut FrameHarness) {
    let behind_count = Rc::new(Cell::new(0.0));
    let dialog_count = Rc::new(Cell::new(0.0));
    h.value("semantics.behind_label", {
        let value = behind_count.clone();
        move || value.get()
    });
    h.value("semantics.dialog_label", {
        let value = dialog_count.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Row::new([
                Widget::from(
                    Semantics::new(
                        SizedBox::new()
                            .width(60.)
                            .height(40.)
                            .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                    )
                    .label("Behind")
                    .button(true),
                ),
                Widget::from(BlockSemantics::new(
                    Semantics::new(
                        SizedBox::new()
                            .width(60.)
                            .height(40.)
                            .child(ColoredBox::new(GREEN, SizedBox::shrink())),
                    )
                    .label("Dialog")
                    .button(true),
                )),
            ]),
        )
    });

    let semantics = h.runtime().tree().semantics();
    behind_count.set(
        semantics
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some("Behind"))
            .count() as f64,
    );
    dialog_count.set(
        semantics
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some("Dialog"))
            .count() as f64,
    );
    h.frames(1);
}

fn indexed(h: &mut FrameHarness) {
    let index_in_parent = Rc::new(Cell::new(-1.0));
    h.value("semantics.index_in_parent", {
        let value = index_in_parent.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Widget::from(IndexedSemantics::new(
                7,
                Semantics::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .label("Indexed item")
                .button(true),
            )),
        )
    });

    if let Some((_, node)) = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Indexed item"))
    {
        index_in_parent.set(node.state.item_index.map_or(-1.0, |index| index as f64));
    }
    h.frames(1);
}

fn scrollable(h: &mut FrameHarness) {
    let scroll_node_count = Rc::new(Cell::new(0.0));
    let scroll_action_count = Rc::new(Cell::new(0.0));
    let scroll_up_action = Rc::new(Cell::new(0.0));
    let scroll_down_action = Rc::new(Cell::new(0.0));
    let scroll_position = Rc::new(Cell::new(-1.0));
    let scroll_extent_max = Rc::new(Cell::new(-1.0));
    let scroll_child_count = Rc::new(Cell::new(-1.0));
    h.value("semantics.scroll_node_count", {
        let value = scroll_node_count.clone();
        move || value.get()
    });
    h.value("semantics.scroll_action_count", {
        let value = scroll_action_count.clone();
        move || value.get()
    });
    h.value("semantics.scroll_up_action", {
        let value = scroll_up_action.clone();
        move || value.get()
    });
    h.value("semantics.scroll_down_action", {
        let value = scroll_down_action.clone();
        move || value.get()
    });
    h.value("semantics.scroll_position", {
        let value = scroll_position.clone();
        move || value.get()
    });
    h.value("semantics.scroll_extent_max", {
        let value = scroll_extent_max.clone();
        move || value.get()
    });
    h.value("semantics.scroll_child_count", {
        let value = scroll_child_count.clone();
        move || value.get()
    });

    let controller = ScrollController::new();
    let view_controller = controller.clone();
    h.mount(move |_| {
        probe(
            "subject",
            ListView::builder(12, |index| {
                Semantics::new(
                    SizedBox::new()
                        .width(800.)
                        .height(100.)
                        .child(ColoredBox::new(
                            if index % 2 == 0 { BLUE } else { GREEN },
                            SizedBox::shrink(),
                        )),
                )
                .label(format!("Row {index}"))
                .button(true)
            })
            .controller(view_controller.clone()),
        )
    });

    capture_scroll_semantics(
        h,
        &scroll_node_count,
        &scroll_action_count,
        &scroll_up_action,
        &scroll_down_action,
        &scroll_position,
        &scroll_extent_max,
        &scroll_child_count,
    );
    h.frames(1);
    controller.jump_to(controller.max_offset());
    h.frames(1);
    capture_scroll_semantics(
        h,
        &scroll_node_count,
        &scroll_action_count,
        &scroll_up_action,
        &scroll_down_action,
        &scroll_position,
        &scroll_extent_max,
        &scroll_child_count,
    );
    h.frames(1);
}

fn capture_scroll_semantics(
    h: &mut FrameHarness,
    node_count: &Cell<f64>,
    action_count: &Cell<f64>,
    up_action: &Cell<f64>,
    down_action: &Cell<f64>,
    position: &Cell<f64>,
    extent_max: &Cell<f64>,
    child_count: &Cell<f64>,
) {
    let mut nodes = 0.0;
    let mut actions = 0.0;
    let mut up = 0.0;
    let mut down = 0.0;
    let mut offset = -1.0;
    let mut max = -1.0;
    let mut children = -1.0;
    for (_, node) in h.runtime().tree().semantics().iter() {
        if format!("{:?}", node.role) == "ScrollView" {
            nodes += 1.0;
            if let Some(value) = node.value.as_deref()
                && let Some((current, extent)) = value.split_once('/')
            {
                offset = current.parse().unwrap_or(-1.0);
                max = extent.parse().unwrap_or(-1.0);
            }
            up = node
                .actions
                .iter()
                .any(|action| format!("{action:?}") == "ScrollForward") as u8
                as f64;
            down = node
                .actions
                .iter()
                .any(|action| format!("{action:?}") == "ScrollBackward") as u8
                as f64;
            actions = up + down;
        }
        if children < 0.0 && node.state.set_size.is_some() {
            children = node.state.set_size.map_or(-1.0, |count| count as f64);
        }
    }
    node_count.set(nodes);
    action_count.set(actions);
    up_action.set(up);
    down_action.set(down);
    position.set(offset);
    extent_max.set(max);
    child_count.set(children);
}

fn action_dispatch(h: &mut FrameHarness) {
    let calls = Rc::new(Cell::new(0.0));
    h.value("semantics.action_calls", {
        let value = calls.clone();
        move || value.get()
    });
    let callback_calls = calls.clone();
    h.mount(move |_| {
        let callback_calls = callback_calls.clone();
        probe(
            "subject",
            Widget::from(
                Semantics::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .label("Activate")
                .button(true)
                .on_tap(move || callback_calls.set(callback_calls.get() + 1.0)),
            ),
        )
    });

    let node = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Activate"))
        .map(|(id, _)| id)
        .expect("semantic action target");
    assert!(
        h.runtime()
            .dispatch_semantic_action(node, SemanticAction::Activate)
    );
    h.frames(1);
}

fn stale_action(h: &mut FrameHarness) {
    let calls = Rc::new(Cell::new(0.0));
    h.value("semantics.stale_action_calls", {
        let value = calls.clone();
        move || value.get()
    });
    let callback_calls = calls.clone();
    h.mount(move |phase| {
        let child = if phase == 0 {
            let callback_calls = callback_calls.clone();
            Widget::from(
                Semantics::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .label("Temporary action")
                .button(true)
                .on_tap(move || callback_calls.set(callback_calls.get() + 1.0)),
            )
        } else {
            SizedBox::new()
                .width(120.)
                .height(40.)
                .child(ColoredBox::new(BLUE, SizedBox::shrink()))
                .into()
        };
        probe("subject", child)
    });

    let old_id = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Temporary action"))
        .map(|(id, _)| id)
        .expect("semantic node before removal");
    h.frames(1);
    h.set_phase(1);
    h.frames(1);
    assert!(
        !h.runtime()
            .dispatch_semantic_action(old_id, SemanticAction::Activate)
    );
    h.frames(1);
}
