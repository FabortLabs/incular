//! Concrete retained-widget descriptors.
//!
//! Public descriptor types lower into the opaque `Widget` transport. Mutable mounted state continues to live exclusively in `WidgetTree`.

use super::*;
use crate::tree::specs::TextParagraphPolicy;

/// Renderer-neutral image sizing policy (maps to Flutter's `BoxFit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageFit {
    Fill,
    #[default]
    Contain,
    Cover,
    FitWidth,
    FitHeight,
    None,
    ScaleDown,
}

/// Alias for [`ImageFit`], matching Flutter naming.
pub type BoxFit = ImageFit;

/// Repetition policy for raster image painting inside its allocated bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageRepeat {
    #[default]
    NoRepeat,
    RepeatX,
    RepeatY,
    Repeat,
}
/// The paint and sizing options an image carries, shared by [`Image`] and
/// [`ProviderImage`](crate::ProviderImage).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImageOptions {
    pub(crate) width: Option<f32>,
    pub(crate) height: Option<f32>,
    pub(crate) scale: f32,
    /// `None` is Flutter's null fit: scale-down, or fill with a center slice.
    pub(crate) fit: Option<ImageFit>,
    pub(crate) repeat: ImageRepeat,
    pub(crate) alignment: Alignment,
    pub(crate) sampling: ImageSampling,
    pub(crate) center_slice: Option<Rect>,
    pub(crate) match_text_direction: bool,
    /// Flutter `color`: a tint blended over the image.
    pub(crate) color: Option<Color>,
    /// Flutter `colorBlendMode`; `None` is Flutter's `srcIn`.
    pub(crate) color_blend_mode: Option<BlendMode>,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            scale: 1.0,
            fit: None,
            repeat: ImageRepeat::NoRepeat,
            alignment: Alignment::CENTER,
            sampling: ImageSampling::Linear,
            center_slice: None,
            match_text_direction: false,
            color: None,
            color_blend_mode: None,
        }
    }
}

/// Builder methods for the [`ImageOptions`] in a type's `options` field.
macro_rules! image_option_setters {
    () => {
        /// Sets an explicit width. Negative values floor to zero, matching
        /// the other size-bearing descriptors; core `Size::new` would
        /// otherwise panic later during layout.
        #[must_use]
        pub fn width(mut self, width: f32) -> Self {
            self.options.width = Some(width.max(0.0));
            self
        }
        /// Sets an explicit height. Negative values floor to zero.
        #[must_use]
        pub fn height(mut self, height: f32) -> Self {
            self.options.height = Some(height.max(0.0));
            self
        }
        /// Sets the mapping from source pixels to intrinsic logical size: a
        /// scale of `n` reports each `n` source pixels as one logical pixel,
        /// so the intrinsic size is the decoded size divided by `n`. Values
        /// are floored to a small positive epsilon; explicit width/height
        /// still win when present. Scale never alters the source identity.
        #[must_use]
        pub fn scale(mut self, scale: f32) -> Self {
            self.options.scale = if scale.is_finite() && scale > 0.0 {
                scale.max(0.001)
            } else {
                1.0
            };
            self
        }
        #[must_use]
        pub fn fit(mut self, fit: $crate::tree::ImageFit) -> Self {
            self.options.fit = Some(fit);
            self
        }
        #[must_use]
        pub fn repeat(mut self, repeat: $crate::tree::ImageRepeat) -> Self {
            self.options.repeat = repeat;
            self
        }
        /// Sets the alignment used when the fitted image is smaller than its
        /// box or cropped by `Cover`.
        #[must_use]
        pub fn alignment(mut self, alignment: incular_config::Alignment) -> Self {
            self.options.alignment = alignment;
            self
        }
        #[must_use]
        pub fn sampling(mut self, sampling: incular_rendering::ImageSampling) -> Self {
            self.options.sampling = sampling;
            self
        }
        #[must_use]
        pub fn filter_quality(self, quality: incular_rendering::FilterQuality) -> Self {
            self.sampling(quality.into())
        }
        /// Flutter `Image.centerSlice`, in logical image pixels: the image
        /// paints as a nine-slice (`Canvas.drawImageNine`), corners unscaled
        /// and the edges and center stretched. Layout is unchanged.
        #[must_use]
        pub fn center_slice(mut self, slice: incular_core::Rect) -> Self {
            self.options.center_slice = Some(slice);
            self
        }
        /// Flutter `Image.matchTextDirection`: under a right-to-left ambient
        /// [`Directionality`](crate::Directionality) the image paints
        /// mirrored horizontally (and its alignment mirrored with it).
        #[must_use]
        pub fn match_text_direction(mut self, matches: bool) -> Self {
            self.options.match_text_direction = matches;
            self
        }
        /// Flutter `color`: blends this color over the image with
        /// [`Self::color_blend_mode`] (Flutter `srcIn` by default).
        #[must_use]
        pub fn color(mut self, color: incular_core::Color) -> Self {
            self.options.color = Some(color);
            self
        }
        /// Flutter `colorBlendMode`: how [`Self::color`] combines with the
        /// image.
        #[must_use]
        pub fn color_blend_mode(mut self, mode: incular_rendering::BlendMode) -> Self {
            self.options.color_blend_mode = Some(mode);
            self
        }
    };
}
pub(crate) use image_option_setters;

/// A declarative raster image. It retains only a shared asset handle.
///
/// For an image that loads from an [`ImageProvider`](crate::ImageProvider)
/// with Flutter's loading, frame and error builders, see
/// [`Image::provider`].
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    image: ImageHandle,
    options: ImageOptions,
}
impl Image {
    #[must_use]
    pub fn new(image: ImageHandle) -> Self {
        Self {
            image,
            options: ImageOptions::default(),
        }
    }

    /// Flutter's `Image(image: provider)`: an image that resolves
    /// `provider` and follows its loading state. See
    /// [`ProviderImage`](crate::ProviderImage).
    #[must_use]
    pub fn provider(provider: impl crate::ImageProvider + 'static) -> crate::ProviderImage {
        crate::ProviderImage::new(provider)
    }

    pub(crate) fn with_options(image: ImageHandle, options: ImageOptions) -> Self {
        Self { image, options }
    }

    image_option_setters!();
}
impl From<Image> for Widget {
    fn from(value: Image) -> Self {
        let Image { image, mut options } = value;
        // Flutter `RenderImage` paints `color` over the image with
        // `colorBlendMode` (`srcIn` by default), as `RawImage` does here.
        if let Some(color) = options.color.take() {
            let filter = options.color_blend_mode.take().map_or_else(
                || ColorFilter::tint(color),
                |mode| ColorFilter::blend(color, mode),
            );
            return crate::ColorFiltered::new(filter, Image::with_options(image, options)).into();
        }
        if options.center_slice.is_some() || options.match_text_direction {
            return crate::Builder::new(move |context| {
                let flip = options.match_text_direction
                    && context.text_direction() == Some(TextDirection::Rtl);
                let plain = ImageOptions {
                    center_slice: None,
                    match_text_direction: false,
                    ..options.clone()
                };
                if options.center_slice.is_none() && !flip {
                    return Image::with_options(image.clone(), plain).into();
                }
                let sizing: Widget = Image::with_options(image.clone(), plain).into();
                crate::painting_effects::painted_image(
                    crate::painting_effects::ImagePaint {
                        image: image.clone(),
                        scale: options.scale,
                        fit: options.fit,
                        alignment: options.alignment,
                        center_slice: options.center_slice,
                        repeat: options.repeat,
                        flip_horizontally: flip,
                        sampling: options.sampling,
                    },
                    Some(sizing),
                )
            })
            .into();
        }
        Widget::image(
            image,
            options.width,
            options.height,
            options.scale,
            options.fit.unwrap_or(ImageFit::ScaleDown),
            options.repeat,
            options.alignment,
            options.sampling,
        )
    }
}

/// Immutable declarative vector shape. Its coordinates remain local; normal
/// layout and compositor translation position it without changing PathId.
#[derive(Clone, Debug, PartialEq)]
pub struct PathView {
    path: Arc<Path>,
    fill: Option<Brush>,
    stroke: Option<(Brush, Stroke)>,
    size: Option<Size>,
}
impl PathView {
    #[must_use]
    pub fn new(path: impl Into<Arc<Path>>) -> Self {
        Self {
            path: path.into(),
            fill: None,
            stroke: None,
            size: None,
        }
    }
    #[must_use]
    pub fn fill(mut self, brush: impl Into<Brush>) -> Self {
        self.fill = Some(brush.into());
        self
    }
    #[must_use]
    pub fn stroke(mut self, brush: impl Into<Brush>, stroke: Stroke) -> Self {
        self.stroke = Some((brush.into(), stroke));
        self
    }
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
}
impl From<PathView> for Widget {
    fn from(value: PathView) -> Self {
        Widget::shape(value.path, value.fill, value.stroke, value.size)
    }
}

