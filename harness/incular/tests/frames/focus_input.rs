use std::{cell::Cell, rc::Rc};

use incular_core::Modifiers;
use incular_core::{Code, Color, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    AbsorbPointer, Actions, ColoredBox, Command, Focus, FocusNode, FocusScope, FocusTraversalGroup,
    FocusTraversalOrder, FocusableActionDetector, GestureDetector, IgnorePointer, KeyboardListener,
    Listener, LogicalShortcutKey, MouseRegion, Positioned, Row, ScrollController, Shortcuts,
    SizedBox, Stack, TapRegion, TapRegionSurface, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "focus_input__focus_scope_default_traversal",
            focus_scope_default_traversal,
        ),
        (
            "focus_input__shortcuts_action_intent",
            shortcuts_action_intent,
        ),
        ("focus_input__keyboard_page_scroll", keyboard_page_scroll),
        (
            "focus_input__mouse_region_enter_hover_exit",
            mouse_region_enter_hover_exit,
        ),
        (
            "focus_input__listener_pointer_lifecycle",
            listener_pointer_lifecycle,
        ),
        (
            "focus_input__ignore_pointer_falls_through",
            ignore_pointer_falls_through,
        ),
        (
            "focus_input__absorb_pointer_blocks_sibling",
            absorb_pointer_blocks_sibling,
        ),
        (
            "focus_input__childless_sizedbox_passes_hits",
            childless_sizedbox_passes_hits,
        ),
        (
            "focus_input__reading_order_band_overlap",
            reading_order_band_overlap,
        ),
        (
            "focus_input__ordered_unordered_reading_order",
            ordered_unordered_reading_order,
        ),
        (
            "focus_input__key_event_bubbles_past_ignored",
            key_event_bubbles_past_ignored,
        ),
        (
            "focus_input__mouse_region_opaque_blocks_underlay",
            mouse_region_opaque_blocks_underlay,
        ),
        (
            "focus_input__mouse_region_translucent_passes_underlay",
            mouse_region_translucent_passes_underlay,
        ),
        (
            "focus_input__mouse_region_nested_enter_order",
            mouse_region_nested_enter_order,
        ),
        (
            "focus_input__tap_region_group_outside",
            tap_region_group_outside,
        ),
        (
            "focus_input__arrow_directional_focus",
            arrow_directional_focus,
        ),
        (
            "focus_input__shift_tab_reverse_traversal",
            shift_tab_reverse_traversal,
        ),
        (
            "focus_input__shortcut_chord_requires_modifiers",
            shortcut_chord_requires_modifiers,
        ),
    ]
}

fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn send_key(h: &mut FrameHarness, key: KeyboardKey, code: Code) {
    h.send(InputEvent::Key(KeyboardEvent::key_down(key.clone(), code)));
    h.send(InputEvent::Key(KeyboardEvent::key_up(key, code)));
}

fn focus_scope_default_traversal(h: &mut FrameHarness) {
    let first = FocusNode::new();
    let middle = FocusNode::new();
    let last = FocusNode::new();
    let (first_read, middle_read, last_read) = (first.clone(), middle.clone(), last.clone());
    h.value("first_focused", move || f64::from(first_read.has_focus()));
    h.value("middle_focused", move || f64::from(middle_read.has_focus()));
    h.value("last_focused", move || f64::from(last_read.has_focus()));
    h.mount(move |_| {
        SizedBox::new()
            .width(120.)
            .height(100.)
            .child(FocusScope::new(Stack::new([
                Widget::from(
                    Positioned::new(
                        Focus::new(probe(
                            "first",
                            SizedBox::new().width(20.).height(20.).child(colored(BLUE)),
                        ))
                        .node(first.clone()),
                    )
                    .top(0.),
                ),
                Widget::from(
                    Positioned::new(
                        Focus::new(probe(
                            "middle",
                            SizedBox::new().width(20.).height(20.).child(colored(GREEN)),
                        ))
                        .node(middle.clone())
                        .autofocus(true),
                    )
                    .top(40.),
                ),
                Widget::from(
                    Positioned::new(
                        Focus::new(probe(
                            "last",
                            SizedBox::new()
                                .width(20.)
                                .height(20.)
                                .child(colored(ORANGE)),
                        ))
                        .node(last.clone()),
                    )
                    .top(20.),
                ),
            ])))
            .into()
    });
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
}

