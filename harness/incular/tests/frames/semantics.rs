//! Semantic frame scenarios. Each one has a Flutter twin in
//! harness/flutter/test/frames/semantics_test.dart; keep them in sync.

use std::cell::Cell;
use std::rc::Rc;

use incular_config::Axis;
use incular_core::{Color, Size};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_semantics::SemanticAction;
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::{
    BlockSemantics, ColoredBox, Column, EditableText, ExcludeSemantics, GestureDetector,
    IndexedSemantics, ListView, MergeSemantics, Offstage, Opacity, Row, ScrollController,
    Semantics, SizedBox, Visibility, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);
const RED: Color = Color::rgba(0xF4, 0x43, 0x36, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x98, 0x00, 0xFF);
const PURPLE: Color = Color::rgba(0x9C, 0x27, 0xB0, 0xFF);

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
        ("semantics__container_label_only", container_label_only),
        ("semantics__opacity_zero", opacity_zero),
        ("semantics__offstage_visibility", offstage_visibility),
        ("semantics__value_hint_tooltip", value_hint_tooltip),
        ("semantics__increase_decrease", increase_decrease),
        ("semantics__roles_flags", roles_flags),
        ("semantics__gesture_tap", gesture_tap),
        ("semantics__live_region", live_region),
        ("semantics__horizontal_scroll", horizontal_scroll),
        ("semantics__set_text", set_text),
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
        if node.label.as_deref() == Some("North\nSouth") {
            merged_count += 1;
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

#[allow(clippy::too_many_arguments)]
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

fn count_labelled(h: &mut FrameHarness, label: &str) -> f64 {
    h.runtime()
        .tree()
        .semantics()
        .iter()
        .filter(|(_, node)| node.label.as_deref() == Some(label))
        .count() as f64
}

fn container_label_only(h: &mut FrameHarness) {
    let tap_node_count = Rc::new(Cell::new(0.0));
    let tap_action = Rc::new(Cell::new(0.0));
    let tap_is_button = Rc::new(Cell::new(0.0));
    let label_only_node_count = Rc::new(Cell::new(0.0));
    h.value("semantics.tap_node_count", {
        let value = tap_node_count.clone();
        move || value.get()
    });
    h.value("semantics.tap_action", {
        let value = tap_action.clone();
        move || value.get()
    });
    h.value("semantics.tap_is_button", {
        let value = tap_is_button.clone();
        move || value.get()
    });
    h.value("semantics.label_only_node_count", {
        let value = label_only_node_count.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Row::new([
                Widget::from(
                    Semantics::new(
                        SizedBox::new()
                            .width(120.)
                            .height(40.)
                            .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                    )
                    .label("Plain")
                    .on_tap(|| {}),
                ),
                Widget::from(
                    Semantics::new(
                        SizedBox::new()
                            .width(120.)
                            .height(40.)
                            .child(ColoredBox::new(GREEN, SizedBox::shrink())),
                    )
                    .label("Label only"),
                ),
            ]),
        )
    });

    let semantics = h.runtime().tree().semantics();
    let tap_node = semantics
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Plain"))
        .map(|(_, node)| node);
    tap_node_count.set(
        semantics
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some("Plain"))
            .count() as f64,
    );
    tap_action.set(tap_node.is_some_and(|node| {
        node.actions
            .iter()
            .any(|action| format!("{action:?}") == "Activate")
    }) as u8 as f64);
    tap_is_button
        .set(tap_node.is_some_and(|node| format!("{:?}", node.role) == "Button") as u8 as f64);
    label_only_node_count.set(
        semantics
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some("Label only"))
            .count() as f64,
    );
    h.frames(1);
}

fn opacity_zero(h: &mut FrameHarness) {
    let faded_count = Rc::new(Cell::new(0.0));
    h.value("semantics.faded_node_count", {
        let value = faded_count.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Widget::from(Opacity::new(
                0.0,
                Semantics::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .label("Faded")
                .button(true),
            )),
        )
    });

    faded_count.set(count_labelled(h, "Faded"));
    h.frames(1);
}

