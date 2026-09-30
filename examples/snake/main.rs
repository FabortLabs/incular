//! Ply's Snake example, ported to Incular's native retained renderer.
//!
//! Run from the Incular workspace: cargo run -p incular --example snake
//! Reference: TheRedDeveloper/ply-website, interactive-examples/src/examples/snake.rs

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use incular::prelude::*;

pub(crate) mod game;
pub(crate) mod view;

#[path = "../tests/support/mod.rs"]
pub(crate) mod example_support;
#[path = "tests/simulations.rs"]
pub(crate) mod simulations;

use game::{Dir, Game, TICK_MILLIS};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manual_clock = simulations::enabled();
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let initial = if manual_clock {
        simulations::initial_game()
    } else {
        Game::new(seed)
    };

    // Keep state, focus identity, and the timer gate OUTSIDE the builder.
    // Signal reads subscribe the builder; completions mutate it on the UI
    // thread. An async task never owns or accesses a non-Send Signal.
    let game = Signal::new(initial);
    let tick_pending = Rc::new(Cell::new(false));
    let focus = FocusNode::new();
    let options = WindowOptions {
        initial_logical_size: Size::new(1000.0, 760.0),
        ..WindowOptions::new("Snake - Incular")
    };

    let app = Application::new_with_options(options, move |cx| {
        let snapshot = game.get();
        if !manual_clock && snapshot.is_playing() && !tick_pending.replace(true) {
            let after_tick = game.clone();
            let pending = tick_pending.clone();
            let _ = cx.spawn_into(
                async {
                    tokio::time::sleep(Duration::from_millis(TICK_MILLIS)).await;
                },
                move |result, _runtime| {
                    pending.set(false);
                    match result {
                        Ok(()) => after_tick.update(Game::tick),
                        Err(error) => eprintln!("Snake timer stopped: {error:?}"),
                    }
                },
            );
        }

        let keyboard_game = game.clone();
        KeyboardListener::new(view::root(snapshot, cx.viewport()))
            .focus_node(focus.clone())
            .autofocus(true)
            .on_key(move |event| {
                if simulations::handle_step(manual_clock, &keyboard_game, &event) {
                    return true;
                }
                handle_key(&keyboard_game, &event)
            })
            .into()
    })?;

    example_support::spawn_if_requested(app.simulation(), simulations::run);
    incular::run(app)?;
    Ok(())
}

pub(crate) fn handle_key(game: &Signal<Game>, event: &KeyboardEvent) -> bool {
    if event.state != KeyState::Down || event.repeat {
        return false;
    }
    let direction = match event.code {
        Code::ArrowUp | Code::KeyW => Some(Dir::Up),
        Code::ArrowDown | Code::KeyS => Some(Dir::Down),
        Code::ArrowLeft | Code::KeyA => Some(Dir::Left),
        Code::ArrowRight | Code::KeyD => Some(Dir::Right),
        _ => None,
    };
    if let Some(direction) = direction {
        game.update(|game| game.set_dir(direction));
        return true;
    }
    if event.code == Code::Space {
        if !game.get().is_playing() {
            game.update(Game::restart);
        }
        return true;
    }
    false
}