/// Reusable vector icon backed by the same retained Path renderer as PathView.
///
/// By default the path is fitted to the icon's square by its own bounds.
/// Icon sets authored on a fixed grid (Lucide's and Material's 24px canvas)
/// keep their built-in padding with [`Self::view_box`], which maps that
/// canvas onto the square instead. Outline icon sets paint with
/// [`Self::stroke`].
#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    path: Arc<Path>,
    size: f32,
    brush: Brush,
    view_box: Option<Rect>,
    stroke: Option<Stroke>,
    fill: bool,
}
impl Icon {
    #[must_use]
    pub fn new(path: impl Into<Arc<Path>>) -> Self {
        Self {
            path: path.into(),
            size: 24.,
            // Flutter's IconThemeData.fallback() colour is opaque black.
            brush: Color::BLACK.into(),
            view_box: None,
            stroke: None,
            fill: true,
        }
    }
    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(0.);
        self
    }
    #[must_use]
    pub fn brush(mut self, brush: impl Into<Brush>) -> Self {
        self.brush = brush.into();
        self
    }
    /// The canvas the path is authored on (for example `0,0 24x24`). It is
    /// scaled uniformly and centered into the icon's square, so the space an
    /// icon set leaves around each glyph is kept rather than refitted away.
    #[must_use]
    pub fn view_box(mut self, view_box: Rect) -> Self {
        self.view_box = (view_box.size.width > 0. && view_box.size.height > 0.).then_some(view_box);
        self
    }
    /// Paints the path's outline with the icon's brush instead of filling
    /// it, as outline icon sets (`stroke="currentColor" fill="none"`) do.
    /// The width is in path units, so it scales with the icon.
    #[must_use]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self.fill = false;
        self
    }
    /// Fills the path in addition to the outline set by [`Self::stroke`].
    #[must_use]
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }
}
impl From<Icon> for Widget {
    fn from(value: Icon) -> Self {
        // A PathView's `size` controls layout only, so the geometry itself
        // is mapped into the square: a 12px check authored on a 24px canvas
        // must paint at 1.5..10.5, not at 3..21. Without a view box the
        // path's own bounds are the canvas.
        let canvas = value.view_box.or_else(|| {
            value
                .path
                .bounds()
                .filter(|bounds| bounds.size.width > 0. && bounds.size.height > 0.)
        });
        let (path, scale) = canvas.map_or_else(
            || (value.path.clone(), 1.),
            |canvas| {
                let scale = (value.size / canvas.size.width).min(value.size / canvas.size.height);
                let fitted = Size::new(canvas.size.width * scale, canvas.size.height * scale);
                let offset = Offset::new(
                    (value.size - fitted.width) * 0.5 - canvas.origin.x * scale,
                    (value.size - fitted.height) * 0.5 - canvas.origin.y * scale,
                );
                (
                    Arc::new(
                        value.path.transformed(
                            CoreTransform::translation(offset)
                                .then(CoreTransform::scale_non_uniform(scale, scale)),
                        ),
                    ),
                    scale,
                )
            },
        );
        let mut view = PathView::new(path).size(Size::new(value.size, value.size));
        if value.fill {
            view = view.fill(value.brush.clone());
        }
        if let Some(stroke) = value.stroke {
            view = view.stroke(
                value.brush,
                Stroke {
                    width: stroke.width * scale,
                    ..stroke
                },
            );
        }
        view.into()
    }
}

/// Small shared demo icons. Each function returns the same immutable `Path`,
/// so repeated icons share PathId and retained tessellation/GPU meshes.
pub mod icons {
    use super::*;
    use std::sync::OnceLock;
    fn path(build: impl FnOnce(&mut incular_painting::PathBuilder)) -> Arc<Path> {
        let mut builder = Path::builder();
        build(&mut builder);
        Arc::new(builder.build())
    }
    #[must_use]
    pub fn check() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(3., 12.))
                    .line_to(Offset::new(9., 18.))
                    .line_to(Offset::new(21., 4.))
                    .line_to(Offset::new(18., 2.))
                    .line_to(Offset::new(9., 14.))
                    .line_to(Offset::new(5., 10.))
                    .close();
            })
        })
        .clone()
    }
    #[must_use]
    pub fn close() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(3., 5.))
                    .line_to(Offset::new(5., 3.))
                    .line_to(Offset::new(12., 10.))
                    .line_to(Offset::new(19., 3.))
                    .line_to(Offset::new(21., 5.))
                    .line_to(Offset::new(14., 12.))
                    .line_to(Offset::new(21., 19.))
                    .line_to(Offset::new(19., 21.))
                    .line_to(Offset::new(12., 14.))
                    .line_to(Offset::new(5., 21.))
                    .line_to(Offset::new(3., 19.))
                    .line_to(Offset::new(10., 12.))
                    .close();
            })
        })
        .clone()
    }
    #[must_use]
    pub fn plus() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(10., 3.))
                    .line_to(Offset::new(14., 3.))
                    .line_to(Offset::new(14., 10.))
                    .line_to(Offset::new(21., 10.))
                    .line_to(Offset::new(21., 14.))
                    .line_to(Offset::new(14., 14.))
                    .line_to(Offset::new(14., 21.))
                    .line_to(Offset::new(10., 21.))
                    .line_to(Offset::new(10., 14.))
                    .line_to(Offset::new(3., 14.))
                    .line_to(Offset::new(3., 10.))
                    .line_to(Offset::new(10., 10.))
                    .close();
            })
        })
        .clone()
    }
    #[must_use]
    pub fn minus() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(3., 10.))
                    .line_to(Offset::new(21., 10.))
                    .line_to(Offset::new(21., 14.))
                    .line_to(Offset::new(3., 14.))
                    .close();
            })
        })
        .clone()
    }
    #[must_use]
    pub fn chevron_right() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(7., 3.))
                    .line_to(Offset::new(10., 0.))
                    .line_to(Offset::new(22., 12.))
                    .line_to(Offset::new(10., 24.))
                    .line_to(Offset::new(7., 21.))
                    .line_to(Offset::new(16., 12.))
                    .close();
            })
        })
        .clone()
    }

    /// A compact downward chevron used by select, disclosure, and menu
    /// controls.  Keeping each direction as a shared immutable path avoids
    /// per-control path construction and makes the icon independent of font
    /// fallback or glyph metrics.
    #[must_use]
    pub fn chevron_down() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(3., 7.))
                    .line_to(Offset::new(6., 4.))
                    .line_to(Offset::new(12., 10.))
                    .line_to(Offset::new(18., 4.))
                    .line_to(Offset::new(21., 7.))
                    .line_to(Offset::new(12., 16.))
                    .close();
            })
        })
        .clone()
    }

    /// A compact upward chevron.
    #[must_use]
    pub fn chevron_up() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(3., 17.))
                    .line_to(Offset::new(6., 20.))
                    .line_to(Offset::new(12., 14.))
                    .line_to(Offset::new(18., 20.))
                    .line_to(Offset::new(21., 17.))
                    .line_to(Offset::new(12., 8.))
                    .close();
            })
        })
        .clone()
    }

    /// A compact left-pointing chevron.
    #[must_use]
    pub fn chevron_left() -> Arc<Path> {
        static PATH: OnceLock<Arc<Path>> = OnceLock::new();
        PATH.get_or_init(|| {
            path(|p| {
                p.move_to(Offset::new(17., 3.))
                    .line_to(Offset::new(20., 6.))
                    .line_to(Offset::new(14., 12.))
                    .line_to(Offset::new(20., 18.))
                    .line_to(Offset::new(17., 21.))
                    .line_to(Offset::new(8., 12.))
                    .close();
            })
        })
        .clone()
    }
}