fn shortcuts_action_intent(h: &mut FrameHarness) {
    let calls = Rc::new(Cell::new(0_u32));
    let observed = calls.clone();
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Character("k".into()), Modifiers::empty()),
        Command::new("activate"),
    );
    let mut actions = Actions::new();
    actions.register(Command::new("activate"), move || {
        observed.set(observed.get() + 1);
    });
    let shortcuts = Rc::new(shortcuts);
    let actions = Rc::new(actions);
    let read = calls.clone();
    h.value("calls", move || f64::from(read.get()));
    h.mount(move |_| {
        FocusableActionDetector::new(probe(
            "action_target",
            SizedBox::new().width(40.).height(30.).child(colored(BLUE)),
        ))
        .autofocus(true)
        .shortcuts(shortcuts.clone())
        .actions(actions.clone())
        .into()
    });
    h.frames(1);
    send_key(h, KeyboardKey::Character("k".into()), Code::KeyK);
    h.frames(1);
}

fn keyboard_page_scroll(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let row_focus = FocusNode::new();
    let read = controller.clone();
    let focus_read = row_focus.clone();
    h.value("offset", move || f64::from(read.offset()));
    h.value("row_focused", move || f64::from(focus_read.has_focus()));
    h.mount(move |_| {
        let row_focus = row_focus.clone();
        incular_widgets::ListView::builder(30, move |index| -> Widget {
            let child =
                SizedBox::expand().child(colored(if index % 2 == 0 { BLUE } else { GREEN }));
            if index == 0 {
                Focus::new(probe("row0", child))
                    .node(row_focus.clone())
                    .autofocus(true)
                    .into()
            } else {
                child.into()
            }
        })
        .item_extent(50.)
        .controller(controller.clone())
        .into()
    });
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::PageDown), Code::PageDown);
    h.frames(10);
}

fn mouse_region_enter_hover_exit(h: &mut FrameHarness) {
    let enters = Rc::new(Cell::new(0_u32));
    let hovers = Rc::new(Cell::new(0_u32));
    let exits = Rc::new(Cell::new(0_u32));
    let (enter_read, hover_read, exit_read) = (enters.clone(), hovers.clone(), exits.clone());
    h.value("enters", move || f64::from(enter_read.get()));
    h.value("hovers", move || f64::from(hover_read.get()));
    h.value("exits", move || f64::from(exit_read.get()));
    h.mount(move |_| {
        MouseRegion::new(probe(
            "hover_target",
            SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
        ))
        .on_enter({
            let enters = enters.clone();
            move |_| enters.set(enters.get() + 1)
        })
        .on_hover({
            let hovers = hovers.clone();
            move |_| hovers.set(hovers.get() + 1)
        })
        .on_exit({
            let exits = exits.clone();
            move |_| exits.set(exits.get() + 1)
        })
        .into()
    });
    h.hover(Offset::new(20., 20.));
    h.frames(1);
    h.hover(Offset::new(30., 30.));
    h.frames(1);
    h.hover(Offset::new(100., 100.));
    h.frames(1);
}

fn listener_pointer_lifecycle(h: &mut FrameHarness) {
    let downs = Rc::new(Cell::new(0_u32));
    let moves = Rc::new(Cell::new(0_u32));
    let ups = Rc::new(Cell::new(0_u32));
    let (down_read, move_read, up_read) = (downs.clone(), moves.clone(), ups.clone());
    h.value("downs", move || f64::from(down_read.get()));
    h.value("moves", move || f64::from(move_read.get()));
    h.value("ups", move || f64::from(up_read.get()));
    h.mount(move |_| {
        Listener::new(probe(
            "listener_target",
            SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
        ))
        .on_pointer_down({
            let downs = downs.clone();
            move |_| downs.set(downs.get() + 1)
        })
        .on_pointer_move({
            let moves = moves.clone();
            move |_| moves.set(moves.get() + 1)
        })
        .on_pointer_up({
            let ups = ups.clone();
            move |_| ups.set(ups.get() + 1)
        })
        .into()
    });
    h.down(Offset::new(20., 20.));
    h.frames(1);
    h.move_to(Offset::new(30., 30.));
    h.frames(1);
    h.up();
    h.frames(1);
}

