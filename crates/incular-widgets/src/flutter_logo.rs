//! Flutter's `FlutterLogo` (`widgets/flutter_logo.dart`,
//! `painting/flutter_logo.dart`): the mark alone, or with the "Flutter"
//! word mark beside or below it.

use std::{sync::Arc, time::Duration};

use incular_animation::Curve;
use incular_config::{Alignment, TextDirection};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{
    Canvas, DisplayList, FillRule, GradientStop, GradientStops, LinearGradient, Path,
};
use incular_text::{FontWeight, TextStyle};

use crate::{
    Align, AnimatedContainer, Builder, CustomPaint, IconTheme, LayoutBuilder, Positioned, Stack,
    Text, Widget,
};

/// Flutter's `FlutterLogoStyle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FlutterLogoStyle {
    /// Only the mark.
    #[default]
    MarkOnly,
    /// The mark with the word "Flutter" to its right.
    Horizontal,
    /// The mark with the word "Flutter" below it.
    Stacked,
}

/// Where `_FlutterLogoPainter.paint` places the logo in a box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlutterLogoLayout {
    /// The logo's aspect ratio (202 x 202, 820 x 232 or 252 x 306) fitted
    /// with `BoxFit.contain` and centered.
    pub frame: Rect,
    /// `logoSquare`: the square the mark is drawn in.
    pub mark: Rect,
    /// The word mark's font size, `100 * 350 / 247` at the painter's text
    /// scale; zero without a word mark.
    pub font_size: f32,
    /// For [`FlutterLogoStyle::Horizontal`], the left edge of the word mark,
    /// which is centered vertically in `frame`. For
    /// [`FlutterLogoStyle::Stacked`], the horizontal center of the word mark
    /// and its top edge.
    pub text_anchor: Offset,
}

/// Flutter's `FlutterLogo`: the logo in a square box whose side is `size`,
/// or the ambient [`IconTheme`] size (24 by default). A changed size
/// animates; a changed style switches at once, where Flutter cross-fades
/// the word mark in.
///
/// The word mark is set in Roboto Light (`FontWeight::W300`), Flutter's
/// face for it, falling back to the platform fonts where Roboto is not
/// installed.
#[derive(Clone)]
pub struct FlutterLogo {
    size: Option<f32>,
    text_color: Color,
    style: FlutterLogoStyle,
    duration: Duration,
    curve: Curve,
}

impl Default for FlutterLogo {
    fn default() -> Self {
        Self::new()
    }
}

/// The font size `_prepareText` lays the word mark out at.
const BASE_FONT_SIZE: f32 = 100.0 * 350.0 / 247.0;

