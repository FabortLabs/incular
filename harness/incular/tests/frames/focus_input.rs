use std::{cell::Cell, rc::Rc};

use incular_core::Modifiers;
use incular_core::{Code, Color, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    AbsorbPointer, Actions, ColoredBox, Command, Focus, FocusNode, FocusScope,
    FocusableActionDetector, GestureDetector, IgnorePointer, Listener, LogicalShortcutKey,
    MouseRegion, Positioned, ScrollController, Shortcuts, SizedBox, Stack, Widget,
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
