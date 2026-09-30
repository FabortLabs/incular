//! Opt-in deterministic native-input simulation. The real-time timer is
//! disabled only under INCULAR_EXAMPLE_SIMULATION; F12 then advances a tick.

use incular::{prelude::*, testing::Simulation as AppSimulation};

use super::{
    example_support::{self, ExampleScenario},
    game::Game,
};

pub const SCENARIO: ExampleScenario =
    ExampleScenario::new("snake", &[], &[(500.0, 380.0)], None, None);

pub fn enabled() -> bool {
    std::env::var_os(example_support::SIMULATION_ENV).is_some()
}

pub fn initial_game() -> Game {
    let mut game = Game::new(0x0053_4e41_4b45);
    // One deterministic food pickup through the actual keyboard/event path.
    game.food = Some((11, 7));
    game
}

pub fn handle_step(manual_clock: bool, game: &Signal<Game>, event: &KeyboardEvent) -> bool {
    if manual_clock && event.code == Code::F12 && event.state == KeyState::Down && !event.repeat {
        game.update(Game::tick);
        true
    } else {
        false
    }
}

fn steps(
    simulation: &AppSimulation,
    count: usize,
) -> Result<(), incular::testing::SimulationError> {
    for _ in 0..count {
        simulation.press(Code::F12)?;
        example_support::wait_for_settled_frame(simulation)?;
    }
    Ok(())
}

pub fn run(simulation: AppSimulation) {
    example_support::run_custom(simulation, SCENARIO, |simulation| {
        steps(simulation, 1)?;
        example_support::capture_to_disk(simulation, SCENARIO.name, "food-eaten")?;

        simulation.press(Code::ArrowUp)?;
        steps(simulation, 3)?;
        simulation.press(Code::KeyA)?;
        steps(simulation, 4)?;
        simulation.press(Code::ArrowDown)?;
        steps(simulation, 2)?;
        simulation.press(Code::KeyD)?;
        steps(simulation, 2)?;
        example_support::capture_to_disk(simulation, SCENARIO.name, "turned")?;

        // Force a wall collision and capture the terminal presentation,
        // then exercise Space through the same listener as native input.
        steps(simulation, 20)?;
        example_support::capture_to_disk(simulation, SCENARIO.name, "game-over")?;
        simulation.press(Code::Space)?;
        example_support::wait_for_settled_frame(simulation)?;
        Ok(())
    });
}