fn ignore_pointer_falls_through(h: &mut FrameHarness) {
    let taps = Rc::new(Cell::new(0_u32));
    let tap_read = taps.clone();
    h.value("taps", move || f64::from(tap_read.get()));
    h.mount(move |_| {
        let underlay: Widget = GestureDetector::new(probe(
            "underlay",
            SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
        ))
        .on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        })
        .into();
        let overlay: Widget = IgnorePointer::new(probe(
            "ignored_overlay",
            SizedBox::new()
                .width(80.)
                .height(60.)
                .child(colored(ORANGE)),
        ))
        .into();
        SizedBox::new()
            .width(80.)
            .height(60.)
            .child(Stack::new([underlay, overlay]))
            .into()
    });
    h.down(Offset::new(20., 20.));
    h.up();
    h.frames(1);
}

fn absorb_pointer_blocks_sibling(h: &mut FrameHarness) {
    let taps = Rc::new(Cell::new(0_u32));
    let tap_read = taps.clone();
    h.value("taps", move || f64::from(tap_read.get()));
    h.mount(move |_| {
        let underlay: Widget = GestureDetector::new(probe(
            "underlay",
            SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
        ))
        .on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        })
        .into();
        let overlay: Widget = AbsorbPointer::new(probe(
            "absorbing_overlay",
            SizedBox::new()
                .width(80.)
                .height(60.)
                .child(colored(ORANGE)),
        ))
        .into();
        SizedBox::new()
            .width(80.)
            .height(60.)
            .child(Stack::new([underlay, overlay]))
            .into()
    });
    h.down(Offset::new(20., 20.));
    h.up();
    h.frames(1);
}

fn childless_sizedbox_passes_hits(h: &mut FrameHarness) {
    let taps = Rc::new(Cell::new(0_u32));
    let tap_read = taps.clone();
    h.value("taps", move || f64::from(tap_read.get()));
    h.mount(move |_| {
        let underlay: Widget = GestureDetector::new(probe(
            "underlay",
            SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
        ))
        .on_tap({
            let taps = taps.clone();
            move || taps.set(taps.get() + 1)
        })
        .into();
        let overlay: Widget = Positioned::new(probe(
            "empty_overlay",
            SizedBox::new().width(20.).height(20.),
        ))
        .top(0.)
        .left(0.)
        .into();
        SizedBox::new()
            .width(80.)
            .height(60.)
            .child(Stack::new([underlay, overlay]))
            .into()
    });
    h.down(Offset::new(10., 10.));
    h.up();
    h.frames(1);
}

fn reading_order_band_overlap(h: &mut FrameHarness) {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let c = FocusNode::new();
    let (a_read, b_read, c_read) = (a.clone(), b.clone(), c.clone());
    h.value("a_focused", move || f64::from(a_read.has_focus()));
    h.value("b_focused", move || f64::from(b_read.has_focus()));
    h.value("c_focused", move || f64::from(c_read.has_focus()));
    h.mount(move |_| {
        SizedBox::new()
            .width(200.)
            .height(100.)
            .child(FocusTraversalGroup::new(Stack::new([
                Widget::from(
                    Positioned::new(
                        Focus::new(probe(
                            "a",
                            SizedBox::new().width(20.).height(20.).child(colored(BLUE)),
                        ))
                        .node(a.clone()),
                    )
                    .top(0.)
                    .left(60.),
                ),
                Widget::from(
                    Positioned::new(
                        Focus::new(probe(
                            "b",
                            SizedBox::new().width(20.).height(20.).child(colored(GREEN)),
                        ))
                        .node(b.clone()),
                    )
                    .top(15.)
                    .left(0.),
                ),
                Widget::from(
                    Positioned::new(
                        Focus::new(probe(
                            "c",
                            SizedBox::new()
                                .width(20.)
                                .height(20.)
                                .child(colored(ORANGE)),
                        ))
                        .node(c.clone()),
                    )
                    .top(40.)
                    .left(120.),
                ),
            ])))
            .into()
    });
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
}

