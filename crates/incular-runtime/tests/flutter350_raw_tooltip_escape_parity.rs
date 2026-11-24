//! Runtime Escape handling for Flutter 3.50.0 RawTooltip.
use incular_config::Constraints;
use incular_core::{
    Code, Color, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Offset,
    PointerDeviceKind, PointerPhase, PointerSampleMetadata, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Actions, Center, Column, Command, FocusNode, KeyboardListener, RawTooltip,
    RawTooltipController, ShortcutKey, Shortcuts, Spacer, TooltipComponentBuilder,
    TooltipTriggerMode, Widget, WidgetsApp,
};
use std::{cell::Cell, rc::Rc, time::Duration};

const SURFACE: Size = Size {
    width: 800.,
    height: 600.,
};
fn tooltip(controller: &RawTooltipController, mode: TooltipTriggerMode) -> Widget {
    RawTooltip::with_builder(
        Widget::box_(Size::new(100., 100.), Color::WHITE),
        TooltipComponentBuilder::new(|| Widget::box_(Size::new(40., 20.), Color::BLACK)),
    )
    .controller(controller.clone())
    .trigger_mode(mode)
    .hover_delay(Duration::ZERO)
    .touch_delay(Duration::from_secs(24 * 60 * 60))
    .into()
}

fn mount(widget: impl Into<Widget>, now: Instant) -> Runtime {
    let mut runtime = Runtime::new(WidgetsApp::new(widget).into()).unwrap();
    frame(&mut runtime, now);
    runtime
}

fn frame(runtime: &mut Runtime, now: Instant) -> usize {
    runtime
        .run_frame_at(Constraints::tight(SURFACE), now)
        .unwrap();
    runtime.tree().transient_surfaces().len()
}

fn key(runtime: &mut Runtime, named: NamedKey, code: Code, now: Instant) {
    let logical = KeyboardKey::Named(named);
    for event in [
        KeyboardEvent::key_down(logical.clone(), code),
        KeyboardEvent::key_up(logical, code),
    ] {
        runtime.handle_input_at(InputEvent::Key(event), now);
    }
}

fn move_mouse(runtime: &mut Runtime, position: Offset, now: Instant) {
    runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind: PointerDeviceKind::Mouse,
            buttons: 0,
            button: None,
            sample: PointerSampleMetadata::EMPTY,
            phase: PointerPhase::Move,
            position,
        },
        now,
    );
}

#[test]
fn escape_closes_a_hovered_tooltip_without_mouse_exit() {
    let start = Instant::now();
    let at = |ms| start + Duration::from_millis(ms);
    let tip = RawTooltipController::new();
    let mut runtime = mount(
        Center::new(tooltip(&tip, TooltipTriggerMode::LongPress)),
        start,
    );
    move_mouse(&mut runtime, Offset::new(0., 0.), at(10));
    frame(&mut runtime, at(20));
    move_mouse(&mut runtime, Offset::new(400., 300.), at(30));
    frame(&mut runtime, at(40));
    assert_eq!(frame(&mut runtime, at(60)), 1, "hovered before any key");
    key(&mut runtime, NamedKey::Escape, Code::Escape, at(70));
    let actual = frame(&mut runtime, at(570));
    tip.hide_at(at(580));
    assert_eq!(frame(&mut runtime, at(590)), 0, "explicit hide control");
    assert_eq!(actual, 0);
}

#[test]
fn escape_closes_both_programmatically_shown_tooltips() {
    let start = Instant::now();
    let at = |ms| start + Duration::from_millis(ms);
    let first = RawTooltipController::new();
    let second = RawTooltipController::new();
    let mut runtime = mount(
        Column::new([
            tooltip(&first, TooltipTriggerMode::LongPress),
            Spacer::new().into(),
            tooltip(&second, TooltipTriggerMode::LongPress),
        ]),
        start,
    );
    first.show_at(at(10));
    second.show_at(at(10));
    assert_eq!(frame(&mut runtime, at(20)), 2);
    key(&mut runtime, NamedKey::Escape, Code::Escape, at(30));
    let actual = frame(&mut runtime, at(530));
    first.hide_at(at(540));
    second.hide_at(at(540));
    assert_eq!(frame(&mut runtime, at(550)), 0, "explicit hide control");
    assert_eq!(actual, 0);
}

#[test]
fn open_tooltip_takes_first_escape_before_ancestor_dismiss_shortcut() {
    let start = Instant::now();
    let at = |ms| start + Duration::from_millis(ms);
    let tip = RawTooltipController::new();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let mut actions = Actions::new();
    let command = Command::new("test-ancestor-dismiss");
    actions.register(command.clone(), move || observed.set(observed.get() + 1));
    let mut shortcuts = Shortcuts::new();
    shortcuts.bind(ShortcutKey::new(Code::Escape, Modifiers::empty()), command);
    let focus = FocusNode::new();
    let parent = KeyboardListener::new(tooltip(&tip, TooltipTriggerMode::LongPress))
        .focus_node(focus.clone())
        .autofocus(true)
        .with_shortcuts(Rc::new(shortcuts), Rc::new(actions));
    let mut runtime = mount(parent, start);
    assert!(
        runtime.focused_element().is_some(),
        "shortcut scope has focus"
    );

    tip.show_at(at(10));
    assert_eq!(frame(&mut runtime, at(20)), 1);
    key(&mut runtime, NamedKey::Escape, Code::Escape, at(30));
    let first = (frame(&mut runtime, at(530)), calls.get());
    key(&mut runtime, NamedKey::Escape, Code::Escape, at(540));
    let second = (frame(&mut runtime, at(1040)), calls.get());

    assert_eq!([first, second], [(0, 0), (0, 1)]);
}