impl FlutterLogo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            size: None,
            text_color: Color::rgba(0x75, 0x75, 0x75, 0xFF),
            style: FlutterLogoStyle::MarkOnly,
            duration: Duration::from_millis(750),
            curve: Curve::FastOutSlowIn,
        }
    }

    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// The word mark color. Defaults to `0xFF757575`.
    #[must_use]
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Defaults to [`FlutterLogoStyle::MarkOnly`].
    #[must_use]
    pub fn style(mut self, style: FlutterLogoStyle) -> Self {
        self.style = style;
        self
    }

    /// How long a size change animates. Defaults to 750 ms.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Defaults to [`Curve::FastOutSlowIn`].
    #[must_use]
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// `_FlutterLogoPainter.paint`'s geometry for a box of `size`.
    #[must_use]
    pub fn layout(size: Size, style: FlutterLogoStyle) -> FlutterLogoLayout {
        let logo = match style {
            FlutterLogoStyle::Horizontal => Size::new(820., 232.),
            FlutterLogoStyle::Stacked => Size::new(252., 306.),
            FlutterLogoStyle::MarkOnly => Size::new(202., 202.),
        };
        // `applyBoxFit(BoxFit.contain, ...)` inscribed at the center.
        let fit = (size.width / logo.width).min(size.height / logo.height);
        let fitted = Size::new(logo.width * fit, logo.height * fit);
        let frame = Rect::from_origin_size(
            Offset::new(
                (size.width - fitted.width) / 2.,
                (size.height - fitted.height) / 2.,
            ),
            fitted,
        );
        let side = size.width.min(size.height);
        let center_square = Rect::from_origin_size(
            Offset::new((size.width - side) / 2., (size.height - side) / 2.),
            Size::new(side, side),
        );
        // The mark keeps 10.4 of its 202 units clear at each end.
        let inset = 1. - (10.4 * 2.) / 202.;
        let (left, top) = (frame.origin.x, frame.origin.y);
        let (width, height) = (frame.size.width, frame.size.height);
        match style {
            FlutterLogoStyle::MarkOnly => FlutterLogoLayout {
                frame,
                mark: center_square,
                font_size: 0.,
                text_anchor: Offset::ZERO,
            },
            FlutterLogoStyle::Horizontal => {
                let font_size = 2. / 3. * height * inset;
                // 256.4 is the left edge of the F in the 820-wide logo, and
                // 32 the gap before the F in a 350-pixel font.
                let text_left = left + 256.4 / 820. * width - 32. / 350. * font_size;
                FlutterLogoLayout {
                    frame,
                    mark: Rect::from_origin_size(frame.origin, Size::new(height, height)),
                    font_size: BASE_FONT_SIZE * font_size / 100.,
                    text_anchor: Offset::new(text_left, top),
                }
            }
            FlutterLogoStyle::Stacked => {
                let mark_side = height * 191. / 306.;
                let font_size = 0.35 * mark_side * inset;
                FlutterLogoLayout {
                    frame,
                    mark: Rect::from_origin_size(
                        Offset::new(left + (width - mark_side) / 2., top),
                        Size::new(mark_side, mark_side),
                    ),
                    font_size: BASE_FONT_SIZE * font_size / 100.,
                    text_anchor: Offset::new(left + width / 2., top + mark_side),
                }
            }
        }
    }

    /// `FlutterLogoDecoration.paint` for the mark only: the 166 x 202 mark
    /// centered in the largest square of `size`.
    #[must_use]
    pub fn paint(size: Size) -> DisplayList {
        Self::paint_mark(size, FlutterLogoStyle::MarkOnly)
    }

    /// The mark of `style` in a box of `size`, in [`FlutterLogo::layout`]'s
    /// `mark` square. The word mark is a [`Text`] laid over it.
    #[must_use]
    pub fn paint_mark(size: Size, style: FlutterLogoStyle) -> DisplayList {
        let mut canvas = Canvas::default();
        if size.width > 0. && size.height > 0. {
            paint_mark(&mut canvas, Self::layout(size, style).mark);
        }
        canvas.finish()
    }

    /// The word mark placed at `layout`'s anchor, or `None` for the mark
    /// only.
    fn word_mark(
        layout: &FlutterLogoLayout,
        size: Size,
        style: FlutterLogoStyle,
        color: Color,
    ) -> Option<Widget> {
        let text: Widget = Text::new("Flutter")
            .style(
                TextStyle::default()
                    .font_family("Roboto")
                    .font_size(layout.font_size)
                    .font_weight(FontWeight::W300),
            )
            .color(color)
            .text_direction(TextDirection::Ltr)
            .soft_wrap(false)
            .max_lines(Some(1))
            .into();
        let anchor = layout.text_anchor;
        match style {
            FlutterLogoStyle::MarkOnly => None,
            FlutterLogoStyle::Horizontal => Some(
                Positioned::new(Align::new(Alignment::CENTER_LEFT, text))
                    .left(anchor.x)
                    .top(anchor.y)
                    .height(layout.frame.size.height)
                    .into(),
            ),
            // A box centered on the anchor and wider than the logo centers
            // the word mark on it.
            FlutterLogoStyle::Stacked => Some(
                Positioned::new(Align::new(Alignment::TOP_CENTER, text))
                    .left(anchor.x - size.width)
                    .width(2. * size.width)
                    .top(anchor.y)
                    .into(),
            ),
        }
    }
}