/// Flutter's `DecoratedBox`: paints a decoration behind (or, with
/// [`DecoratedBox::position`], in front of) its child. It does not clip the
/// child.
#[derive(Clone, Debug, PartialEq)]
pub struct DecoratedBox {
    child: Widget,
    size: Option<Size>,
    background: Option<Brush>,
    border: Option<Border>,
    radius: CornerRadii,
    /// The whole decoration, painted by the full box painter when it has
    /// parts the background, border and radius above cannot express.
    decoration: Option<crate::BoxDecoration>,
    position: crate::DecorationPosition,
}
impl DecoratedBox {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
            size: None,
            background: None,
            border: None,
            radius: CornerRadii::default(),
            decoration: None,
            position: crate::DecorationPosition::Background,
        }
    }
    /// Flutter `DecoratedBox.position` for the decoration given with
    /// [`DecoratedBox::decoration`]. Defaults to
    /// [`DecorationPosition::Background`](crate::DecorationPosition::Background).
    #[must_use]
    pub fn position(mut self, position: crate::DecorationPosition) -> Self {
        self.position = position;
        self
    }
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
    #[must_use]
    pub fn background(mut self, brush: impl Into<Brush>) -> Self {
        self.background = Some(brush.into());
        self
    }
    #[must_use]
    pub fn border(mut self, border: impl Into<Border>) -> Self {
        self.border = Some(border.into());
        self
    }
    #[must_use]
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = CornerRadii::uniform(radius);
        self
    }
    /// Rounds each corner by its own radius.
    #[must_use]
    pub fn corner_radii(mut self, radii: CornerRadii) -> Self {
        self.radius = radii;
        self
    }
    #[must_use]
    pub fn decoration(mut self, decoration: crate::BoxDecoration) -> Self {
        // Flutter's `BoxDecoration` paints `color` or, without one, its
        // `gradient`.
        if let Some(color) = decoration.color {
            self.background = Some(Brush::Solid(color));
        } else if let Some(gradient) = decoration.gradient.clone() {
            self.background = Some(gradient);
        }
        if let Some(border) = decoration.border {
            // A `BorderStyle::None` side paints nothing: Flutter's
            // `BorderSide.toPaint` uses a transparent colour. Its width still
            // feeds layout insets (`strokeInset`), so only the colour is cleared.
            let color = if border.top.style == crate::BorderStyle::None {
                Color::TRANSPARENT
            } else {
                border.top.color
            };
            self.border = Some(incular_rendering::Border::new(border.top.width, color));
        }
        if let Some(radius) = decoration.border_radius {
            self.radius = radius.to_corner_radii();
        }
        self.decoration = Some(decoration);
        self
    }
}
impl From<DecoratedBox> for Widget {
    fn from(value: DecoratedBox) -> Self {
        let full = value.decoration.filter(|decoration| {
            value.position == crate::DecorationPosition::Foreground
                || decoration.needs_full_painter()
        });
        if let Some(decoration) = full {
            let child = match value.size {
                Some(size) => crate::SizedBox::new()
                    .width(size.width)
                    .height(size.height)
                    .child(value.child)
                    .into(),
                None => value.child,
            };
            return crate::painting_effects::decorated(decoration, value.position, child);
        }
        Widget::decorated(
            value.size,
            value.background,
            value.border,
            value.radius,
            value.child,
        )
    }
}

/// Public text description. Its font metrics are resolved during layout, not
/// paint.  A plain [`Text::new`] uses [`TextStyle::default`] (16 logical px,
/// system UI family, normal weight) and reports its natural shaped size; it
/// does not expand to the parent width unless a parent layout allocates that
/// width explicitly.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    text: String,
    style: TextStyle,
    align: Option<TextAlign>,
    soft_wrap: Option<bool>,
    max_lines: Option<Option<usize>>,
    overflow: Option<TextOverflow>,
    text_direction: Option<incular_config::TextDirection>,
    text_height_behavior: Option<TextHeightBehavior>,
    text_scaler: TextScaler,
    strut: Option<StrutStyle>,
}
impl Text {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: TextStyle::default(),
            align: None,
            soft_wrap: None,
            max_lines: None,
            overflow: None,
            text_direction: None,
            text_height_behavior: None,
            text_scaler: TextScaler::default(),
            strut: None,
        }
    }
    /// Flutter's `Text.textHeightBehavior`. Without it the nearest
    /// [`DefaultTextHeightBehavior`](crate::DefaultTextHeightBehavior) is
    /// used, else the default behavior.
    #[must_use]
    pub fn text_height_behavior(mut self, behavior: TextHeightBehavior) -> Self {
        self.text_height_behavior = Some(behavior);
        self
    }
    /// Sets the paragraph's text direction, like Flutter's `Text.textDirection`.
    ///
    /// Without it the nearest ambient [`Directionality`](crate::Directionality)
    /// is used; a `Text` that finds neither is a build error
    /// (`TreeError::MissingDirectionality`), exactly as Flutter's
    /// `Directionality.of` fails.
    #[must_use]
    pub fn text_direction(mut self, direction: incular_config::TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }
    /// Flutter's `Text.strutStyle`: every line is at least as tall above and
    /// below its baseline as the strut's font, or exactly that tall with
    /// `force_strut_height`. An unset strut font size is 14 and an unset
    /// family is the text's; the text scaler scales the strut size too.
    #[must_use]
    pub fn strut_style(mut self, strut: StrutStyle) -> Self {
        self.strut = Some(strut);
        self
    }
    /// Flutter's `Text.textScaler`: scales the resolved font size. The default
    /// is the identity scaler, so the system text scale is not applied here.
    #[must_use]
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> Self {
        self.text_scaler = text_scaler;
        self
    }
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = color;
        self
    }
    /// Sets the paragraph alignment. Without it the nearest
    /// [`DefaultTextStyle`](crate::DefaultTextStyle) alignment applies, else
    /// start.
    #[must_use]
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = Some(align);
        self
    }
    /// Enables or disables Parley's soft line breaking at the allocated width.
    /// Without it the nearest [`DefaultTextStyle`](crate::DefaultTextStyle)
    /// value applies, else `true`.
    #[must_use]
    pub fn soft_wrap(mut self, soft_wrap: bool) -> Self {
        self.soft_wrap = Some(soft_wrap);
        self
    }
    /// Caps the paragraph at `max_lines`. `None` inherits the nearest
    /// [`DefaultTextStyle`](crate::DefaultTextStyle) cap, else no cap
    /// (Flutter's null `maxLines`).
    #[must_use]
    pub fn max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines.map(Some);
        self
    }
    /// Sets the overflow policy. Without it the nearest
    /// [`DefaultTextStyle`](crate::DefaultTextStyle) value applies, else clip.
    #[must_use]
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = Some(overflow);
        self
    }
}
impl From<Text> for Widget {
    fn from(value: Text) -> Self {
        Widget::text_configured(
            value.text,
            value.style,
            TextParagraphPolicy {
                align: value.align,
                soft_wrap: value.soft_wrap,
                max_lines: value.max_lines,
                overflow: value.overflow,
            },
            value.text_direction,
            value.text_height_behavior,
            value.text_scaler,
            None,
            None,
            value.strut.map(Box::new),
        )
    }
}

/// Makes the renderer-independent rich paragraph description available in a
/// retained widget tree. Each styled span keeps its own size, weight, family,
/// line height and color; paragraph policy is the one `RichText` states.
impl From<RichText> for Widget {
    fn from(value: RichText) -> Self {
        rich_text_widget(value, Vec::new(), None)
    }
}

/// The inline boxes a rich paragraph reserves (one per [`WidgetSpan`]) and,
/// for each inline child widget, the index of the box it fills.
#[derive(Clone, Debug, PartialEq)]
pub struct InlinePlaceholders {
    pub placeholders: Arc<[incular_text::TextPlaceholder]>,
    pub slots: Arc<[usize]>,
}

/// A [`RichText`] whose [`WidgetSpan`]s are filled with widgets, as Flutter's
/// `RichText` builds its `WidgetSpan` children. Each child is laid out with
/// the paragraph's maximum width and unbounded height (Flutter
/// `RenderParagraph`, `BoxConstraints(maxWidth:)`); its size, alignment and
/// baseline reserve the span's inline box, and the child is painted and
/// hit-tested where the paragraph places that box. A span without a child
/// reserves its stated size.
#[derive(Clone)]
pub struct RichParagraph {
    text: RichText,
    children: Vec<(u64, Widget)>,
    strut: Option<StrutStyle>,
}

impl RichParagraph {
    #[must_use]
    pub fn new(text: RichText) -> Self {
        Self {
            text,
            children: Vec::new(),
            strut: None,
        }
    }

    /// Fills the [`WidgetSpan`] with this `id` (Flutter `WidgetSpan.child`).
    /// An id no span carries adds nothing.
    #[must_use]
    pub fn child(mut self, id: u64, child: impl Into<Widget>) -> Self {
        self.children.push((id, child.into()));
        self
    }

