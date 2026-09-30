//! Framework-independent rules for the Ply-style Snake game.

pub const GRID_W: i32 = 20;
pub const GRID_H: i32 = 15;
pub const GRID_CELLS: usize = (GRID_W * GRID_H) as usize;
pub const TICK_MILLIS: u64 = 120;
pub type Point = (i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const fn delta(self) -> Point {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn between(from: Point, to: Point) -> Option<Self> {
        match (to.0 - from.0, to.1 - from.1) {
            (0, -1) => Some(Self::Up),
            (0, 1) => Some(Self::Down),
            (-1, 0) => Some(Self::Left),
            (1, 0) => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Lost,
    Won,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    /// Head first, tail last. Consecutive points are orthogonally adjacent.
    pub snake: Vec<Point>,
    pub dir: Dir,
    pub next_dir: Dir,
    /// None only after filling the board.
    pub food: Option<Point>,
    pub score: u32,
    pub outcome: Outcome,
    rng: u64,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let (cx, cy) = (GRID_W / 2, GRID_H / 2);
        let mut game = Self {
            snake: vec![(cx, cy), (cx - 1, cy), (cx - 2, cy)],
            dir: Dir::Right,
            next_dir: Dir::Right,
            food: None,
            score: 0,
            outcome: Outcome::Playing,
            rng: seed,
        };
        game.spawn_food();
        game
    }

    pub fn is_playing(&self) -> bool {
        self.outcome == Outcome::Playing
    }

    pub fn set_dir(&mut self, direction: Dir) {
        // Compare with the last executed direction, not a queued turn. This
        // preserves the source's last-valid-input-wins behavior and prevents
        // a rapid pair of inputs from reversing into the neck in one tick.
        if self.is_playing() && direction != self.dir.opposite() {
            self.next_dir = direction;
        }
    }

    pub fn tick(&mut self) {
        if !self.is_playing() {
            return;
        }

        self.dir = self.next_dir;
        let (dx, dy) = self.dir.delta();
        let (hx, hy) = self.snake[0];
        let next = (hx + dx, hy + dy);

        // Deliberately match Ply: every currently occupied cell, including
        // the current tail cell, counts as a collision.
        if !(0..GRID_W).contains(&next.0)
            || !(0..GRID_H).contains(&next.1)
            || self.snake.contains(&next)
        {
            self.outcome = Outcome::Lost;
            return;
        }

        self.snake.insert(0, next);
        if self.food == Some(next) {
            self.score += 1;
            self.spawn_food();
        } else {
            self.snake.pop();
        }
    }

    pub fn restart(&mut self) {
        // Continue the random sequence rather than restarting with identical
        // food every time. Timing belongs to the application, not the model.
        let seed = self.next_random();
        *self = Self::new(seed);
    }

    fn spawn_food(&mut self) {
        let mut occupied = [false; GRID_CELLS];
        for &(x, y) in &self.snake {
            occupied[(y * GRID_W + x) as usize] = true;
        }
        let free: Vec<Point> = (0..GRID_CELLS)
            .filter(|&index| !occupied[index])
            .map(|index| (index as i32 % GRID_W, index as i32 / GRID_W))
            .collect();

        // Unlike retrying random coordinates forever, this terminates even
        // when there are no free cells (the original example's edge case).
        if free.is_empty() {
            self.food = None;
            self.outcome = Outcome::Won;
            return;
        }
        let index = (self.next_random() % free.len() as u64) as usize;
        self.food = Some(free[index]);
    }

    fn next_random(&mut self) -> u64 {
        // SplitMix64: deterministic for tests; no extra crate is needed.
        // This is game randomness, not a cryptographic random generator.
        self.rng = self.rng.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.rng;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}