fn ordered_unordered_reading_order(h: &mut FrameHarness) {
    let x = FocusNode::new();
    let y = FocusNode::new();
    let z = FocusNode::new();
    let (x_read, y_read, z_read) = (x.clone(), y.clone(), z.clone());
    h.value("x_focused", move || f64::from(x_read.has_focus()));
    h.value("y_focused", move || f64::from(y_read.has_focus()));
    h.value("z_focused", move || f64::from(z_read.has_focus()));
    h.mount(move |_| {
        SizedBox::new()
            .width(200.)
            .height(100.)
            .child(
                FocusTraversalGroup::new(Stack::new([
                    Widget::from(
                        Positioned::new(
                            Focus::new(probe(
                                "x",
                                SizedBox::new().width(20.).height(20.).child(colored(BLUE)),
                            ))
                            .node(x.clone()),
                        )
                        .top(40.)
                        .left(0.),
                    ),
                    Widget::from(
                        Positioned::new(
                            Focus::new(probe(
                                "y",
                                SizedBox::new().width(20.).height(20.).child(colored(GREEN)),
                            ))
                            .node(y.clone()),
                        )
                        .top(0.)
                        .left(60.),
                    ),
                    Widget::from(
                        Positioned::new(
                            FocusTraversalOrder::new(
                                Focus::new(probe(
                                    "z",
                                    SizedBox::new()
                                        .width(20.)
                                        .height(20.)
                                        .child(colored(ORANGE)),
                                ))
                                .node(z.clone()),
                            )
                            .order(1.),
                        )
                        .top(20.)
                        .left(120.),
                    ),
                ]))
                .ordered(),
            )
            .into()
    });
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
}

fn key_event_bubbles_past_ignored(h: &mut FrameHarness) {
    let outer = FocusNode::new();
    let inner = FocusNode::new();
    let inner_calls = Rc::new(Cell::new(0_u32));
    let outer_calls = Rc::new(Cell::new(0_u32));
    let (inner_read, outer_read) = (inner_calls.clone(), outer_calls.clone());
    h.value("inner_calls", move || f64::from(inner_read.get()));
    h.value("outer_calls", move || f64::from(outer_read.get()));
    h.mount(move |_| {
        let inner_calls = inner_calls.clone();
        let outer_calls = outer_calls.clone();
        let inner_widget: Widget = KeyboardListener::new(probe(
            "key_target",
            SizedBox::new().width(40.).height(30.).child(colored(BLUE)),
        ))
        .focus_node(inner.clone())
        .autofocus(true)
        .on_key(move |_| {
            inner_calls.set(inner_calls.get() + 1);
            false
        })
        .into();
        KeyboardListener::new(inner_widget)
            .focus_node(outer.clone())
            .on_key(move |_| {
                outer_calls.set(outer_calls.get() + 1);
                true
            })
            .into()
    });
    h.frames(1);
    send_key(h, KeyboardKey::Character("k".into()), Code::KeyK);
    h.frames(1);
}

fn mouse_region_overlap(h: &mut FrameHarness, top_opaque: bool) {
    let under = Rc::new(Cell::new(0_u32));
    let top = Rc::new(Cell::new(0_u32));
    let (under_read, top_read) = (under.clone(), top.clone());
    h.value("under_enters", move || f64::from(under_read.get()));
    h.value("top_enters", move || f64::from(top_read.get()));
    h.mount(move |_| {
        let under_enters = under.clone();
        let top_enters = top.clone();
        let underlay: Widget = MouseRegion::new(probe(
            "under",
            SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
        ))
        .on_enter(move |_| under_enters.set(under_enters.get() + 1))
        .into();
        let overlay: Widget = MouseRegion::new(probe(
            "top",
            SizedBox::new()
                .width(40.)
                .height(30.)
                .child(colored(ORANGE)),
        ))
        .opaque(top_opaque)
        .on_enter(move |_| top_enters.set(top_enters.get() + 1))
        .into();
        SizedBox::new()
            .width(80.)
            .height(60.)
            .child(Stack::new([underlay, overlay]))
            .into()
    });
    h.hover(Offset::new(20., 20.));
    h.frames(1);
    h.hover(Offset::new(60., 50.));
    h.frames(1);
}

fn mouse_region_opaque_blocks_underlay(h: &mut FrameHarness) {
    mouse_region_overlap(h, true);
}

fn mouse_region_translucent_passes_underlay(h: &mut FrameHarness) {
    mouse_region_overlap(h, false);
}

