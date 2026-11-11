//! Clicking a button must not take keyboard focus from the node that holds
//! it, and a node can take focus back programmatically. Flutter buttons and
//! `InkWell`s do not request focus on tap, so keyboard shortcuts owned by
//! another focus node (a media player's Space) keep working after a click.

use incular_config::Constraints;
use incular_core::{
    Code, Color, InputEvent, KeyState, KeyboardEvent, KeyboardKey, Location, Modifiers, Offset,
    PointerPhase, Size,
};
use incular_runtime::Runtime;
use incular_widgets::{Column, FocusNode, KeyboardListener, Widget, internal::ActionSurface};
use std::{cell::Cell, rc::Rc};

fn space(state: KeyState) -> InputEvent {
    InputEvent::Key(KeyboardEvent {
        state,
        key: KeyboardKey::Character(" ".into()),
        code: Code::Space,
        location: Location::Standard,
        modifiers: Modifiers::default(),
        repeat: false,
        is_composing: false,
    })
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(200., 200.)))
        .expect("frame");
}

fn click(runtime: &mut Runtime, position: Offset) {
    for phase in [PointerPhase::Move, PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
        frame(runtime);
    }
}

#[test]
fn clicking_a_button_keeps_focus_on_the_focused_node() {
    let player = FocusNode::new();
    let toggles = Rc::new(Cell::new(0));
    let presses = Rc::new(Cell::new(0));
    let root = Column::new([
        Widget::from(
            KeyboardListener::new(Widget::box_(Size::new(200., 100.), Color::BLACK))
                .focus_node(player.clone())
                .autofocus(true)
                .on_key_down({
                    let toggles = toggles.clone();
                    move |event| {
                        if event.code == Code::Space {
                            toggles.set(toggles.get() + 1);
                        }
                    }
                }),
        ),
        ActionSurface::new("Next")
            .size(Size::new(200., 40.))
            .on_press({
                let presses = presses.clone();
                move || presses.set(presses.get() + 1)
            })
            .into(),
    ]);
    let mut runtime = Runtime::new(
        incular_widgets::Directionality::new(incular_widgets::TextDirection::Ltr, root).into(),
    )
    .expect("mount");
    frame(&mut runtime);
    assert!(player.has_focus(), "autofocus");

    click(&mut runtime, Offset::new(100., 120.));
    assert_eq!(presses.get(), 1, "the click activates the button");
    assert!(
        player.has_focus(),
        "a click must not move focus to the button"
    );

    let _ = runtime.handle_input(space(KeyState::Down));
    let _ = runtime.handle_input(space(KeyState::Up));
    frame(&mut runtime);
    assert_eq!(toggles.get(), 1, "Space reaches the focused player");
    assert_eq!(
        presses.get(),
        1,
        "Space must not re-activate the clicked button"
    );
}

#[test]
fn focus_node_request_takes_focus_back_from_a_button() {
    let player = FocusNode::new();
    let button = FocusNode::new();
    let root = Column::new([
        Widget::from(
            KeyboardListener::new(Widget::box_(Size::new(200., 100.), Color::BLACK))
                .focus_node(player.clone()),
        ),
        incular_widgets::Focus::new(Widget::box_(Size::new(200., 40.), Color::WHITE))
            .node(button.clone())
            .into(),
    ]);
    let mut runtime = Runtime::new(
        incular_widgets::Directionality::new(incular_widgets::TextDirection::Ltr, root).into(),
    )
    .expect("mount");
    frame(&mut runtime);
    button.request_focus();
    frame(&mut runtime);
    assert!(button.has_focus());
    player.request_focus();
    frame(&mut runtime);
    assert!(player.has_focus(), "request_focus hands focus back");
    assert!(!button.has_focus());
}