fn labelled_box(label: &str, color: Color) -> Semantics {
    Semantics::new(
        SizedBox::new()
            .width(60.)
            .height(40.)
            .child(ColoredBox::new(color, SizedBox::shrink())),
    )
    .label(label)
    .button(true)
}

fn offstage_visibility(h: &mut FrameHarness) {
    let offstage_count = Rc::new(Cell::new(0.0));
    let invisible_count = Rc::new(Cell::new(0.0));
    let maintained_count = Rc::new(Cell::new(0.0));
    h.value("semantics.offstage_node_count", {
        let value = offstage_count.clone();
        move || value.get()
    });
    h.value("semantics.invisible_node_count", {
        let value = invisible_count.clone();
        move || value.get()
    });
    h.value("semantics.maintained_node_count", {
        let value = maintained_count.clone();
        move || value.get()
    });

    h.mount(|_| {
        probe(
            "subject",
            Row::new([
                Widget::from(Offstage::new(labelled_box("Offstage", BLUE))),
                Widget::from(Visibility::new(labelled_box("Invisible", GREEN)).visible(false)),
                Widget::from(
                    Visibility::new(labelled_box("Maintained", RED))
                        .visible(false)
                        .maintain_state(true)
                        .maintain_animation(true)
                        .maintain_size(true)
                        .maintain_semantics(true),
                ),
            ]),
        )
    });

    offstage_count.set(count_labelled(h, "Offstage"));
    invisible_count.set(count_labelled(h, "Invisible"));
    maintained_count.set(count_labelled(h, "Maintained"));
    h.frames(1);
}

fn value_hint_tooltip(h: &mut FrameHarness) {
    let value_length = Rc::new(Cell::new(0.0));
    let hint_length = Rc::new(Cell::new(0.0));
    let tooltip_length = Rc::new(Cell::new(0.0));
    h.value("semantics.value_length", {
        let value = value_length.clone();
        move || value.get()
    });
    h.value("semantics.hint_length", {
        let value = hint_length.clone();
        move || value.get()
    });
    h.value("semantics.tooltip_length", {
        let value = tooltip_length.clone();
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
                .label("Slider")
                .value("50%")
                .hint("Adjusts volume")
                .tooltip("Volume")
                .button(true),
            ),
        )
    });

    if let Some((_, node)) = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Slider"))
    {
        value_length.set(node.value.as_ref().map_or(0, |value| value.len()) as f64);
        // Incular stores hint and tooltip in the single description field.
        hint_length.set(node.description.as_ref().map_or(0, |text| text.len()) as f64);
        tooltip_length.set(node.state.tooltip.as_ref().map_or(0, |text| text.len()) as f64);
    }
    h.frames(1);
}

fn increase_decrease(h: &mut FrameHarness) {
    let increase_calls = Rc::new(Cell::new(0.0));
    let decrease_calls = Rc::new(Cell::new(0.0));
    let has_increase = Rc::new(Cell::new(0.0));
    let has_decrease = Rc::new(Cell::new(0.0));
    h.value("semantics.increase_calls", {
        let value = increase_calls.clone();
        move || value.get()
    });
    h.value("semantics.decrease_calls", {
        let value = decrease_calls.clone();
        move || value.get()
    });
    h.value("semantics.has_increase", {
        let value = has_increase.clone();
        move || value.get()
    });
    h.value("semantics.has_decrease", {
        let value = has_decrease.clone();
        move || value.get()
    });
    let increase_target = increase_calls.clone();
    let decrease_target = decrease_calls.clone();
    h.mount(move |_| {
        let increase_target = increase_target.clone();
        let decrease_target = decrease_target.clone();
        probe(
            "subject",
            Widget::from(
                Semantics::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .label("Stepper")
                .value("5")
                .slider(true)
                .on_increase(move || increase_target.set(increase_target.get() + 1.0))
                .on_decrease(move || decrease_target.set(decrease_target.get() + 1.0)),
            ),
        )
    });

    let node = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Stepper"))
        .map(|(id, node)| {
            (
                id,
                node.actions
                    .iter()
                    .any(|action| format!("{action:?}") == "Increment"),
                node.actions
                    .iter()
                    .any(|action| format!("{action:?}") == "Decrement"),
            )
        })
        .expect("stepper semantic node");
    has_increase.set(node.1 as u8 as f64);
    has_decrease.set(node.2 as u8 as f64);
    h.frames(1);
    assert!(
        h.runtime()
            .dispatch_semantic_action(node.0, SemanticAction::Increment)
    );
    assert!(
        h.runtime()
            .dispatch_semantic_action(node.0, SemanticAction::Decrement)
    );
    h.frames(1);
}

