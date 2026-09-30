//! Native Incular painting and overlays; no Ply, Macroquad, HTML, or images.

use incular::prelude::*;

use super::game::{Dir, GRID_CELLS, GRID_H, GRID_W, Game, Outcome};

pub const BACKGROUND: Color = Color::rgba(30, 27, 27, 255);
const BOARD_DARK: Color = Color::rgba(38, 34, 32, 255);
const BOARD_LIGHT: Color = Color::rgba(42, 39, 37, 255);
const HEAD: Color = Color::rgba(110, 203, 99, 255);
const FOOD: Color = Color::rgba(255, 101, 77, 255);
const TEXT: Color = Color::rgba(232, 224, 220, 255);
const MUTED: Color = Color::rgba(158, 149, 144, 255);
const HUE_STEP: f32 = 12.0;
const CORNER: f32 = 12.0;

pub fn root(game: Game, viewport: Size) -> Widget {
    let width = finite_extent(viewport.width, 1000.0);
    let height = finite_extent(viewport.height, 760.0);
    let cell = cell_size(width, height);
    let child: Widget = if cell > 0.0 {
        board(&game, cell)
    } else {
        SizedBox::shrink().into()
    };
    Container::new()
        .width(width)
        .height(height)
        .background(BACKGROUND)
        .alignment(Alignment::CENTER)
        .child(child)
        .into()
}

fn finite_extent(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}

pub(crate) fn cell_size(width: f32, height: f32) -> f32 {
    // Match Ply's 50%-width board; also fit unusually short windows. All
    // coordinates use logical pixels, so the native host handles DPI.
    (width.max(0.0) * 0.5 / GRID_W as f32).min((height - 32.0).max(0.0) / GRID_H as f32)
}

fn board(game: &Game, cell: f32) -> Widget {
    let size = Size::new(GRID_W as f32 * cell, GRID_H as f32 * cell);
    // Keep the overlays inside the board even when the window is very small.
    let scale = (size.width / 240.0).min(1.0);
    let mut layers: Vec<Widget> = vec![
        RepaintBoundary::new(CustomPaint::new(size, artwork(game, cell))).into(),
        Positioned::new(
            Center::new(badge(
                Text::new(game.score.to_string()).style(text_style(22.0 * scale, TEXT)),
                14.0 * scale,
                4.0 * scale,
                12.0 * scale,
                200,
            ))
            .height_factor(1.0),
        )
        .left(0.0)
        .right(0.0)
        .top(8.0 * scale)
        .into(),
    ];

    if game.is_playing() {
        layers.push(
            Positioned::new(
                Center::new(badge(
                    Text::new("Arrow keys or WASD")
                        .style(text_style(12.0 * scale, Color::rgba(110, 101, 96, 255))),
                    10.0 * scale,
                    4.0 * scale,
                    8.0 * scale,
                    160,
                ))
                .height_factor(1.0),
            )
            .left(0.0)
            .right(0.0)
            .bottom(8.0 * scale)
            .into(),
        );
    } else {
        let (title, color) = match game.outcome {
            Outcome::Won => ("You Win!", HEAD),
            _ => ("Game Over", FOOD),
        };
        let lines = Column::new(vec![
            Text::new(title).style(text_style(22.0 * scale, color)),
            Text::new("Press Space to restart").style(text_style(14.0 * scale, MUTED)),
        ])
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .spacing(4.0 * scale);
        layers.push(badge(lines, 20.0 * scale, 12.0 * scale, 8.0 * scale, 220));
    }

    // Overlays are display-only; the enclosing keyboard listener owns input.
    SizedBox::from_size(size)
        .child(Stack::new(layers).alignment(Alignment::CENTER))
        .into()
}

fn badge(
    child: impl Into<Widget>,
    horizontal: f32,
    vertical: f32,
    radius: f32,
    alpha: u8,
) -> Widget {
    DecoratedBox::new(Padding::symmetric(horizontal, vertical, child))
        .background(Color::rgba(30, 27, 27, alpha))
        .radius(radius)
        .into()
}

