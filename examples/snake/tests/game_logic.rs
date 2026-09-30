//! Model tests sharing the application's rules module.

#![allow(dead_code)]

use super::example::game;

use game::{Dir, GRID_CELLS, GRID_H, GRID_W, Game, Outcome, Point, TICK_MILLIS};

fn fixture(snake: &[Point], direction: Dir) -> Game {
    let mut game = Game::new(42);
    game.snake = snake.to_vec();
    game.dir = direction;
    game.next_dir = direction;
    game.food = Some((19, 14));
    game.score = snake.len().saturating_sub(3) as u32;
    game
}

fn serpentine_cells() -> Vec<Point> {
    (0..GRID_H)
        .flat_map(|y| {
            (0..GRID_W).map(move |x| {
                let x = if y % 2 == 0 { x } else { GRID_W - 1 - x };
                (x, y)
            })
        })
        .collect()
}

#[test]
fn original_dimensions_speed_and_initial_state_are_preserved() {
    assert_eq!((GRID_W, GRID_H, TICK_MILLIS), (20, 15, 120));
    let game = Game::new(0);
    assert_eq!(game.snake, [(10, 7), (9, 7), (8, 7)]);
    assert_eq!(game.dir, Dir::Right);
    assert_eq!(game.next_dir, Dir::Right);
    assert_eq!(game.score, 0);
    assert!(game.is_playing());
}

#[test]
fn initial_food_is_in_bounds_and_outside_the_body() {
    for seed in 0..1024 {
        let game = Game::new(seed);
        let food = game.food.expect("a new board has free cells");
        assert!((0..GRID_W).contains(&food.0));
        assert!((0..GRID_H).contains(&food.1));
        assert!(!game.snake.contains(&food));
    }
}

#[test]
fn identical_seeds_produce_identical_games() {
    for seed in [0, 1, 42, u64::MAX] {
        assert_eq!(Game::new(seed), Game::new(seed));
    }
}

#[test]
fn ordinary_tick_moves_one_cell_without_growing() {
    let mut game = fixture(&[(10, 7), (9, 7), (8, 7)], Dir::Right);
    game.tick();
    assert_eq!(game.snake, [(11, 7), (10, 7), (9, 7)]);
    assert_eq!(game.score, 0);
    assert!(game.is_playing());
}

#[test]
fn eating_grows_and_updates_score_and_food() {
    let mut game = fixture(&[(10, 7), (9, 7), (8, 7)], Dir::Right);
    game.food = Some((11, 7));
    game.tick();
    assert_eq!(game.snake, [(11, 7), (10, 7), (9, 7), (8, 7)]);
    assert_eq!(game.score, 1);
    assert!(!game.snake.contains(&game.food.unwrap()));
}

#[test]
fn repeated_pickups_do_not_drop_the_tail() {
    let mut game = fixture(&[(10, 7), (9, 7), (8, 7)], Dir::Right);
    for x in 11..=15 {
        game.food = Some((x, 7));
        game.tick();
        assert_eq!(game.snake[0], (x, 7));
        assert_eq!(game.snake.last(), Some(&(8, 7)));
        assert_eq!(game.score, (x - 10) as u32);
        assert_eq!(game.snake.len(), 3 + (x - 10) as usize);
    }
}

#[test]
fn turns_are_applied_only_on_ticks() {
    let mut game = fixture(&[(10, 7), (9, 7), (8, 7)], Dir::Right);
    game.set_dir(Dir::Up);
    assert_eq!(game.dir, Dir::Right);
    assert_eq!(game.next_dir, Dir::Up);
    game.tick();
    assert_eq!(game.dir, Dir::Up);
    assert_eq!(game.snake[0], (10, 6));
}

#[test]
fn direct_reverse_is_rejected() {
    for direction in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
        let mut game = Game::new(0);
        game.dir = direction;
        game.next_dir = direction;
        game.set_dir(direction.opposite());
        assert_eq!(game.next_dir, direction);
    }
}

#[test]
fn rapid_turns_cannot_reverse_before_a_tick() {
    let mut game = fixture(&[(10, 7), (9, 7), (8, 7)], Dir::Right);
    game.set_dir(Dir::Up);
    game.set_dir(Dir::Left);
    game.tick();
    assert_eq!(game.dir, Dir::Up);
    assert_eq!(game.snake[0], (10, 6));
}

