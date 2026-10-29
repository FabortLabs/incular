//! Flutter's `FlutterLogo` (`widgets/flutter_logo.dart`,
//! `painting/flutter_logo.dart`), in its default mark-only style.

use std::{sync::Arc, time::Duration};

use incular_animation::Curve;
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{
    Canvas, DisplayList, FillRule, GradientStop, GradientStops, LinearGradient, Path,
};

use crate::{AnimatedContainer, Builder, CustomPaint, IconTheme, LayoutBuilder, Widget};

/// Flutter's `FlutterLogo` with `FlutterLogoStyle.markOnly`: the mark,
/// centered in a square box whose side is `size`, or the ambient
/// [`IconTheme`] size (24 by default). A changed size animates.
///
/// Flutter's horizontal and stacked styles, which add the word mark, are
/// not ported.
#[derive(Clone)]
pub struct FlutterLogo {
    size: Option<f32>,
    duration: Duration,
    curve: Curve,
}

impl Default for FlutterLogo {
    fn default() -> Self {
        Self::new()
    }
}

impl FlutterLogo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            size: None,
            duration: Duration::from_millis(750),
            curve: Curve::FastOutSlowIn,
        }
    }

    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
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

    /// `FlutterLogoDecoration.paint` for the mark: the 166 x 202 mark
    /// centered in the largest square of `size`.
    #[must_use]
    pub fn paint(size: Size) -> DisplayList {
        let mut canvas = Canvas::default();
        if size.width > 0. && size.height > 0. {
            let side = size.width.min(size.height);
            let square = Rect::from_origin_size(
                Offset::new((size.width - side) / 2., (size.height - side) / 2.),
                Size::new(side, side),
            );
            paint_mark(&mut canvas, square);
        }
        canvas.finish()
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
            AnimatedContainer::new(value.duration)
                .curve(value.curve)
                .width(size)
                .height(size)
                .child(LayoutBuilder::new(|_, constraints| {
                    let size = constraints.biggest();
                    CustomPaint::new(size, FlutterLogo::paint(size)).into()
                }))
                .into()
        })
        .into()
    }
}