    /// Flutter `RichText.strutStyle`; see [`Text::strut_style`].
    #[must_use]
    pub fn strut_style(mut self, strut: StrutStyle) -> Self {
        self.strut = Some(strut);
        self
    }
}

impl From<RichParagraph> for Widget {
    fn from(value: RichParagraph) -> Self {
        rich_text_widget(value.text, value.children, value.strut)
    }
}

fn rich_text_widget(
    value: RichText,
    children: Vec<(u64, Widget)>,
    strut: Option<StrutStyle>,
) -> Widget {
    let placeholders = value.text.placeholders();
    let inline = (!placeholders.is_empty()).then(|| {
        let mut slots = Vec::new();
        let mut widgets = Vec::new();
        for (id, child) in children {
            // The first span with this id that no earlier child filled.
            if let Some(slot) = (0..placeholders.len())
                .find(|&slot| placeholders[slot].id == id && !slots.contains(&slot))
            {
                slots.push(slot);
                widgets.push(child);
            }
        }
        Box::new((
            InlinePlaceholders {
                placeholders: placeholders.into(),
                slots: slots.into(),
            },
            widgets,
        ))
    });
    // Unscaled runs: the text scaler is applied once, during lowering.
    let runs = value.text.flatten(&TextStyle::default());
    let style = runs
        .first()
        .map_or_else(TextStyle::default, |run| run.style.clone());
    let spoken = value.text.semantics_text();
    let spans: Option<Arc<[StyledTextRun]>> = Some(runs.into());
    let widget = Widget::text_configured(
        value.plain_text(),
        style,
        TextParagraphPolicy {
            align: Some(value.text_align),
            soft_wrap: Some(value.soft_wrap),
            max_lines: Some(value.max_lines),
            overflow: Some(value.overflow),
        },
        None,
        None,
        value.text_scaler,
        spans,
        inline,
        strut.map(Box::new),
    );
    match spoken {
        Some(label) => widget.accessibility_label(label),
        None => widget,
    }
}

/// Flutter's `ToolbarOptions`: which buttons a legacy text selection
/// toolbar offers. Every option defaults to off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ToolbarOptions {
    pub copy: bool,
    pub cut: bool,
    pub paste: bool,
    pub select_all: bool,
}

impl ToolbarOptions {
    /// Flutter `ToolbarOptions.empty`.
    pub const EMPTY: Self = Self {
        copy: false,
        cut: false,
        paste: false,
        select_all: false,
    };

    /// `EditableText`'s default for a field: none for a read-only obscured
    /// field, select all and paste for an obscured one, select all and copy
    /// for a read-only one, and all four otherwise.
    #[must_use]
    pub const fn for_field(read_only: bool, obscure_text: bool) -> Self {
        match (obscure_text, read_only) {
            (true, true) => Self::EMPTY,
            (true, false) => Self {
                select_all: true,
                paste: true,
                ..Self::EMPTY
            },
            (false, true) => Self {
                select_all: true,
                copy: true,
                ..Self::EMPTY
            },
            (false, false) => Self {
                copy: true,
                cut: true,
                paste: true,
                select_all: true,
            },
        }
    }
}

///
/// This is Incular's renderer-neutral counterpart to Flutter's
/// `widgets/EditableText`.  Material `TextField` and multiline field chrome
/// belong to a higher-level component library. Keep the controller outside a
/// rebuilt widget description so text, selection, and active composition
/// survive rebuilds.
///
/// Numeric policy is resolved once, here: `min_lines`/`max_lines` are
/// `None` or at least one, caret sizes are non-negative, and `multiline`
/// tracks whether `max_lines` differs from a single line with the last
/// setter winning. The widget conversion trusts this invariant instead of
/// re-clamping.
pub struct EditableText {
    controller: TextEditingController,
    size: Size,
    style: TextStyle,
    placeholder: String,
    placeholder_color: Color,
    focused_border: Option<(incular_rendering::Border, f32)>,
    focused_border_outset: incular_config::EdgeInsets,
    content_padding: incular_config::EdgeInsets,
    on_editing_complete: Option<Rc<dyn Fn()>>,
    on_submit: Option<Rc<dyn Fn(String)>>,
    multiline: bool,
    min_lines: Option<usize>,
    max_lines: Option<usize>,
    expands: bool,
    text_align: TextAlign,
    enabled: bool,
    read_only: bool,
    obscure_text: bool,
    cursor_width: f32,
    cursor_height: Option<f32>,
    cursor_radius: f32,
    /// `None` resolves to `!read_only` when the field is lowered, as in
    /// Flutter's `showCursor ?? !readOnly`.
    show_cursor: Option<bool>,
    cursor_color: Color,
    selection_color: Color,
    input_type: TextInputTypeHint,
    input_action: TextInputActionHint,
    text_direction: Option<incular_config::TextDirection>,
    paragraph_layout: bool,
    /// `None` resolves like Flutter's `_inferAutocorrect`.
    autocorrect: Option<bool>,
    enable_suggestions: bool,
    /// `None` resolves to `!read_only || !obscure_text`.
    enable_interactive_selection: Option<bool>,
    text_capitalization: incular_platform::TextCapitalization,
    keyboard_appearance: incular_config::Brightness,
    autofill_hints: Option<Vec<String>>,
    enable_ime_personalized_learning: bool,
    scroll_padding: incular_config::EdgeInsets,
    cursor_opacity_animates: bool,
    /// `None` resolves to [`ToolbarOptions::for_field`].
    toolbar_options: Option<ToolbarOptions>,
    /// Flutter's `contextMenuBuilder`; `None` shows no context menu.
    context_menu_builder: Option<crate::EditableTextContextMenuBuilder>,
    edit_transform: Option<super::specs::EditTransform>,
    edit_changed: Option<super::specs::EditChanged>,
    /// Input formatters, applied in the order added to each user edit.
    input_formatters: Vec<Rc<dyn crate::TextInputFormatter>>,
}
impl EditableText {
    #[must_use]
    pub fn new(controller: TextEditingController) -> Self {
        Self {
            controller,
            size: Size::ZERO,
            style: TextStyle::default(),
            placeholder: String::new(),
            placeholder_color: Color::rgba(150, 154, 170, 255),
            focused_border: None,
            focused_border_outset: incular_config::EdgeInsets::ZERO,
            content_padding: incular_config::EdgeInsets::ZERO,
            on_editing_complete: None,
            on_submit: None,
            multiline: false,
            min_lines: None,
            max_lines: Some(1),
            expands: false,
            text_align: TextAlign::Start,
            enabled: true,
            read_only: false,
            obscure_text: false,
            // Flutter's EditableText default cursorWidth is 2.0.
            cursor_width: 2.0,
            cursor_height: None,
            cursor_radius: 0.0,
            show_cursor: None,
            cursor_color: Color::WHITE,
            selection_color: Color::rgba(72, 120, 220, 150),
            input_type: TextInputTypeHint::Text,
            input_action: TextInputActionHint::Unspecified,
            text_direction: None,
            paragraph_layout: false,
            autocorrect: None,
            enable_suggestions: true,
            enable_interactive_selection: None,
            text_capitalization: incular_platform::TextCapitalization::None,
            keyboard_appearance: incular_config::Brightness::Light,
            autofill_hints: Some(Vec::new()),
            enable_ime_personalized_learning: true,
            scroll_padding: incular_config::EdgeInsets::all(20.0),
            cursor_opacity_animates: false,
            toolbar_options: None,
            context_menu_builder: None,
            edit_transform: None,
            edit_changed: None,
            input_formatters: Vec::new(),
        }
    }

    /// Appends an input formatter (Flutter's `inputFormatters`). Formatters
    /// run in the order added, on each user edit only: keyboard, IME and
    /// paste edits. Programmatic controller updates are not formatted, as in
    /// Flutter. Each formatter sees the field's value before the edit as its
    /// old value.
    #[must_use]
    pub fn input_formatter(mut self, formatter: impl crate::TextInputFormatter + 'static) -> Self {
        self.input_formatters.push(Rc::new(formatter));
        self
    }

    /// Rewrites every user edit before it is applied, after any input
    /// formatters. Use [`Self::input_formatter`] for Flutter-style formatters.
    #[must_use]
    pub fn edit_transform(
        mut self,
        transform: impl Fn(
            &incular_text::TextEditingValue,
            &incular_text::TextEditingValue,
        ) -> incular_text::TextEditingValue
        + 'static,
    ) -> Self {
        self.edit_transform = Some(Rc::new(transform));
        self
    }