fn mouse_region_nested_enter_order(h: &mut FrameHarness) {
    let enter_seq = Rc::new(Cell::new(0_f64));
    let exit_seq = Rc::new(Cell::new(0_f64));
    let (enter_read, exit_read) = (enter_seq.clone(), exit_seq.clone());
    h.value("enter_sequence", move || enter_read.get());
    h.value("exit_sequence", move || exit_read.get());
    h.mount(move |_| {
        let (inner_enter, inner_exit) = (enter_seq.clone(), exit_seq.clone());
        let (outer_enter, outer_exit) = (enter_seq.clone(), exit_seq.clone());
        let inner: Widget = MouseRegion::new(probe(
            "nested_target",
            SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
        ))
        .on_enter(move |_| inner_enter.set(inner_enter.get() * 10. + 2.))
        .on_exit(move |_| inner_exit.set(inner_exit.get() * 10. + 2.))
        .into();
        MouseRegion::new(inner)
            .on_enter(move |_| outer_enter.set(outer_enter.get() * 10. + 1.))
            .on_exit(move |_| outer_exit.set(outer_exit.get() * 10. + 1.))
            .into()
    });
    h.hover(Offset::new(20., 20.));
    h.frames(1);
    h.hover(Offset::new(100., 100.));
    h.frames(1);
}

fn tap_region_group_outside(h: &mut FrameHarness) {
    let counts: Vec<Rc<Cell<u32>>> = (0..6).map(|_| Rc::new(Cell::new(0_u32))).collect();
    let names = [
        "a_inside",
        "a_outside",
        "b_inside",
        "b_outside",
        "c_inside",
        "c_outside",
    ];
    for (name, count) in names.iter().zip(counts.iter()) {
        let read = count.clone();
        h.value(name, move || f64::from(read.get()));
    }
    h.mount(move |_| {
        let c = |index: usize| counts[index].clone();
        let region_a: Widget = TapRegion::new(probe(
            "region_a",
            SizedBox::new().width(40.).height(30.).child(colored(BLUE)),
        ))
        .group_id("g")
        .on_tap_inside({
            let count = c(0);
            move |_| count.set(count.get() + 1)
        })
        .on_tap_outside({
            let count = c(1);
            move |_| count.set(count.get() + 1)
        })
        .into();
        let region_b: Widget = TapRegion::new(probe(
            "region_b",
            SizedBox::new().width(40.).height(30.).child(colored(GREEN)),
        ))
        .group_id("g")
        .on_tap_inside({
            let count = c(2);
            move |_| count.set(count.get() + 1)
        })
        .on_tap_outside({
            let count = c(3);
            move |_| count.set(count.get() + 1)
        })
        .into();
        let region_c: Widget = TapRegion::new(probe(
            "region_c",
            SizedBox::new()
                .width(40.)
                .height(30.)
                .child(colored(ORANGE)),
        ))
        .on_tap_inside({
            let count = c(4);
            move |_| count.set(count.get() + 1)
        })
        .on_tap_outside({
            let count = c(5);
            move |_| count.set(count.get() + 1)
        })
        .into();
        TapRegionSurface::new(SizedBox::new().width(120.).height(60.).child(Stack::new([
            Widget::from(Positioned::new(region_a).top(0.).left(0.)),
            Widget::from(Positioned::new(region_b).top(0.).left(40.)),
            Widget::from(Positioned::new(region_c).top(0.).left(80.)),
        ])))
        .into()
    });
    h.down(Offset::new(20., 15.));
    h.up();
    h.frames(1);
    h.down(Offset::new(60., 15.));
    h.up();
    h.frames(1);
    h.down(Offset::new(100., 15.));
    h.up();
    h.frames(1);
}

fn arrow_directional_focus(h: &mut FrameHarness) {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let c = FocusNode::new();
    let (a_read, b_read, c_read) = (a.clone(), b.clone(), c.clone());
    h.value("a_focused", move || f64::from(a_read.has_focus()));
    h.value("b_focused", move || f64::from(b_read.has_focus()));
    h.value("c_focused", move || f64::from(c_read.has_focus()));
    h.mount(move |_| {
        SizedBox::new()
            .width(200.)
            .height(60.)
            .child(Row::new([
                Widget::from(
                    Focus::new(probe(
                        "a",
                        SizedBox::new().width(40.).height(30.).child(colored(BLUE)),
                    ))
                    .node(a.clone())
                    .autofocus(true),
                ),
                Widget::from(
                    Focus::new(probe(
                        "b",
                        SizedBox::new().width(40.).height(30.).child(colored(GREEN)),
                    ))
                    .node(b.clone()),
                ),
                Widget::from(
                    Focus::new(probe(
                        "c",
                        SizedBox::new()
                            .width(40.)
                            .height(30.)
                            .child(colored(ORANGE)),
                    ))
                    .node(c.clone()),
                ),
            ]))
            .into()
    });
    h.frames(1);
    send_key(
        h,
        KeyboardKey::Named(NamedKey::ArrowRight),
        Code::ArrowRight,
    );
    h.frames(1);
    send_key(
        h,
        KeyboardKey::Named(NamedKey::ArrowRight),
        Code::ArrowRight,
    );
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::ArrowLeft), Code::ArrowLeft);
    h.frames(1);
}