fn roles_flags(h: &mut FrameHarness) {
    let header_count = Rc::new(Cell::new(0.0));
    let link_count = Rc::new(Cell::new(0.0));
    let image_count = Rc::new(Cell::new(0.0));
    let toggled_count = Rc::new(Cell::new(0.0));
    let hidden_count = Rc::new(Cell::new(0.0));
    h.value("semantics.header_count", {
        let value = header_count.clone();
        move || value.get()
    });
    h.value("semantics.link_count", {
        let value = link_count.clone();
        move || value.get()
    });
    h.value("semantics.image_count", {
        let value = image_count.clone();
        move || value.get()
    });
    h.value("semantics.toggled_count", {
        let value = toggled_count.clone();
        move || value.get()
    });
    h.value("semantics.hidden_count", {
        let value = hidden_count.clone();
        move || value.get()
    });

    let box_of = |color| {
        SizedBox::new()
            .width(60.)
            .height(40.)
            .child(ColoredBox::new(color, SizedBox::shrink()))
    };
    h.mount(move |_| {
        probe(
            "subject",
            Column::new([
                Widget::from(Semantics::new(box_of(BLUE)).label("Heading").header(true)),
                Widget::from(Semantics::new(box_of(GREEN)).label("Link").link(true)),
                Widget::from(Semantics::new(box_of(ORANGE)).label("Picture").image(true)),
                Widget::from(
                    Semantics::new(box_of(PURPLE))
                        .label("Toggle")
                        .button(true)
                        .toggled(true),
                ),
                Widget::from(
                    Semantics::new(box_of(RED))
                        .label("Concealed")
                        .button(true)
                        .hidden(true),
                ),
            ]),
        )
    });

    let mut headers = 0.0;
    let mut links = 0.0;
    let mut images = 0.0;
    let mut toggled = 0.0;
    let mut hidden = 0.0;
    for (_, node) in h.runtime().tree().semantics().iter() {
        match format!("{:?}", node.role).as_str() {
            "Heading" => headers += 1.0,
            "Link" => links += 1.0,
            "Image" => images += 1.0,
            _ => {}
        }
        if node.state.toggled == Some(true) {
            toggled += 1.0;
        }
        if node.state.hidden {
            hidden += 1.0;
        }
    }
    header_count.set(headers);
    link_count.set(links);
    image_count.set(images);
    toggled_count.set(toggled);
    hidden_count.set(hidden);
    h.frames(1);
}

fn gesture_tap(h: &mut FrameHarness) {
    let tap_node_count = Rc::new(Cell::new(0.0));
    let tap_calls = Rc::new(Cell::new(0.0));
    h.value("semantics.tap_node_count", {
        let value = tap_node_count.clone();
        move || value.get()
    });
    h.value("semantics.tap_calls", {
        let value = tap_calls.clone();
        move || value.get()
    });
    let callback_calls = tap_calls.clone();
    h.mount(move |_| {
        let callback_calls = callback_calls.clone();
        probe(
            "subject",
            Widget::from(
                GestureDetector::new(
                    SizedBox::new()
                        .width(120.)
                        .height(40.)
                        .child(ColoredBox::new(BLUE, SizedBox::shrink())),
                )
                .on_tap(move || callback_calls.set(callback_calls.get() + 1.0)),
            ),
        )
    });

    let tap_node = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| {
            node.actions
                .iter()
                .any(|action| format!("{action:?}") == "Activate")
        })
        .map(|(id, _)| id);
    tap_node_count.set(
        h.runtime()
            .tree()
            .semantics()
            .iter()
            .filter(|(_, node)| {
                node.actions
                    .iter()
                    .any(|action| format!("{action:?}") == "Activate")
            })
            .count() as f64,
    );
    h.frames(1);
    if let Some(id) = tap_node {
        assert!(
            h.runtime()
                .dispatch_semantic_action(id, SemanticAction::Activate)
        );
    }
    h.frames(1);
}