    /// Called with the new text after each user edit (Flutter's
    /// `onChanged`).
    #[must_use]
    pub fn on_changed(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.edit_changed = Some(Rc::new(callback));
        self
    }

    /// Sets both edit callbacks from shared closures; `None` clears one.
    #[must_use]
    pub fn edit_callbacks(
        mut self,
        transform: Option<super::specs::EditTransform>,
        changed: Option<super::specs::EditChanged>,
    ) -> Self {
        self.edit_transform = transform;
        self.edit_changed = changed;
        self
    }
    /// Sets the direction of the edited text, like Flutter's
    /// `EditableText.textDirection`. Without it the nearest ambient
    /// `Directionality` is used; an editor that finds neither is a build error
    /// (`TreeError::MissingDirectionality`).
    #[must_use]
    pub fn text_direction(mut self, direction: incular_config::TextDirection) -> Self {
        self.text_direction = Some(direction);
        self
    }
    #[must_use]
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the color used while the editor is empty and showing its
    /// placeholder text.
    #[must_use]
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_color = color;
        self
    }

    /// Sets a border painted by the retained editor only while it owns focus.
    /// This is presentation-only: focus tenure remains in `WidgetTree`.
    #[must_use]
    pub fn focused_border(mut self, border: crate::Border, radius: f32) -> Self {
        self.focused_border = Some((border.into(), radius.max(0.0)));
        self
    }
    /// Grows the focused border past the editor's own bounds by `outset`,
    /// so a field that wraps the editor in padding and a border (a styled
    /// text field's rounded box) outlines that outer box instead of the
    /// inner text area. The border's radius applies to the grown rectangle.
    #[must_use]
    pub fn focused_border_outset(mut self, outset: incular_config::EdgeInsets) -> Self {
        self.focused_border_outset = outset;
        self
    }
    /// Space between the editor's bounds and its text, in the role of
    /// Flutter's `InputDecoration.contentPadding`. The padding stays inside
    /// the editor, so taps on it still focus the field and the focused
    /// border still outlines the whole editor.
    ///
    /// Defaults to none, as Flutter's `EditableText` has none: the editor's
    /// height is its lines' height. Field wrappers such as the themed text
    /// inputs set their own padding.
    #[must_use]
    pub fn content_padding(mut self, padding: incular_config::EdgeInsets) -> Self {
        self.content_padding = padding;
        self
    }
    /// Called when the user completes editing (Flutter's `onEditingComplete`),
    /// before [`Self::on_submit`]. Providing it replaces the default
    /// behavior, which gives up focus for a completion action.
    #[must_use]
    pub fn on_editing_complete(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_editing_complete = Some(Rc::new(callback));
        self
    }
    #[must_use]
    pub fn on_submit(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.on_submit = Some(Rc::new(callback));
        self
    }
    /// Enables multiline editing. Material text-field wrappers should use
    /// this core primitive rather than introducing a separate `TextArea`
    /// widget name.
    #[must_use]
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        if multiline && self.max_lines == Some(1) {
            self.max_lines = None;
        } else if !multiline {
            self.max_lines = Some(1);
        }
        self
    }
    /// Sets an explicit multiline height while preserving the current width.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.size = Size::new(self.size.width, height);
        self
    }

    #[must_use]
    pub fn min_lines(mut self, lines: Option<usize>) -> Self {
        self.min_lines = lines.map(|value| value.max(1));
        self
    }

    #[must_use]
    pub fn max_lines(mut self, lines: Option<usize>) -> Self {
        self.max_lines = lines.map(|value| value.max(1));
        self.multiline = !matches!(self.max_lines, Some(1));
        self
    }

    #[must_use]
    pub fn expands(mut self, expands: bool) -> Self {
        self.expands = expands;
        self
    }

    #[must_use]
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.text_align = align;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[must_use]
    pub fn obscure_text(mut self, obscure_text: bool) -> Self {
        self.obscure_text = obscure_text;
        self
    }

    /// Lays out a multi-line field one hard-broken paragraph at a time, as a
    /// [`TextDocumentLayout`](incular_text::TextDocumentLayout), instead of
    /// as one paragraph like Flutter's `EditableText`.
    ///
    /// Editing then lays out only the changed paragraphs, so typing anywhere
    /// in a long document costs about as much as typing at its end. Caret,
    /// selection and painting are unchanged. Fields limited by `max_lines`
    /// keep the whole-text layout.
    #[must_use]
    pub fn paragraph_layout(mut self, paragraph_layout: bool) -> Self {
        self.paragraph_layout = paragraph_layout;
        self
    }

    /// Sets the caret width in logical pixels.
    #[must_use]
    pub fn cursor_width(mut self, width: f32) -> Self {
        self.cursor_width = width.max(0.0);
        self
    }

    /// Sets an optional caret height. `None` uses the shaped line height.
    #[must_use]
    pub fn cursor_height(mut self, height: Option<f32>) -> Self {
        self.cursor_height = height.map(|value| value.max(0.0));
        self
    }

    /// Sets the caret corner radius. A zero radius keeps the caret rectangular.
    #[must_use]
    pub fn cursor_radius(mut self, radius: f32) -> Self {
        self.cursor_radius = radius.max(0.0);
        self
    }

    /// Controls whether the caret is painted while this editor is focused.
    #[must_use]
    pub fn show_cursor(mut self, show: bool) -> Self {
        self.show_cursor = Some(show);
        self
    }

    /// Sets the caret paint color.
    #[must_use]
    pub fn cursor_color(mut self, color: Color) -> Self {
        self.cursor_color = color;
        self
    }

    /// Sets the selection highlight paint color.
    #[must_use]
    pub fn selection_color(mut self, color: Color) -> Self {
        self.selection_color = color;
        self
    }

    /// Selects the native keyboard/input method hint for this editor. The
    /// runtime still validates and owns the committed text value.
    #[must_use]
    pub fn input_type(mut self, input_type: TextInputTypeHint) -> Self {
        self.input_type = input_type;
        self
    }

    /// Selects the action shown by a native software keyboard.
    #[must_use]
    pub fn input_action(mut self, input_action: TextInputActionHint) -> Self {
        self.input_action = input_action;
        self
    }

    /// Whether the platform corrects spelling as the user types. Unset, it
    /// follows Flutter's `_inferAutocorrect`: off on iOS when an autofill
    /// hint is `username`, `password` or `newPassword`, otherwise on.
    #[must_use]
    pub fn autocorrect(mut self, autocorrect: bool) -> Self {
        self.autocorrect = Some(autocorrect);
        self
    }

    /// Whether the keyboard shows suggestions. Defaults to true.
    #[must_use]
    pub fn enable_suggestions(mut self, enable: bool) -> Self {
        self.enable_suggestions = enable;
        self
    }

    /// Flutter's deprecated `toolbarOptions`: the buttons a legacy
    /// selection toolbar offers. Unset, [`ToolbarOptions::for_field`]
    /// decides from `read_only` and `obscure_text`.
    #[must_use]
    pub fn toolbar_options(mut self, options: ToolbarOptions) -> Self {
        self.toolbar_options = Some(options);
        self
    }

    /// Builds the menu a secondary click opens (Flutter's
    /// `EditableText.contextMenuBuilder`). Unset, like Flutter's bare
    /// `EditableText`, the field has no context menu; Material and Controls
    /// text fields install their platform toolbars.
    #[must_use]
    pub fn context_menu_builder(
        mut self,
        builder: impl for<'a> Fn(&crate::BuildContext<'a>, &crate::TextContextMenu) -> Widget + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(builder));
        self
    }

    /// Replaces the context menu builder; `None` removes the menu.
    #[must_use]
    pub fn context_menu_builder_rc(
        mut self,
        builder: Option<crate::EditableTextContextMenuBuilder>,
    ) -> Self {
        self.context_menu_builder = builder;
        self
    }

    /// The toolbar options in effect.
    #[must_use]
    pub fn resolved_toolbar_options(&self) -> ToolbarOptions {
        self.toolbar_options
            .unwrap_or_else(|| ToolbarOptions::for_field(self.read_only, self.obscure_text))
    }

    /// Flutter's legacy `cutEnabled`: offered, and the field is neither
    /// read-only nor obscured.
    #[must_use]
    pub fn cut_enabled(&self) -> bool {
        self.resolved_toolbar_options().cut && !self.read_only && !self.obscure_text
    }

    /// Flutter's legacy `copyEnabled`: offered, and the field is not
    /// obscured.
    #[must_use]
    pub fn copy_enabled(&self) -> bool {
        self.resolved_toolbar_options().copy && !self.obscure_text
    }

    /// Flutter's legacy `pasteEnabled`: offered, and the field is not
    /// read-only.
    #[must_use]
    pub fn paste_enabled(&self) -> bool {
        self.resolved_toolbar_options().paste && !self.read_only
    }

    /// Flutter's legacy `selectAllEnabled`: offered, interactive selection
    /// is on, and the field is not both read-only and obscured.
    #[must_use]
    pub fn select_all_enabled(&self) -> bool {
        self.resolved_toolbar_options().select_all
            && (!self.read_only || !self.obscure_text)
            && self.enable_interactive_selection.unwrap_or(true)
    }

    /// The selection flag sent to the platform input method. Unset, it is
    /// `!read_only || !obscure_text`, as in Flutter; either way a read-only
    /// obscured field reports false.
    #[must_use]
    pub fn enable_interactive_selection(mut self, enable: bool) -> Self {
        self.enable_interactive_selection = Some(enable);
        self
    }

    /// The casing a soft keyboard applies. Defaults to none.
    #[must_use]
    pub fn text_capitalization(
        mut self,
        capitalization: incular_platform::TextCapitalization,
    ) -> Self {
        self.text_capitalization = capitalization;
        self
    }

    /// The keyboard colour scheme on iOS. Defaults to light.
    #[must_use]
    pub fn keyboard_appearance(mut self, appearance: incular_config::Brightness) -> Self {
        self.keyboard_appearance = appearance;
        self
    }

    /// Flutter `AutofillHints` names; the first one the platform supports
    /// is used. `None` disables autofill. Defaults to an empty list, which
    /// enables autofill without a hint.
    #[must_use]
    pub fn autofill_hints<I, S>(mut self, hints: Option<I>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.autofill_hints = hints.map(|hints| hints.into_iter().map(Into::into).collect());
        self
    }

    /// Whether the IME may learn from what the user types. Defaults to true.
    #[must_use]
    pub fn enable_ime_personalized_learning(mut self, enable: bool) -> Self {
        self.enable_ime_personalized_learning = enable;
        self
    }

    /// The margin kept around the caret when a focused field scrolls its
    /// ancestors to show it (Flutter's `scrollPadding`). Defaults to 20 on
    /// every side.
    #[must_use]
    pub fn scroll_padding(mut self, padding: incular_config::EdgeInsets) -> Self {
        self.scroll_padding = padding.normalized();
        self
    }

    /// Whether the caret fades in and out like iOS instead of switching on
    /// and off (Flutter's `cursorOpacityAnimates`). Defaults to false.
    #[must_use]
    pub fn cursor_opacity_animates(mut self, animates: bool) -> Self {
        self.cursor_opacity_animates = animates;
        self
    }
}