fn send_chord(
    h: &mut FrameHarness,
    modifier: KeyboardKey,
    modifier_code: Code,
    key: KeyboardKey,
    code: Code,
    modifiers: Modifiers,
) {
    h.send(InputEvent::Key(KeyboardEvent::key_down(
        modifier.clone(),
        modifier_code,
    )));
    let mut down = KeyboardEvent::key_down(key.clone(), code);
    down.modifiers = modifiers;
    let mut up = KeyboardEvent::key_up(key, code);
    up.modifiers = modifiers;
    h.send(InputEvent::Key(down));
    h.send(InputEvent::Key(up));
    h.send(InputEvent::Key(KeyboardEvent::key_up(
        modifier,
        modifier_code,
    )));
}

fn shift_tab_reverse_traversal(h: &mut FrameHarness) {
    let a = FocusNode::new();
    let b = FocusNode::new();
    let c = FocusNode::new();
    let (a_read, b_read, c_read) = (a.clone(), b.clone(), c.clone());
    h.value("a_focused", move || f64::from(a_read.has_focus()));
    h.value("b_focused", move || f64::from(b_read.has_focus()));
    h.value("c_focused", move || f64::from(c_read.has_focus()));
    h.mount(move |_| {
        FocusTraversalGroup::new(Row::new([
            Widget::from(
                Focus::new(probe(
                    "a",
                    SizedBox::new().width(40.).height(30.).child(colored(BLUE)),
                ))
                .node(a.clone())
                .autofocus(true),
            ),
            Widget::from(
                Focus::new(probe(
                    "b",
                    SizedBox::new().width(40.).height(30.).child(colored(GREEN)),
                ))
                .node(b.clone()),
            ),
            Widget::from(
                Focus::new(probe(
                    "c",
                    SizedBox::new()
                        .width(40.)
                        .height(30.)
                        .child(colored(ORANGE)),
                ))
                .node(c.clone()),
            ),
        ]))
        .into()
    });
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
    send_key(h, KeyboardKey::Named(NamedKey::Tab), Code::Tab);
    h.frames(1);
    send_chord(
        h,
        KeyboardKey::Named(NamedKey::Shift),
        Code::ShiftLeft,
        KeyboardKey::Named(NamedKey::Tab),
        Code::Tab,
        Modifiers::SHIFT,
    );
    h.frames(1);
}

fn shortcut_chord_requires_modifiers(h: &mut FrameHarness) {
    let ctrl = Rc::new(Cell::new(0_u32));
    let plain = Rc::new(Cell::new(0_u32));
    let (ctrl_read, plain_read) = (ctrl.clone(), plain.clone());
    h.value("ctrl_calls", move || f64::from(ctrl_read.get()));
    h.value("plain_calls", move || f64::from(plain_read.get()));
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Character("k".into()), Modifiers::CONTROL),
        Command::new("ctrl"),
    );
    shortcuts.bind_logical(
        LogicalShortcutKey::new(KeyboardKey::Character("k".into()), Modifiers::empty()),
        Command::new("plain"),
    );
    let mut actions = Actions::new();
    actions.register(Command::new("ctrl"), move || ctrl.set(ctrl.get() + 1));
    actions.register(Command::new("plain"), move || plain.set(plain.get() + 1));
    let shortcuts = Rc::new(shortcuts);
    let actions = Rc::new(actions);
    h.mount(move |_| {
        FocusableActionDetector::new(probe(
            "chord_target",
            SizedBox::new().width(40.).height(30.).child(colored(BLUE)),
        ))
        .autofocus(true)
        .shortcuts(shortcuts.clone())
        .actions(actions.clone())
        .into()
    });
    h.frames(1);
    send_chord(
        h,
        KeyboardKey::Named(NamedKey::Control),
        Code::ControlLeft,
        KeyboardKey::Character("k".into()),
        Code::KeyK,
        Modifiers::CONTROL,
    );
    h.frames(1);
    send_key(h, KeyboardKey::Character("k".into()), Code::KeyK);
    h.frames(1);
}