fn live_region(h: &mut FrameHarness) {
    let live_region_count = Rc::new(Cell::new(0.0));
    h.value("semantics.live_region_count", {
        let value = live_region_count.clone();
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
                .label("Status")
                .group(true)
                .live_region(true),
            ),
        )
    });
    let count = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .filter(|(_, node)| node.state.live_region)
        .count();
    live_region_count.set(count as f64);
    h.frames(1);
}

fn horizontal_scroll(h: &mut FrameHarness) {
    let scroll_action_count = Rc::new(Cell::new(0.0));
    let scroll_position = Rc::new(Cell::new(-1.0));
    let scroll_extent_max = Rc::new(Cell::new(-1.0));
    h.value("semantics.scroll_action_count", {
        let value = scroll_action_count.clone();
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

    let controller = ScrollController::new();
    let view_controller = controller.clone();
    h.mount(move |_| {
        probe(
            "subject",
            SizedBox::new().height(100.).child(
                ListView::builder(12, |index| {
                    Semantics::new(
                        SizedBox::new()
                            .width(100.)
                            .height(100.)
                            .child(ColoredBox::new(
                                if index % 2 == 0 { BLUE } else { GREEN },
                                SizedBox::shrink(),
                            )),
                    )
                    .label(format!("Column {index}"))
                    .button(true)
                })
                .scroll_direction(Axis::Horizontal)
                .controller(view_controller.clone()),
            ),
        )
    });

    let node_count = Rc::new(Cell::new(0.0));
    let up = Rc::new(Cell::new(0.0));
    let down = Rc::new(Cell::new(0.0));
    let child_count = Rc::new(Cell::new(0.0));
    let sample = |h: &mut FrameHarness| {
        capture_scroll_semantics(
            h,
            &node_count,
            &scroll_action_count,
            &up,
            &down,
            &scroll_position,
            &scroll_extent_max,
            &child_count,
        );
    };
    sample(h);
    h.frames(1);
    controller.jump_to(controller.max_offset());
    h.frames(1);
    sample(h);
    h.frames(1);
}

fn set_text(h: &mut FrameHarness) {
    let controller = TextEditingController::new();
    let text_source = controller.clone();
    h.value("semantics.text_length", move || {
        text_source.text().len() as f64
    });
    let set_text_nodes = Rc::new(Cell::new(0.0));
    h.value("semantics.set_text_nodes", {
        let value = set_text_nodes.clone();
        move || value.get()
    });
    let field_controller = controller.clone();
    h.mount(move |_| {
        probe(
            "subject",
            SizedBox::new().width(200.).height(48.).child(
                EditableText::new(field_controller.clone())
                    .size(Size::new(200., 48.))
                    .style(TextStyle::default()),
            ),
        )
    });
    let node = h
        .runtime()
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| {
            node.actions
                .iter()
                .any(|action| format!("{action:?}") == "SetText")
        })
        .map(|(id, _)| id)
        .expect("text field semantic node");
    set_text_nodes.set(
        h.runtime()
            .tree()
            .semantics()
            .iter()
            .filter(|(_, node)| {
                node.actions
                    .iter()
                    .any(|action| format!("{action:?}") == "SetText")
            })
            .count() as f64,
    );
    h.frames(1);
    assert!(
        h.runtime()
            .dispatch_semantic_action(node, SemanticAction::SetText("hello".to_owned()))
    );
    h.frames(1);
}