/// Flutter's `EditableText._inferAutocorrect`.
fn infer_autocorrect(autofill_hints: Option<&[String]>) -> bool {
    let Some(hints) = autofill_hints.filter(|hints| !hints.is_empty()) else {
        return true;
    };
    // `username`, `password` and `newPassword` are password related hints;
    // iOS flashes its password bar while autocorrect is on.
    !(cfg!(target_os = "ios")
        && hints
            .iter()
            .any(|hint| matches!(hint.as_str(), "username" | "password" | "newPassword")))
}
/// Folds the input formatters into one user-edit transform: each formatter
/// sees the old value and the output of the one before it, then the raw
/// `edit_transform` (if any) runs last.
fn compose_edit_transform(
    formatters: Vec<Rc<dyn crate::TextInputFormatter>>,
    transform: Option<super::specs::EditTransform>,
) -> Option<super::specs::EditTransform> {
    if formatters.is_empty() {
        return transform;
    }
    Some(Rc::new(move |old, next| {
        let mut value = formatters.iter().fold(next.clone(), |value, formatter| {
            formatter.format_edit_update(old, &value)
        });
        if let Some(transform) = &transform {
            value = transform(old, &value);
        }
        value
    }))
}

impl From<EditableText> for Widget {
    fn from(value: EditableText) -> Self {
        // Numeric policy is already normalized by the setters above.
        let (input_type, input_action) = (value.input_type, value.input_action);
        let context_menu_builder = value.context_menu_builder.clone();
        let edit_transform = compose_edit_transform(value.input_formatters, value.edit_transform);
        let spec = TextFieldSpec {
            context_menu: None,
            controller: value.controller,
            edit_transform,
            edit_changed: value.edit_changed,
            size: value.size,
            style: value.style,
            placeholder: value.placeholder,
            placeholder_color: value.placeholder_color,
            focused_border: value.focused_border,
            focused_border_outset: value.focused_border_outset,
            content_padding: value.content_padding,
            on_editing_complete: value.on_editing_complete,
            on_submit: value.on_submit,
            multiline: value.multiline,
            min_lines: value.min_lines,
            max_lines: value.max_lines,
            expands: value.expands,
            text_align: value.text_align,
            enabled: value.enabled,
            read_only: value.read_only,
            obscure_text: value.obscure_text,
            cursor_width: value.cursor_width,
            cursor_height: value.cursor_height,
            cursor_radius: value.cursor_radius,
            show_cursor: value.show_cursor.unwrap_or(!value.read_only),
            cursor_color: value.cursor_color,
            selection_color: value.selection_color,
            text_direction: value.text_direction,
            paragraph_layout: value.paragraph_layout,
            traits: incular_platform::TextInputTraits {
                autocorrect: value
                    .autocorrect
                    .unwrap_or_else(|| infer_autocorrect(value.autofill_hints.as_deref())),
                enable_suggestions: value.enable_suggestions,
                enable_interactive_selection: value.enable_interactive_selection.unwrap_or(true)
                    && (!value.read_only || !value.obscure_text),
                text_capitalization: value.text_capitalization,
                keyboard_appearance: value.keyboard_appearance,
                autofill_hints: value.autofill_hints,
                enable_ime_personalized_learning: value.enable_ime_personalized_learning,
            },
            scroll_padding: value.scroll_padding,
            cursor_opacity_animates: value.cursor_opacity_animates,
        };
        let field = move |context_menu| {
            let mut spec = spec.clone();
            spec.context_menu = context_menu;
            Widget::from_kind(WidgetKind::TextField(Box::new(spec)))
                .with_text_input_hints(input_type, input_action)
        };
        match context_menu_builder {
            Some(builder) => crate::context_menu::with_text_context_menu(
                move |state| field(Some(state)),
                builder,
            ),
            None => field(None),
        }
    }
}