#[test]
fn last_valid_turn_wins_like_the_original() {
    let mut game = fixture(&[(10, 7), (9, 7), (8, 7)], Dir::Right);
    game.set_dir(Dir::Up);
    game.set_dir(Dir::Down);
    game.tick();
    assert_eq!(game.snake[0], (10, 8));
}

#[test]
fn all_four_walls_end_the_game_without_moving_out_of_bounds() {
    let cases = [
        ([(0, 3), (1, 3), (2, 3)], Dir::Left),
        ([(19, 3), (18, 3), (17, 3)], Dir::Right),
        ([(3, 0), (3, 1), (3, 2)], Dir::Up),
        ([(3, 14), (3, 13), (3, 12)], Dir::Down),
    ];
    for (snake, direction) in cases {
        let mut game = fixture(&snake, direction);
        game.tick();
        assert_eq!(game.outcome, Outcome::Lost);
        assert_eq!(game.snake, snake);
    }
}

#[test]
fn body_collision_ends_the_game() {
    let mut game = fixture(&[(2, 2), (2, 3), (3, 3), (3, 2), (3, 1)], Dir::Up);
    game.set_dir(Dir::Right);
    game.tick();
    assert_eq!(game.outcome, Outcome::Lost);
    assert_eq!(game.snake[0], (2, 2));
}

#[test]
fn current_tail_collision_matches_ply() {
    let mut game = fixture(&[(2, 2), (2, 3), (1, 3), (1, 2)], Dir::Up);
    game.set_dir(Dir::Left);
    game.tick();
    assert_eq!(game.outcome, Outcome::Lost);
}

#[test]
fn terminal_states_ignore_ticks_and_direction_changes() {
    for outcome in [Outcome::Lost, Outcome::Won] {
        let mut game = Game::new(7);
        game.outcome = outcome;
        let before = game.clone();
        game.tick();
        game.set_dir(Dir::Down);
        assert_eq!(game, before);
    }
}

#[test]
fn restart_clears_terminal_state_score_and_body() {
    let mut game = fixture(&[(19, 7), (18, 7), (17, 7), (16, 7)], Dir::Right);
    game.tick();
    assert_eq!(game.outcome, Outcome::Lost);
    game.restart();
    assert_eq!(game.snake, [(10, 7), (9, 7), (8, 7)]);
    assert_eq!(game.score, 0);
    assert_eq!(game.dir, Dir::Right);
    assert_eq!(game.next_dir, Dir::Right);
    assert!(game.is_playing());
    assert!(!game.snake.contains(&game.food.unwrap()));
}

#[test]
fn last_free_cell_wins_instead_of_looping_forever() {
    let cells = serpentine_cells();
    let snake: Vec<Point> = cells[..GRID_CELLS - 1].iter().rev().copied().collect();
    let mut game = fixture(&snake, Dir::Right);
    game.food = Some(cells[GRID_CELLS - 1]);
    game.tick();
    assert_eq!(game.outcome, Outcome::Won);
    assert_eq!(game.snake.len(), GRID_CELLS);
    assert_eq!(game.score, (GRID_CELLS - 3) as u32);
    assert_eq!(game.food, None);
    let before = game.clone();
    game.tick();
    assert_eq!(game, before);
}

#[test]
fn food_spawning_with_one_free_cell_always_terminates() {
    let cells = serpentine_cells();
    let snake: Vec<Point> = cells[..GRID_CELLS - 2].iter().rev().copied().collect();
    let mut game = fixture(&snake, Dir::Right);
    game.food = Some(cells[GRID_CELLS - 2]);
    game.tick();
    assert!(game.is_playing());
    assert_eq!(game.food, Some(cells[GRID_CELLS - 1]));
    game.tick();
    assert_eq!(game.outcome, Outcome::Won);
}

#[test]
fn direction_geometry_handles_adjacent_and_invalid_points() {
    for direction in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
        let (dx, dy) = direction.delta();
        assert_eq!(Dir::between((5, 5), (5 + dx, 5 + dy)), Some(direction));
        assert_eq!(direction.opposite().opposite(), direction);
    }
    assert_eq!(Dir::between((0, 0), (1, 1)), None);
    assert_eq!(Dir::between((0, 0), (0, 0)), None);
    assert_eq!(Dir::between((0, 0), (2, 0)), None);
}
