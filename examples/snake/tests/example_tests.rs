#![allow(dead_code)]

#[path = "../main.rs"]
mod example;
mod game_logic;

use example::game::{Dir, Game, Outcome};
use incular::prelude::*;

fn key(code: Code) -> KeyboardEvent {
    KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), code)
}

fn radii(value: CornerRadii) -> [f32; 4] {
    [
        value.top_left,
        value.top_right,
        value.bottom_right,
        value.bottom_left,
    ]
}

#[test]
fn scenario_contract_is_valid() {
    example::example_support::assert_scenario(example::simulations::SCENARIO);
}

#[test]
fn all_eight_direction_keys_reach_the_game() {
    for (code, expected) in [
        (Code::ArrowUp, Dir::Up),
        (Code::KeyW, Dir::Up),
        (Code::ArrowDown, Dir::Down),
        (Code::KeyS, Dir::Down),
        (Code::ArrowLeft, Dir::Left),
        (Code::KeyA, Dir::Left),
        (Code::ArrowRight, Dir::Right),
        (Code::KeyD, Dir::Right),
    ] {
        let mut initial = Game::new(0);
        initial.dir = match expected {
            Dir::Up | Dir::Down => Dir::Right,
            Dir::Left | Dir::Right => Dir::Up,
        };
        initial.next_dir = initial.dir;
        let game = Signal::new(initial);
        assert!(example::handle_key(&game, &key(code)));
        assert_eq!(game.get().next_dir, expected);
    }
}

#[test]
fn repeat_keyup_and_unrelated_input_are_ignored() {
    let game = Signal::new(Game::new(0));
    let mut event = key(Code::ArrowUp);
    event.repeat = true;
    assert!(!example::handle_key(&game, &event));
    event.repeat = false;
    event.state = KeyState::Up;
    assert!(!example::handle_key(&game, &event));
    assert!(!example::handle_key(&game, &key(Code::KeyQ)));
    assert_eq!(game.get().next_dir, Dir::Right);
}

#[test]
fn space_restarts_only_after_the_game_ends() {
    let game = Signal::new(Game::new(0));
    let initial = game.get();
    assert!(example::handle_key(&game, &key(Code::Space)));
    assert_eq!(game.get(), initial);
    for outcome in [Outcome::Lost, Outcome::Won] {
        game.update(|game| {
            game.outcome = outcome;
            game.score = 42;
        });
        assert!(example::handle_key(&game, &key(Code::Space)));
        assert!(game.get().is_playing());
        assert_eq!(game.get().score, 0);
    }
}

#[test]
fn manual_steps_are_disabled_in_normal_play() {
    let game = Signal::new(example::simulations::initial_game());
    assert!(!example::simulations::handle_step(
        false,
        &game,
        &key(Code::F12)
    ));
    assert_eq!(game.get().snake[0], (10, 7));
    assert!(example::simulations::handle_step(
        true,
        &game,
        &key(Code::F12)
    ));
    assert_eq!(game.get().snake[0], (11, 7));
    assert_eq!(game.get().score, 1);
}

#[test]
fn board_keeps_half_width_and_square_cells() {
    let cell = example::view::cell_size(1000.0, 760.0);
    assert_eq!(cell, 25.0);
    assert_eq!(cell * 20.0, 500.0);
    assert_eq!(cell * 15.0, 375.0);
}

#[test]
fn board_fits_short_and_degenerate_windows() {
    let cell = example::view::cell_size(1000.0, 100.0);
    assert!((cell * 15.0 - 68.0).abs() < 0.001);
    assert_eq!(example::view::cell_size(0.0, 760.0), 0.0);
    assert_eq!(example::view::cell_size(1000.0, 0.0), 0.0);
}

#[test]
fn head_rounding_faces_the_direction_of_travel() {
    for (direction, expected) in [
        (Dir::Up, [12.0, 12.0, 0.0, 0.0]),
        (Dir::Down, [0.0, 0.0, 12.0, 12.0]),
        (Dir::Left, [12.0, 0.0, 0.0, 12.0]),
        (Dir::Right, [0.0, 12.0, 12.0, 0.0]),
    ] {
        let mut game = Game::new(0);
        game.dir = direction;
        assert_eq!(
            radii(example::view::segment_corners(&game, 0, 12.0)),
            expected
        );
    }
}

#[test]
fn tail_rounding_faces_away_from_the_previous_segment() {
    for (direction, expected) in [
        (Dir::Up, [12.0, 12.0, 0.0, 0.0]),
        (Dir::Down, [0.0, 0.0, 12.0, 12.0]),
        (Dir::Left, [12.0, 0.0, 0.0, 12.0]),
        (Dir::Right, [0.0, 12.0, 12.0, 0.0]),
    ] {
        let (dx, dy) = direction.delta();
        let mut game = Game::new(0);
        game.snake = vec![(5 - dx, 5 - dy), (5, 5), (5 + dx, 5 + dy)];
        assert_eq!(
            radii(example::view::segment_corners(&game, 2, 12.0)),
            expected
        );
    }
}

#[test]
fn all_eight_turns_round_the_outside_corner() {
    for (toward_head, toward_tail, expected) in [
        (Dir::Right, Dir::Down, [12.0, 0.0, 0.0, 0.0]),
        (Dir::Down, Dir::Right, [12.0, 0.0, 0.0, 0.0]),
        (Dir::Left, Dir::Down, [0.0, 12.0, 0.0, 0.0]),
        (Dir::Down, Dir::Left, [0.0, 12.0, 0.0, 0.0]),
        (Dir::Left, Dir::Up, [0.0, 0.0, 12.0, 0.0]),
        (Dir::Up, Dir::Left, [0.0, 0.0, 12.0, 0.0]),
        (Dir::Right, Dir::Up, [0.0, 0.0, 0.0, 12.0]),
        (Dir::Up, Dir::Right, [0.0, 0.0, 0.0, 12.0]),
    ] {
        let (hx, hy) = toward_head.delta();
        let (tx, ty) = toward_tail.delta();
        let mut game = Game::new(0);
        game.snake = vec![(5 + hx, 5 + hy), (5, 5), (5 + tx, 5 + ty)];
        assert_eq!(
            radii(example::view::segment_corners(&game, 1, 12.0)),
            expected
        );
    }
}

#[test]
fn straight_body_has_no_rounded_corners() {
    let game = Game::new(0);
    assert_eq!(
        radii(example::view::segment_corners(&game, 1, 12.0)),
        [0.0; 4]
    );
}

#[test]
fn body_palette_matches_the_original_hue_steps() {
    assert_eq!(example::view::body_color(1), Color::rgba(197, 82, 82, 255));
    assert_eq!(example::view::body_color(6), Color::rgba(197, 197, 82, 255));
    assert_eq!(example::view::body_color(1), example::view::body_color(31));
}

#[test]
fn paint_commands_can_be_built_without_a_gpu() {
    let mut game = Game::new(0);
    for outcome in [Outcome::Playing, Outcome::Lost, Outcome::Won] {
        game.outcome = outcome;
        let _display_list = example::view::artwork(&game, 25.0);
        let _widget = example::view::root(game.clone(), Size::new(1000.0, 760.0));
    }
}