fn text_style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        color,
        ..TextStyle::default()
    }
}

pub(crate) fn artwork(game: &Game, cell: f32) -> DisplayList {
    let size = Size::new(GRID_W as f32 * cell, GRID_H as f32 * cell);
    let bounds = Rect::from_origin_size(Offset::new(0.0, 0.0), size);
    let mut canvas = Canvas::default();
    canvas.save_clip_rrect(RRect::uniform(bounds, 4.0));
    canvas.rect(bounds, BOARD_DARK);

    // O(board area + snake length), rather than searching the snake for
    // every square. The rounded cutouts expose the same dark base as Ply.
    let mut cells = [None; GRID_CELLS];
    for (index, &(x, y)) in game.snake.iter().enumerate() {
        cells[(y * GRID_W + x) as usize] = Some(index);
    }
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let index = cells[(y * GRID_W + x) as usize];
            let (color, radii) = match index {
                Some(0) => (HEAD, segment_corners(game, 0, CORNER)),
                Some(index) => (body_color(index), segment_corners(game, index, CORNER)),
                None if game.food == Some((x, y)) => (FOOD, CornerRadii::uniform(cell * 0.5)),
                None if (x + y) % 2 == 0 => (BOARD_LIGHT, CornerRadii::ZERO),
                None => (BOARD_DARK, CornerRadii::ZERO),
            };
            let square = Rect::from_origin_size(
                Offset::new(x as f32 * cell, y as f32 * cell),
                Size::new(cell, cell),
            );
            canvas.rrect(RRect::new(square, radii), color);
        }
    }
    canvas.restore();
    canvas.finish()
}

pub(crate) fn segment_corners(game: &Game, index: usize, radius: f32) -> CornerRadii {
    if index == 0 {
        return cap_corners(game.dir, radius);
    }
    if index + 1 == game.snake.len() {
        return Dir::between(game.snake[index - 1], game.snake[index])
            .map_or(CornerRadii::ZERO, |dir| cap_corners(dir, radius));
    }

    let position = game.snake[index];
    match (
        Dir::between(position, game.snake[index - 1]),
        Dir::between(position, game.snake[index + 1]),
    ) {
        (Some(Dir::Right), Some(Dir::Down)) | (Some(Dir::Down), Some(Dir::Right)) => {
            corners(radius, 0.0, 0.0, 0.0)
        }
        (Some(Dir::Left), Some(Dir::Down)) | (Some(Dir::Down), Some(Dir::Left)) => {
            corners(0.0, radius, 0.0, 0.0)
        }
        (Some(Dir::Left), Some(Dir::Up)) | (Some(Dir::Up), Some(Dir::Left)) => {
            corners(0.0, 0.0, radius, 0.0)
        }
        (Some(Dir::Right), Some(Dir::Up)) | (Some(Dir::Up), Some(Dir::Right)) => {
            corners(0.0, 0.0, 0.0, radius)
        }
        _ => CornerRadii::ZERO,
    }
}

fn cap_corners(direction: Dir, radius: f32) -> CornerRadii {
    match direction {
        Dir::Up => corners(radius, radius, 0.0, 0.0),
        Dir::Down => corners(0.0, 0.0, radius, radius),
        Dir::Left => corners(radius, 0.0, 0.0, radius),
        Dir::Right => corners(0.0, radius, radius, 0.0),
    }
}

fn corners(tl: f32, tr: f32, br: f32, bl: f32) -> CornerRadii {
    CornerRadii {
        top_left: tl,
        top_right: tr,
        bottom_right: br,
        bottom_left: bl,
    }
}

pub(crate) fn body_color(index: usize) -> Color {
    let hue = ((index.saturating_sub(1)) as f32 * HUE_STEP).rem_euclid(360.0);
    let saturation = 0.5_f32;
    let lightness = 0.55_f32;
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let x = chroma * (1.0 - (hue / 60.0 % 2.0 - 1.0).abs());
    let (r, g, b) = match (hue / 60.0) as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma * 0.5;
    Color::rgba(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
        255,
    )
}