fn polygon(points: &[(f32, f32)]) -> Arc<Path> {
    let mut builder = Path::builder();
    builder.move_to(Offset::new(points[0].0, points[0].1));
    for &(x, y) in &points[1..] {
        builder.line_to(Offset::new(x, y));
    }
    builder.close();
    Arc::new(builder.build())
}

/// `FlutterLogoDecoration._paintLogo`.
fn paint_mark(canvas: &mut Canvas, rect: Rect) {
    // The mark is drawn in a 166 x 202 space, centered in a 202 square.
    canvas.save_transform(CoreTransform::translation(rect.origin).then(
        CoreTransform::scale_non_uniform(rect.size.width / 202., rect.size.height / 202.).then(
            CoreTransform::translation(Offset::new((202. - 166.) / 2., 0.)),
        ),
    ));
    let light = Color::rgba(0x54, 0xC5, 0xF8, 0xFF);
    let medium = Color::rgba(0x29, 0xB6, 0xF6, 0xFF);
    let dark = Color::rgba(0x01, 0x57, 0x9B, 0xFF);
    let triangle = LinearGradient::new(
        Offset::new(87.2623 + 37.9092, 28.8384 + 123.4389),
        Offset::new(42.9205 + 37.9092, 35.0952 + 123.4389),
        GradientStops::new(vec![
            GradientStop {
                offset: 0.,
                color: Color::rgba(0x1A, 0x23, 0x7E, 0x00),
            },
            GradientStop {
                offset: 1.,
                color: Color::rgba(0x1A, 0x23, 0x7E, 0x66),
            },
        ]),
    );
    // The basic shape.
    canvas.fill_path(
        polygon(&[(37.7, 128.9), (9.8, 101.0), (100.4, 10.4), (156.2, 10.4)]),
        light,
        FillRule::NonZero,
    );
    canvas.fill_path(
        polygon(&[(156.2, 94.0), (100.4, 94.0), (78.5, 115.9), (106.4, 143.8)]),
        light,
        FillRule::NonZero,
    );
    canvas.fill_path(
        polygon(&[
            (79.5, 170.7),
            (100.4, 191.6),
            (156.2, 191.6),
            (107.4, 142.8),
        ]),
        dark,
        FillRule::NonZero,
    );
    // The overlap between the middle and bottom beams. Flutter's matrix
    // writes 0.7071, not 1/√2.
    #[allow(clippy::approx_constant)]
    canvas.save_transform(CoreTransform::from_coefficients([
        0.7071, -0.7071, 0.7071, 0.7071, -77.697, 98.057,
    ]));
    canvas.rect(
        Rect::from_origin_size(Offset::new(59.8, 123.1), Size::new(39.4, 39.4)),
        medium,
    );
    canvas.restore();
    // The gradient below the middle beam, over the bottom beam.
    canvas.fill_path(
        polygon(&[(79.5, 170.7), (120.9, 156.4), (107.4, 142.8)]),
        triangle,
        FillRule::NonZero,
    );
    canvas.restore();
}

impl From<FlutterLogo> for Widget {
    fn from(value: FlutterLogo) -> Self {
        Builder::new(move |context| {
            let size = value
                .size
                .or_else(|| {
                    context
                        .depend_on::<IconTheme>()
                        .and_then(|theme| theme.icon_size())
                })
                .unwrap_or(24.);
            let (style, text_color) = (value.style, value.text_color);
            AnimatedContainer::new(value.duration)
                .curve(value.curve)
                .width(size)
                .height(size)
                .child(LayoutBuilder::new(move |_, constraints| {
                    let size = constraints.biggest();
                    let mark: Widget =
                        CustomPaint::new(size, FlutterLogo::paint_mark(size, style)).into();
                    let layout = FlutterLogo::layout(size, style);
                    match FlutterLogo::word_mark(&layout, size, style, text_color) {
                        Some(word_mark) => Stack::new([mark, word_mark]).into(),
                        None => mark,
                    }
                }))
                .into()
        })
        .into()
    }
}
