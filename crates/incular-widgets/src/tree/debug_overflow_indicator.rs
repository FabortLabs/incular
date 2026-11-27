//! Flutter-style debug overlays for render boxes that overflow.

use std::sync::OnceLock;

use incular_rendering::{GradientStop, GradientStops, LinearGradient, TileMode};
use incular_text::FontWeight;

use super::flex_overflow::overflow_pixels_text;
use super::*;

const FRACTION: f32 = 0.1;
const FONT_SIZE: f32 = 7.5;
const LABEL_PADDING: f32 = 1.0;

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn label(self) -> &'static str {
        match self {
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
            Self::Top => "TOP",
            Self::Bottom => "BOTTOM",
        }
    }

    fn marker(self, size: Size) -> (Rect, Offset, f32) {
        let stripe_width = size.width * FRACTION;
        let stripe_height = size.height * FRACTION;
        let label_inset = FONT_SIZE + LABEL_PADDING;
        let (origin, stripe, label, angle) = match self {
            Self::Left => (
                Offset::ZERO,
                Size::new(stripe_width, size.height),
                Offset::new(label_inset, size.height * 0.5),
                std::f32::consts::FRAC_PI_2,
            ),
            Self::Right => (
                Offset::new(size.width * (1. - FRACTION), 0.),
                Size::new(stripe_width, size.height),
                Offset::new(size.width - label_inset, size.height * 0.5),
                -std::f32::consts::FRAC_PI_2,
            ),
            Self::Top => (
                Offset::ZERO,
                Size::new(size.width, stripe_height),
                Offset::new(size.width * 0.5, LABEL_PADDING),
                0.,
            ),
            Self::Bottom => (
                Offset::new(0., size.height * (1. - FRACTION)),
                Size::new(size.width, stripe_height),
                Offset::new(size.width * 0.5, size.height - label_inset),
                0.,
            ),
        };
        (Rect::from_origin_size(origin, stripe), label, angle)
    }
}

fn stripe_brush() -> Brush {
    static GRADIENT: OnceLock<LinearGradient> = OnceLock::new();
    Brush::LinearGradient(
        GRADIENT
            .get_or_init(|| {
                let black = Color::rgba(0, 0, 0, 191);
                let yellow = Color::rgba(255, 255, 0, 191);
                let stops = GradientStops::new(vec![
                    GradientStop {
                        offset: 0.25,
                        color: black,
                    },
                    GradientStop {
                        offset: 0.25,
                        color: yellow,
                    },
                    GradientStop {
                        offset: 0.75,
                        color: yellow,
                    },
                    GradientStop {
                        offset: 0.75,
                        color: black,
                    },
                ]);
                let mut gradient = LinearGradient::new(Offset::ZERO, Offset::new(10., 10.), stops);
                gradient.tile_mode = TileMode::Repeated;
                gradient
            })
            .clone(),
    )
}

impl WidgetTree {
    /// Flutter paints diagnostics above the overflowing children. Record
    /// the exact physical sides; no marker should exist on a zero-area box.
    pub(super) fn paint_debug_overflow_indicator(
        &mut self,
        id: RenderObjectId,
        kind: &RenderKind,
        size: Size,
        out: &mut DisplayList,
    ) {
        if size.width <= 0. || size.height <= 0. {
            return;
        }
        match kind {
            RenderKind::Flex { .. } => {
                if let Some(overflow) = self.active_flex_overflow(id) {
                    let side = if overflow.axis.is_horizontal() {
                        Side::Right
                    } else {
                        Side::Bottom
                    };
                    self.paint_overflow_side(side, overflow.pixels, size, out);
                    self.publish_flex_overflow_on_paint(id);
                }
            }
            RenderKind::Unconstrained {
                clip_behavior: Clip::None,
                ..
            } => {
                let Some(child) = self
                    .renders
                    .get(id.0)
                    .and_then(|node| node.children.first())
                    .and_then(|child| self.renders.get(child.0))
                else {
                    return;
                };
                let left = f64::from(-child.offset.x).max(0.);
                let top = f64::from(-child.offset.y).max(0.);
                let right = (f64::from(child.offset.x) + f64::from(child.size.width)
                    - f64::from(size.width))
                .max(0.);
                let bottom = (f64::from(child.offset.y) + f64::from(child.size.height)
                    - f64::from(size.height))
                .max(0.);
                let sides = [
                    (Side::Left, left),
                    (Side::Right, right),
                    (Side::Top, top),
                    (Side::Bottom, bottom),
                ];
                if sides.iter().any(|(_, pixels)| *pixels > 0.) {
                    for (side, pixels) in sides.into_iter().filter(|(_, pixels)| *pixels > 0.) {
                        self.paint_overflow_side(side, pixels, size, out);
                    }
                    self.publish_constraint_overflow(ConstraintOverflowDiagnostic {
                        render: id,
                        left,
                        top,
                        bottom,
                        right,
                    });
                }
            }
            _ => {}
        }
    }

    fn paint_overflow_side(&mut self, side: Side, pixels: f64, size: Size, out: &mut DisplayList) {
        let (marker, label_offset, rotation) = side.marker(size);
        out.push(PaintCommand::RRect {
            rrect: RRect::uniform(marker, 0.),
            brush: stripe_brush(),
        });
        let label = format!(
            "{} OVERFLOWED BY {} PIXELS",
            side.label(),
            overflow_pixels_text(pixels)
        );
        let color = Color::rgba(144, 0, 0, 255);
        let style = TextStyle::new()
            .size(FONT_SIZE)
            .font_weight(FontWeight::W800)
            .color(color);
        let layout = self.text_engine.get_mut().layout_with_options(
            &label,
            &style,
            TextLayoutOptions::new(None, TextAlign::Left),
        );
        let origin = Offset::new(-layout.metrics.size.width * 0.5, 0.);
        let transform = CoreTransform::translation(origin)
            .then(CoreTransform::rotation(rotation))
            .then(CoreTransform::translation(label_offset));
        out.push(PaintCommand::PushTransform { transform });
        out.push(PaintCommand::Rect {
            rect: Rect::from_origin_size(Offset::ZERO, layout.metrics.size),
            color: Color::WHITE,
        });
        for line in layout.lines.iter() {
            for run in line.runs.iter() {
                out.push(PaintCommand::GlyphRun {
                    run: run.clone(),
                    color,
                });
            }
        }
        out.push(PaintCommand::PopTransform);
    }
}