/// Declarative retained group opacity. The child is painted into an isolated
/// compositor target when alpha is between zero and one, so overlapping
/// descendants are attenuated exactly once.
///
/// Alpha does not affect hit testing. As in Flutter, a zero alpha hides the
/// child's semantics unless [`Opacity::always_include_semantics`] is set; any
/// other alpha keeps them exposed. Use `Visibility` to remove input too.
#[derive(Clone, Debug, PartialEq)]
pub struct Opacity {
    alpha: f32,
    controller: Option<OpacityController>,
    always_include_semantics: bool,
    child: Widget,
}
impl Opacity {
    #[must_use]
    pub fn new(alpha: f32, child: impl Into<Widget>) -> Self {
        Self {
            alpha: normalize_opacity(alpha),
            controller: None,
            always_include_semantics: false,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn controlled(controller: OpacityController, child: impl Into<Widget>) -> Self {
        Self {
            alpha: controller.opacity(),
            controller: Some(controller),
            always_include_semantics: false,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn controller(mut self, controller: OpacityController) -> Self {
        self.alpha = controller.opacity();
        self.controller = Some(controller);
        self
    }
    #[must_use]
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = normalize_opacity(alpha);
        self
    }
    /// Flutter's `alwaysIncludeSemantics`: keep the child's semantics even
    /// when the alpha is zero.
    #[must_use]
    pub fn always_include_semantics(mut self, include: bool) -> Self {
        self.always_include_semantics = include;
        self
    }
}
impl From<Opacity> for Widget {
    fn from(value: Opacity) -> Self {
        let widget = match value.controller {
            Some(controller) => Widget::controlled_opacity(controller, value.child),
            None => Widget::opacity(value.alpha, value.child),
        };
        widget.always_include_semantics(value.always_include_semantics)
    }
}

/// A retained opacity transition driven directly by an [`OpacityController`].
/// Same input contract as [`Opacity`]: the animated alpha attenuates pixels
/// and hides semantics only at zero, unless
/// [`FadeTransition::always_include_semantics`] is set. It never gates hit
/// testing.
#[derive(Clone, Debug, PartialEq)]
pub struct FadeTransition {
    controller: OpacityController,
    always_include_semantics: bool,
    child: Widget,
}
impl FadeTransition {
    #[must_use]
    pub fn new(controller: OpacityController, child: impl Into<Widget>) -> Self {
        Self {
            controller,
            always_include_semantics: false,
            child: child.into(),
        }
    }
    /// Flutter's `alwaysIncludeSemantics`.
    #[must_use]
    pub fn always_include_semantics(mut self, include: bool) -> Self {
        self.always_include_semantics = include;
        self
    }
}
impl From<FadeTransition> for Widget {
    fn from(value: FadeTransition) -> Self {
        Widget::controlled_opacity(value.controller, value.child)
            .always_include_semantics(value.always_include_semantics)
    }
}

/// A retained translation transition driven directly by a
/// [`TranslationController`].
#[derive(Clone, Debug, PartialEq)]
pub struct SlideTransition {
    controller: TranslationController,
    child: Widget,
}
impl SlideTransition {
    #[must_use]
    pub fn new(controller: TranslationController, child: impl Into<Widget>) -> Self {
        Self {
            controller,
            child: child.into(),
        }
    }
}
impl From<SlideTransition> for Widget {
    fn from(value: SlideTransition) -> Self {
        Widget::translate(value.controller, value.child)
    }
}

/// An arbitrary retained affine transform. Layout remains the child's normal
/// layout; only the compositor, pointer coordinate conversion, and semantic
/// bounds observe the affine transform.
#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    transform: CoreTransform,
    origin: Option<Offset>,
    /// `Transform.scale`/`Transform.rotate` default to `Alignment.center`;
    /// the plain `Transform` constructor has no alignment and pivots about the
    /// top-left corner.
    centered: bool,
    alignment: Option<Alignment>,
    filter_quality: Option<incular_rendering::FilterQuality>,
    child: Widget,
}
impl Transform {
    /// Flutter's `Transform(transform: ...)`: without an origin the matrix is
    /// applied about the top-left corner.
    #[must_use]
    pub fn new(transform: CoreTransform, child: impl Into<Widget>) -> Self {
        Self {
            transform,
            origin: None,
            centered: false,
            alignment: None,
            filter_quality: None,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn translation(offset: Offset, child: impl Into<Widget>) -> Self {
        Self::new(CoreTransform::translation(offset), child)
    }
    /// Flutter's `Transform.scale`: pivots about the center by default.
    #[must_use]
    pub fn scale(scale: f32, child: impl Into<Widget>) -> Self {
        let mut transform = Self::new(CoreTransform::scale(scale), child);
        transform.centered = true;
        transform
    }
    /// Flutter's `Transform.rotate`: pivots about the center by default.
    #[must_use]
    pub fn rotation(radians: f32, child: impl Into<Widget>) -> Self {
        let mut transform = Self::new(CoreTransform::rotation(radians), child);
        transform.centered = true;
        transform
    }
    #[must_use]
    pub fn skew(x: f32, y: f32, child: impl Into<Widget>) -> Self {
        Self::new(CoreTransform::skew(x, y), child)
    }
    /// Selects the local pivot. The default is the child's center for
    /// [`Transform::scale`] and [`Transform::rotation`] (Flutter's
    /// `alignment: Alignment.center`) and the top-left corner otherwise.
    #[must_use]
    pub fn origin(mut self, origin: Offset) -> Self {
        self.origin = Some(Offset::finite_or_zero(origin));
        self
    }
    /// Flutter's `Transform.alignment`: the pivot as a point of the child's
    /// box, added to any [`Transform::origin`].
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
    /// Flutter's `filterQuality`: when set, the child is
    /// rasterized and resampled through the transform with this quality
    /// (Flutter applies it as `ImageFilter.matrix`), instead of being drawn
    /// through it. `None`, the default, draws through the transform.
    #[must_use]
    pub fn filter_quality(mut self, quality: incular_rendering::FilterQuality) -> Self {
        self.filter_quality = Some(quality);
        self
    }
}
impl From<Transform> for Widget {
    fn from(value: Transform) -> Self {
        if let Some(alignment) = value.alignment {
            // `alignment.alongSize(size)` needs the child's size, so the pivot
            // shift is two fractional translations around a transform about
            // `origin`: T(a·size) T(origin) M T(-origin) T(-a·size).
            let fraction = Offset::new((alignment.x + 1.) / 2., (alignment.y + 1.) / 2.);
            let inner = crate::FractionalTranslation::new(
                Offset::new(-fraction.x, -fraction.y),
                value.child,
            );
            let transformed = Widget::transform_around(
                value.transform,
                value.origin.unwrap_or(Offset::ZERO),
                inner.into(),
            )
            .with_transform_filter_quality(value.filter_quality);
            return crate::FractionalTranslation::new(fraction, transformed).into();
        }
        match value.origin {
            Some(origin) => Widget::transform_around(value.transform, origin, value.child),
            None if value.centered => Widget::transform(value.transform, value.child),
            None => Widget::transform_around(value.transform, Offset::ZERO, value.child),
        }
        .with_transform_filter_quality(value.filter_quality)
    }
}

/// A compositor-only scale transition driven by [`ScaleController`].
#[derive(Clone, Debug, PartialEq)]
pub struct ScaleTransition {
    controller: ScaleController,
    filter_quality: Option<incular_rendering::FilterQuality>,
    child: Widget,
}
impl ScaleTransition {
    #[must_use]
    pub fn new(controller: ScaleController, child: impl Into<Widget>) -> Self {
        Self {
            controller,
            filter_quality: None,
            child: child.into(),
        }
    }
    /// Flutter's `filterQuality`: the quality the scaled child is
    /// resampled with while the controller animates. It is ignored while
    /// the scale is at rest, as in Flutter.
    #[must_use]
    pub fn filter_quality(mut self, quality: incular_rendering::FilterQuality) -> Self {
        self.filter_quality = Some(quality);
        self
    }
}
impl From<ScaleTransition> for Widget {
    fn from(value: ScaleTransition) -> Self {
        Widget::controlled_scale(value.controller, value.child)
            .with_transform_filter_quality(value.filter_quality)
    }
}

/// A compositor-only clockwise rotation transition driven by
/// [`RotationController`].
#[derive(Clone, Debug, PartialEq)]
pub struct RotationTransition {
    controller: RotationController,
    alignment: Alignment,
    filter_quality: Option<incular_rendering::FilterQuality>,
    child: Widget,
}
impl RotationTransition {
    #[must_use]
    pub fn new(controller: RotationController, child: impl Into<Widget>) -> Self {
        Self {
            controller,
            alignment: Alignment::CENTER,
            filter_quality: None,
            child: child.into(),
        }
    }

    /// Creates a rotation from a normalized turns value without exposing the
    /// retained controller implementation. One turn is a full revolution.
    #[must_use]
    pub fn from_turns(turns: f32, child: impl Into<Widget>) -> Self {
        let controller = RotationController::new();
        controller.set_radians(if turns.is_finite() {
            turns * std::f32::consts::TAU
        } else {
            0.
        });
        Self::new(controller, child)
    }

    /// Selects the normalized pivot for the compositor transform. The
    /// default is the child's center, matching Flutter's transition.
    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
    /// Flutter's `filterQuality`: the quality the rotated child is
    /// resampled with while the controller animates. It is ignored while
    /// the rotation is at rest, as in Flutter.
    #[must_use]
    pub fn filter_quality(mut self, quality: incular_rendering::FilterQuality) -> Self {
        self.filter_quality = Some(quality);
        self
    }
}
impl From<RotationTransition> for Widget {
    fn from(value: RotationTransition) -> Self {
        // Resolve alignment against the retained child size at compositor
        // update time; a turns change therefore does not relayout or repaint
        // the child subtree.
        Widget::controlled_rotation_with_alignment(
            value.controller,
            Some(value.alignment),
            value.child,
        )
        .with_transform_filter_quality(value.filter_quality)
    }
}

/// A compact composition surface for the retained transition primitives.
/// It intentionally merges fade and slide behavior instead of creating a
/// large hierarchy of narrowly different animation widget types.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    opacity: Option<OpacityController>,
    translation: Option<TranslationController>,
    child: Widget,
}
impl Transition {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            opacity: None,
            translation: None,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn fade(mut self, controller: OpacityController) -> Self {
        self.opacity = Some(controller);
        self
    }
    #[must_use]
    pub fn slide(mut self, controller: TranslationController) -> Self {
        self.translation = Some(controller);
        self
    }
}
impl From<Transition> for Widget {
    fn from(value: Transition) -> Self {
        let child = match value.translation {
            Some(controller) => Widget::translate(controller, value.child),
            None => value.child,
        };
        match value.opacity {
            Some(controller) => Widget::controlled_opacity(controller, child),
            None => child,
        }
    }
}

/// Declarative Gaussian blur isolation. Sigma is in logical pixels and is
/// converted to physical pixels by the renderer at the current DPI.
#[derive(Clone, Debug, PartialEq)]
pub struct Blur {
    sigma_x: f32,
    sigma_y: f32,
    controller: Option<BlurController>,
    child: Widget,
}
impl Blur {
    #[must_use]
    pub fn new(sigma: f32, child: impl Into<Widget>) -> Self {
        Self {
            sigma_x: normalize_sigma(sigma),
            sigma_y: normalize_sigma(sigma),
            controller: None,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn asymmetric(sigma_x: f32, sigma_y: f32, child: impl Into<Widget>) -> Self {
        Self {
            sigma_x: normalize_sigma(sigma_x),
            sigma_y: normalize_sigma(sigma_y),
            controller: None,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn controlled(controller: BlurController, child: impl Into<Widget>) -> Self {
        Self {
            sigma_x: controller.sigma(),
            sigma_y: controller.sigma(),
            controller: Some(controller),
            child: child.into(),
        }
    }
    #[must_use]
    pub fn sigma_x(mut self, sigma: f32) -> Self {
        self.sigma_x = normalize_sigma(sigma);
        self.controller = None;
        self
    }
    #[must_use]
    pub fn sigma_y(mut self, sigma: f32) -> Self {
        self.sigma_y = normalize_sigma(sigma);
        self.controller = None;
        self
    }
}
impl From<Blur> for Widget {
    fn from(value: Blur) -> Self {
        match value.controller {
            Some(controller) => Widget::controlled_blur(controller, value.child),
            None => Widget::asymmetric_blur(value.sigma_x, value.sigma_y, value.child),
        }
    }
}

/// Declarative arbitrary-subtree drop shadow. The source subtree is isolated
/// and its alpha is blurred, so text, images, gradients, and paths all share
/// the same shadow semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct DropShadow {
    offset: Offset,
    sigma_x: f32,
    sigma_y: f32,
    color: Color,
    controller: Option<DropShadowController>,
    child: Widget,
}
impl DropShadow {
    #[must_use]
    pub fn new(offset: Offset, sigma: f32, color: Color, child: impl Into<Widget>) -> Self {
        Self {
            offset: Offset::finite_or_zero(offset),
            sigma_x: normalize_sigma(sigma),
            sigma_y: normalize_sigma(sigma),
            color,
            controller: None,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn asymmetric(
        offset: Offset,
        sigma_x: f32,
        sigma_y: f32,
        color: Color,
        child: impl Into<Widget>,
    ) -> Self {
        Self {
            offset: Offset::finite_or_zero(offset),
            sigma_x: normalize_sigma(sigma_x),
            sigma_y: normalize_sigma(sigma_y),
            color,
            controller: None,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn controlled(controller: DropShadowController, child: impl Into<Widget>) -> Self {
        Self {
            offset: controller.offset(),
            sigma_x: controller.sigma(),
            sigma_y: controller.sigma(),
            color: controller.color(),
            controller: Some(controller),
            child: child.into(),
        }
    }
    #[must_use]
    pub fn offset(mut self, offset: Offset) -> Self {
        self.offset = Offset::finite_or_zero(offset);
        self.controller = None;
        self
    }
    #[must_use]
    pub fn sigma(mut self, sigma: f32) -> Self {
        self.sigma_x = normalize_sigma(sigma);
        self.sigma_y = normalize_sigma(sigma);
        self.controller = None;
        self
    }
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.controller = None;
        self
    }
}
impl From<DropShadow> for Widget {
    fn from(value: DropShadow) -> Self {
        match value.controller {
            Some(controller) => Widget::controlled_drop_shadow(controller, value.child),
            None => Widget::drop_shadow(value.offset, value.sigma_x, value.color, value.child),
        }
    }
}

/// Declarative 4x5 color-matrix stage. The matrix is evaluated in straight
/// RGBA and converted back to the retained premultiplied texture format.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorFiltered {
    filter: ColorFilter,
    controller: Option<ColorFilterController>,
    child: Widget,
}
impl ColorFiltered {
    #[must_use]
    pub fn new(filter: ColorFilter, child: impl Into<Widget>) -> Self {
        Self {
            filter,
            controller: None,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn controlled(controller: ColorFilterController, child: impl Into<Widget>) -> Self {
        Self {
            filter: controller.filter(),
            controller: Some(controller),
            child: child.into(),
        }
    }
    #[must_use]
    pub fn matrix(mut self, matrix: [f32; 20]) -> Self {
        self.filter = ColorFilter::matrix(matrix);
        self.controller = None;
        self
    }
    #[must_use]
    pub fn filter(mut self, filter: ColorFilter) -> Self {
        self.filter = filter;
        self.controller = None;
        self
    }
    #[must_use]
    pub fn controller(mut self, controller: ColorFilterController) -> Self {
        self.filter = controller.filter();
        self.controller = Some(controller);
        self
    }
}
impl From<ColorFiltered> for Widget {
    fn from(value: ColorFiltered) -> Self {
        match value.controller {
            Some(controller) => Widget::controlled_color_filtered(controller, value.child),
            None => Widget::color_filtered(value.filter, value.child),
        }
    }
}

/// Declarative retained blend group. Destination-dependent modes promote only
/// the required composition scope to a sampleable intermediate target.
#[derive(Clone, Debug, PartialEq)]
pub struct Blend {
    mode: BlendMode,
    child: Widget,
}
impl Blend {
    #[must_use]
    pub fn new(mode: BlendMode, child: impl Into<Widget>) -> Self {
        Self {
            mode,
            child: child.into(),
        }
    }
    #[must_use]
    pub fn mode(mut self, mode: BlendMode) -> Self {
        self.mode = mode;
        self
    }
}
impl From<Blend> for Widget {
    fn from(value: Blend) -> Self {
        Widget::blend(value.mode, value.child)
    }
}

/// Small composable builder for ordered retained effects. Each method wraps
/// the current child, so calls read in the same order as execution.
#[derive(Clone, Debug, PartialEq)]
pub struct Effects {
    child: Widget,
}
impl Effects {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }
    #[must_use]
    pub fn color_filter(mut self, filter: ColorFilter) -> Self {
        self.child = Widget::color_filtered(filter, self.child);
        self
    }
    #[must_use]
    pub fn color_matrix(self, filter: ColorFilter) -> Self {
        self.color_filter(filter)
    }
    #[must_use]
    pub fn blur(mut self, sigma: f32) -> Self {
        self.child = Widget::blur(sigma, self.child);
        self
    }
    #[must_use]
    pub fn asymmetric_blur(mut self, sigma_x: f32, sigma_y: f32) -> Self {
        self.child = Widget::asymmetric_blur(sigma_x, sigma_y, self.child);
        self
    }
    #[must_use]
    pub fn opacity(mut self, alpha: f32) -> Self {
        self.child = Widget::opacity(alpha, self.child);
        self
    }
    #[must_use]
    pub fn drop_shadow(mut self, offset: Offset, sigma: f32, color: Color) -> Self {
        self.child = Widget::drop_shadow(offset, sigma, color, self.child);
        self
    }
    #[must_use]
    pub fn blend(mut self, mode: BlendMode) -> Self {
        self.child = Widget::blend(mode, self.child);
        self
    }
    #[must_use]
    pub fn build(self) -> Widget {
        self.child
    }
}
impl From<Effects> for Widget {
    fn from(value: Effects) -> Self {
        value.build()
    }
}

/// Vertical retained viewport. Keep a [`ScrollController`] outside a rebuild
/// when application code needs the position to survive a recreated description.
pub struct ScrollView;
impl ScrollView {
    #[must_use]
    pub fn vertical(controller: ScrollController, child: impl Into<Widget>) -> Widget {
        Widget::scroll_view(controller, child.into())
    }
}
