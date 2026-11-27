//! Concrete retained-widget descriptors.
//!
//! Public descriptor types lower into the opaque `Widget` transport. Mutable mounted state continues to live exclusively in `WidgetTree`.

use super::*;
use crate::tree::specs::{AppPrivateCommandCallback, EditableOutsideTap, TextParagraphPolicy};

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
    pub(crate) alignment: crate::AlignmentGeometry,
    pub(crate) sampling: ImageSampling,
    pub(crate) center_slice: Option<Rect>,
    pub(crate) match_text_direction: bool,
    /// Flutter `color`: a tint blended over the image.
    pub(crate) color: Option<Color>,
    /// Flutter `colorBlendMode`; `None` is Flutter's `srcIn`.
    pub(crate) color_blend_mode: Option<BlendMode>,
    pub(crate) destination_blend_mode: BlendMode,
    pub(crate) invert_colors: bool,
    pub(crate) opacity: f32,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            scale: 1.0,
            fit: None,
            repeat: ImageRepeat::NoRepeat,
            alignment: Alignment::CENTER.into(),
            sampling: ImageSampling::Linear,
            center_slice: None,
            match_text_direction: false,
            color: None,
            color_blend_mode: None,
            destination_blend_mode: BlendMode::SrcOver,
            invert_colors: false,
            opacity: 1.0,
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
        pub fn alignment(mut self, alignment: impl Into<$crate::AlignmentGeometry>) -> Self {
            self.options.alignment = alignment.into();
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

    /// Displays frames from an already-resolved, possibly animated source.
    #[must_use]
    pub fn playback(playback: incular_image::ImagePlayback) -> crate::ProviderImage {
        crate::ProviderImage::from_playback(playback)
    }

    /// Loads a resolution-aware asset from the closest asset bundle.
    #[must_use]
    pub fn asset(path: impl Into<std::path::PathBuf>) -> crate::ProviderImage {
        Self::provider(crate::AssetImage::new(path))
    }

    /// Decodes encoded image bytes through a memory image provider.
    #[must_use]
    pub fn memory(bytes: impl Into<std::sync::Arc<[u8]>>) -> crate::ProviderImage {
        Self::provider(crate::MemoryImage::new(bytes))
    }

    pub(crate) fn with_options(image: ImageHandle, options: ImageOptions) -> Self {
        Self { image, options }
    }

    image_option_setters!();
}
impl From<Image> for Widget {
    fn from(value: Image) -> Self {
        Widget::image(Some(value.image), value.options)
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
        if value.decoration.as_ref().is_some_and(|decoration| {
            decoration.directional_border_radius.is_some() && !decoration.has_aligned_gradient()
        }) {
            return crate::Builder::new(move |context| {
                let direction = context
                    .text_direction()
                    .expect("validated directional DecoratedBox");
                let mut resolved = value.clone();
                if let Some(decoration) = resolved.decoration.take() {
                    let decoration = decoration.resolved_for_direction(direction);
                    if let Some(radius) = decoration.border_radius {
                        resolved.radius = radius.to_corner_radii();
                    }
                    resolved.decoration = Some(decoration);
                }
                resolved.into()
            })
            .require_directionality(&crate::tree::DirectionRequirement {
                widget: "DecoratedBox",
                reason: crate::BoxDecoration::DIRECTIONAL_RADIUS_NEEDS_DIRECTION,
            })
            .into();
        }
        if let Some(decoration) = &value.decoration
            && (value.position == crate::DecorationPosition::Foreground
                || decoration.needs_full_painter())
        {
            let child = match value.size {
                Some(size) => crate::SizedBox::new()
                    .width(size.width)
                    .height(size.height)
                    .child(value.child)
                    .into(),
                None => value.child,
            };
            return crate::painting_effects::decorated(
                value
                    .decoration
                    .expect("description of the full decoration"),
                value.position,
                child,
            );
        }
        Widget::decorated_with_hit(
            value.size,
            value.background,
            value.border,
            value.radius,
            value.child,
            value.decoration.map(Rc::new),
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
    style: Arc<TextStyle>,
    align: Option<TextAlign>,
    soft_wrap: Option<bool>,
    max_lines: Option<Option<usize>>,
    overflow: Option<TextOverflow>,
    text_direction: Option<incular_config::TextDirection>,
    text_height_behavior: Option<TextHeightBehavior>,
    text_scaler: Option<TextScaler>,
    strut: Option<StrutStyle>,
    locale: Option<incular_config::Locale>,
    semantics_identifier: Option<String>,
    semantics_label: Option<String>,
}
impl Text {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        static DEFAULT_STYLE: std::sync::LazyLock<Arc<TextStyle>> =
            std::sync::LazyLock::new(|| Arc::new(TextStyle::default()));
        Self {
            text: text.into(),
            style: DEFAULT_STYLE.clone(),
            align: None,
            soft_wrap: None,
            max_lines: None,
            overflow: None,
            text_direction: None,
            text_height_behavior: None,
            text_scaler: None,
            strut: None,
            locale: None,
            semantics_identifier: None,
            semantics_label: None,
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
    /// Flutter's `Text.textScaler`: scales the resolved font size. Without it
    /// the text follows the system text scale, as Flutter's
    /// `MediaQuery.textScalerOf(context)` does; pass [`TextScaler::no_scaling`]
    /// to opt out.
    #[must_use]
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> Self {
        self.text_scaler = Some(text_scaler);
        self
    }
    /// Overrides the locale used for shaping and for the spoken text label.
    #[must_use]
    pub fn locale(mut self, locale: incular_config::Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Sets the label announced by assistive technology without changing
    /// visible text. The replacement does not reuse the visible text's
    /// pronunciation spans.
    #[must_use]
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    /// Publishes an accessibility identifier on the retained text node.
    #[must_use]
    pub fn semantics_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.semantics_identifier = Some(identifier.into());
        self
    }

    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = Arc::new(style);
        self
    }
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        Arc::make_mut(&mut self.style).color = color;
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
        use incular_semantics::{
            SemanticRole, SemanticState, SemanticTextAttributes, StringAttribute,
        };

        let mut style = value.style;
        let text_attributes = value.locale.and_then(|locale| {
            let tag = locale.to_string();
            Arc::make_mut(&mut style).locale = Some(tag.clone());
            (!value.text.is_empty()).then(|| {
                Arc::new(SemanticTextAttributes {
                    label: vec![StringAttribute::locale(
                        0..value.text.encode_utf16().count(),
                        tag,
                    )],
                    ..Default::default()
                })
            })
        });
        let mut widget = Widget::text_configured(
            value.text,
            style,
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
        );
        if let Some(attributes) = text_attributes {
            widget.semantic_properties_mut().implicit_text_attributes = Some(attributes);
        }
        if let Some(identifier) = value.semantics_identifier {
            widget = widget.semantics(
                crate::ExplicitSemantics::new(SemanticRole::GenericContainer).state(SemanticState {
                    identifier: Some(identifier),
                    ..SemanticState::default()
                }),
            );
        }
        if let Some(label) = value.semantics_label {
            widget = widget.accessibility_label(label);
        }
        widget
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

/// A [`RichText`] whose [`WidgetSpan`](crate::WidgetSpan)s are filled with widgets, as Flutter's
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

    /// Fills the [`WidgetSpan`](crate::WidgetSpan) with this `id` (Flutter `WidgetSpan.child`).
    /// An id no span carries adds nothing.
    #[must_use]
    pub fn child(mut self, id: u64, child: impl Into<Widget>) -> Self {
        self.children.push((id, child.into()));
        self
    }

    /// Compares a paragraph's spans and actual inline child descriptions.
    /// Editing an inline child's subtree requires layout even when it keeps
    /// its placeholder identity and its last measured width.
    #[must_use]
    pub fn compare_spans(&self, other: &Self) -> incular_text::InlineSpanComparison {
        use incular_text::InlineSpanComparison;
        if self.children != other.children {
            InlineSpanComparison::Layout
        } else {
            self.text.text.compare_to(&other.text.text)
        }
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
    let has_placeholders = !placeholders.is_empty();
    let inline =
        has_placeholders.then(|| {
            let capacity = children.len().min(placeholders.len());
            let mut slots = Vec::with_capacity(capacity);
            let mut widgets = Vec::with_capacity(capacity);
            for (id, child) in children {
                // The first span with this id that no earlier child filled.
                if let Some(slot) = placeholders.iter().enumerate().find_map(|(slot, span)| {
                    (span.id == id && !slots.contains(&slot)).then_some(slot)
                }) {
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
    let spans: Option<Arc<[StyledTextRun]>> = Some(runs.into());
    let mut widget = Widget::text_configured(
        value.plain_text(),
        Arc::new(style),
        TextParagraphPolicy {
            align: Some(value.text_align),
            soft_wrap: Some(value.soft_wrap),
            max_lines: Some(value.max_lines),
            overflow: Some(value.overflow),
        },
        None,
        None,
        Some(value.text_scaler),
        spans,
        inline,
        strut.map(Box::new),
    );
    let parts = super::semantics::paragraph::ParagraphSemanticPart::for_spans(&value.text);
    let pronunciation = if parts.is_none() {
        paragraph_pronunciation(&value.text)
    } else {
        None
    };
    let label = if parts.is_some() || pronunciation.is_some() {
        None
    } else if has_placeholders {
        // Replacement characters are layout protocol bytes, not speech.
        // An actual inline child contributes its own semantics if placed.
        Some(value.text.to_plain_text(incular_text::PlainTextOptions {
            include_semantics_labels: true,
            include_placeholders: false,
        }))
    } else {
        value.text.semantics_text()
    };
    if let WidgetKind::Text { pointer_spans, .. } = widget.kind_mut() {
        *pointer_spans = paragraph_mouse_spans(value.text);
    }
    if let Some(parts) = parts {
        widget.semantic_properties_mut().paragraph_parts = Some(parts);
        widget
    } else if let Some(pronunciation) = pronunciation {
        crate::Semantics::new(widget).attributed_label(pronunciation).into()
    } else if let Some(label) = label {
        widget.accessibility_label(label)
    } else {
        widget
    }
}

fn paragraph_mouse_spans(root: incular_text::TextSpan) -> Option<Arc<ParagraphMouseSpans>> {
    fn has_annotation(span: &incular_text::TextSpan) -> bool {
        span.mouse.is_some() && !span.text.is_empty()
            || span.children.iter().any(|child| {
                matches!(child, incular_text::InlineSpan::Text(child) if has_annotation(child))
            })
    }

    fn collect(
        span: &incular_text::TextSpan,
        offset: &mut usize,
        path: &mut Vec<usize>,
        ranges: &mut Vec<ParagraphMouseRange>,
    ) {
        let end = *offset + span.text.len();
        if end > *offset && let Some(annotation) = &span.mouse {
            ranges.push(ParagraphMouseRange {
                source: *offset..end,
                path: path.clone(),
                annotation: annotation.clone(),
            });
        }
        *offset = end;
        for (index, child) in span.children.iter().enumerate() {
            match child {
                incular_text::InlineSpan::Text(child) => {
                    path.push(index);
                    collect(child, offset, path, ranges);
                    path.pop();
                }
                incular_text::InlineSpan::Widget(_) => *offset += '\u{fffc}'.len_utf8(),
            }
        }
    }

    if !has_annotation(&root) {
        return None;
    }
    let mut ranges = Vec::new();
    collect(&root, &mut 0, &mut Vec::new(), &mut ranges);
    Some(Arc::new(ParagraphMouseSpans { root, ranges }))
}

fn paragraph_pronunciation(
    span: &incular_text::TextSpan,
) -> Option<incular_semantics::AttributedString> {
    use incular_semantics::AttributedString;
    use incular_text::{InlineSpan, combine_inline_span_semantics};

    fn has_pronunciation(span: &incular_text::TextSpan) -> bool {
        span.locale.is_some()
            || span.spell_out == Some(true)
            || span.children.iter().any(|child| {
                matches!(child, InlineSpan::Text(span) if has_pronunciation(span))
            })
    }

    if !has_pronunciation(span) {
        return None;
    }
    let mut fragments = combine_inline_span_semantics(&span.semantics_information());
    // An inline widget or named semantics span is its own fragment in
    // Flutter. Keep the existing flat fallback until those nodes are
    // projected separately instead of publishing incorrect shared ranges.
    let [fragment] = fragments.as_mut_slice() else {
        return None;
    };
    if fragment.string_attributes.is_empty() {
        return None;
    }
    let attributes = fragment.string_attributes.drain(..).map(paragraph_string_attribute);
    AttributedString::try_with_attributes(std::mem::take(&mut fragment.label), attributes).ok()
}

pub(super) fn paragraph_string_attribute(
    attribute: incular_text::InlineSpanStringAttribute<'_>,
) -> incular_semantics::StringAttribute {
    use incular_semantics::StringAttribute;
    use incular_text::InlineSpanStringAttribute;
    match attribute {
        InlineSpanStringAttribute::SpellOut(range) => StringAttribute::spell_out(range),
        InlineSpanStringAttribute::Locale { range, locale } => {
            StringAttribute::locale(range, locale.to_string())
        }
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
    focus_node: Option<crate::FocusNode>,
    autofocus: bool,
    select_all_on_focus: Option<bool>,
    size: Size,
    style: TextStyle,
    text_scaler: Option<TextScaler>,
    strut: Option<Box<StrutStyle>>,
    placeholder: String,
    placeholder_color: Color,
    focused_border: Option<(incular_rendering::Border, f32)>,
    focused_border_outset: incular_config::EdgeInsets,
    content_padding: incular_config::EdgeInsets,
    clip_behavior: incular_config::Clip,
    fixed_scroll_offset: Option<f32>,
    on_editing_complete: Option<Rc<dyn Fn()>>,
    on_submit: Option<Rc<dyn Fn(String)>>,
    on_tap_outside: Option<EditableOutsideTap>,
    on_tap_up_outside: Option<EditableOutsideTap>,
    on_app_private_command: Option<AppPrivateCommandCallback>,
    content_insertion: Option<crate::ContentInsertionConfiguration>,
    multiline: bool,
    min_lines: Option<usize>,
    max_lines: Option<usize>,
    expands: bool,
    force_line: bool,
    text_width_basis: incular_text::TextWidthBasis,
    text_align: TextAlign,
    enabled: bool,
    read_only: bool,
    obscure_text: bool,
    obscuring_character: char,
    cursor_width: f32,
    cursor_height: Option<f32>,
    cursor_radius: f32,
    /// `None` resolves to `!read_only` when the field is lowered, as in
    /// Flutter's `showCursor ?? !readOnly`.
    show_cursor: Option<bool>,
    paint_cursor_above_text: bool,
    cursor_color: Color,
    background_cursor_color: Option<Color>,
    selection_color: Option<Color>,
    autocorrection_text_rect_color: Option<Color>,
    input_type: Option<TextInputTypeHint>,
    input_action: TextInputActionHint,
    text_direction: Option<incular_config::TextDirection>,
    locale: Option<incular_config::Locale>,
    paragraph_layout: bool,
    /// `None` resolves like Flutter's `_inferAutocorrect`.
    autocorrect: Option<bool>,
    smart_dashes_type: Option<crate::SmartDashesType>,
    smart_quotes_type: Option<crate::SmartQuotesType>,
    enable_inline_prediction: Option<bool>,
    enable_suggestions: bool,
    /// `None` resolves to `!read_only || !obscure_text`.
    enable_interactive_selection: Option<bool>,
    text_capitalization: incular_platform::TextCapitalization,
    keyboard_appearance: incular_config::Brightness,
    autofill_hints: Option<Vec<String>>,
    hint_locales: Option<Vec<incular_config::Locale>>,
    enable_ime_personalized_learning: bool,
    scroll_padding: incular_config::EdgeInsets,
    cursor_opacity_animates: bool,
    /// `None` resolves to [`ToolbarOptions::for_field`].
    toolbar_options: Option<ToolbarOptions>,
    /// Flutter's `contextMenuBuilder`; `None` shows no context menu.
    context_menu_builder: Option<crate::EditableTextContextMenuBuilder>,
    selection_handles_builder: Option<crate::TextSelectionHandlesBuilder>,
    collapsed_selection_handle_layout: Option<crate::TextSelectionHandleLayout>,
    edit_transform: Option<super::specs::EditTransform>,
    edit_changed: Option<super::specs::EditChanged>,
    edit_selection_changed: Option<super::specs::EditSelectionChanged>,
    /// Flutter `spellCheckConfiguration`.
    spell_check: Option<crate::SpellCheckConfiguration>,
    /// Input formatters, applied in the order added to each user edit.
    input_formatters: Vec<Rc<dyn crate::TextInputFormatter>>,
}
impl EditableText {
    #[must_use]
    pub fn new(controller: TextEditingController) -> Self {
        Self {
            controller,
            focus_node: None,
            autofocus: false,
            select_all_on_focus: None,
            size: Size::ZERO,
            style: TextStyle::default(),
            text_scaler: None,
            strut: None,
            placeholder: String::new(),
            placeholder_color: Color::rgba(150, 154, 170, 255),
            focused_border: None,
            focused_border_outset: incular_config::EdgeInsets::ZERO,
            content_padding: incular_config::EdgeInsets::ZERO,
            clip_behavior: incular_config::Clip::HardEdge,
            fixed_scroll_offset: None,
            on_editing_complete: None,
            on_submit: None,
            on_tap_outside: None,
            on_tap_up_outside: None,
            on_app_private_command: None,
            content_insertion: None,
            multiline: false,
            min_lines: None,
            max_lines: Some(1),
            expands: false,
            force_line: true,
            text_width_basis: incular_text::TextWidthBasis::Parent,
            text_align: TextAlign::Start,
            enabled: true,
            read_only: false,
            obscure_text: false,
            obscuring_character: '•',
            // Flutter's EditableText default cursorWidth is 2.0.
            cursor_width: 2.0,
            cursor_height: None,
            cursor_radius: 0.0,
            show_cursor: None,
            paint_cursor_above_text: false,
            cursor_color: Color::WHITE,
            background_cursor_color: None,
            selection_color: None,
            autocorrection_text_rect_color: None,
            input_type: None,
            input_action: TextInputActionHint::Unspecified,
            text_direction: None,
            locale: None,
            paragraph_layout: false,
            autocorrect: None,
            smart_dashes_type: None,
            smart_quotes_type: None,
            enable_inline_prediction: None,
            enable_suggestions: true,
            enable_interactive_selection: None,
            text_capitalization: incular_platform::TextCapitalization::None,
            keyboard_appearance: incular_config::Brightness::Light,
            autofill_hints: Some(Vec::new()),
            hint_locales: None,
            enable_ime_personalized_learning: true,
            scroll_padding: incular_config::EdgeInsets::all(20.0),
            cursor_opacity_animates: false,
            toolbar_options: None,
            context_menu_builder: None,
            selection_handles_builder: None,
            collapsed_selection_handle_layout: None,
            edit_transform: None,
            edit_changed: None,
            edit_selection_changed: None,
            input_formatters: Vec::new(),
            spell_check: None,
        }
    }

    /// Binds the editor directly to an application-owned keyboard focus node.
    /// The node survives rebuilds when the application reuses the handle.
    #[must_use]
    pub fn focus_node(mut self, node: crate::FocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    /// Clips overflowing editable content, not its outer decoration or
    /// selection handles. The default is a hard edge; fitting text does
    /// not need a clip.
    #[must_use]
    pub fn clip_behavior(mut self, behavior: incular_config::Clip) -> Self {
        self.clip_behavior = behavior;
        self
    }

    /// Sets a caller-controlled viewport offset in logical pixels, like
    /// Flutter's `ViewportOffset.fixed`. It is horizontal for a single line
    /// and vertical for multiline text. Automatic caret reveal and scrolling
    /// leave a fixed offset untouched. Pass `None` to restore automatic
    /// scrolling; non-finite values are treated as unset.
    #[must_use]
    pub fn fixed_scroll_offset(mut self, pixels: impl Into<Option<f32>>) -> Self {
        self.fixed_scroll_offset = pixels.into().filter(|offset| offset.is_finite());
        self
    }

    /// Requests initial focus if the editor is eligible. Defaults to false.
    #[must_use]
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Overrides the platform's select-all behavior on external focus.
    #[must_use]
    pub fn select_all_on_focus(mut self, select: bool) -> Self {
        self.select_all_on_focus = Some(select);
        self
    }

    /// Flutter `spellCheckConfiguration`: checks the spelling of user
    /// edits with the configuration's service and styles the misspelled
    /// words. Checking runs only when the configuration is enabled and has
    /// both a service and a misspelled text style; obscured fields are
    /// never styled.
    #[must_use]
    pub fn spell_check_configuration(
        mut self,
        configuration: crate::SpellCheckConfiguration,
    ) -> Self {
        self.spell_check = Some(configuration);
        self
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

    /// Observes selection changes from keyboard input, pointer gestures,
    /// platform editing, the selection toolbar and accessibility actions.
    /// Changes made directly to the controller do not call this callback.
    #[must_use]
    pub fn on_selection_changed(
        mut self,
        callback: impl Fn(incular_text::TextSelection, incular_text::SelectionChangedCause) + 'static,
    ) -> Self {
        self.edit_selection_changed = Some(Rc::new(callback));
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

    /// Overrides the inherited text scaling for this editor only.
    #[must_use]
    pub fn text_scaler(mut self, scaler: TextScaler) -> Self {
        self.text_scaler = Some(scaler);
        self
    }

    /// Configures the editor's minimum or forced line metrics.
    #[must_use]
    pub fn strut_style(mut self, strut: StrutStyle) -> Self {
        self.strut = Some(Box::new(strut));
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
    /// Handles pointer-down outside the text-field group while this editor
    /// has focus. A supplied callback replaces the default outside blur.
    #[must_use]
    pub fn on_tap_outside(mut self, callback: impl Fn(incular_gestures::RawPointerEvent) + 'static) -> Self {
        self.on_tap_outside = Some(Rc::new(callback));
        self
    }

    /// Handles pointer-up outside the text-field group if this editor
    /// still has focus. Without a callback, release does nothing.
    #[must_use]
    pub fn on_tap_up_outside(mut self, callback: impl Fn(incular_gestures::RawPointerEvent) + 'static) -> Self {
        self.on_tap_up_outside = Some(Rc::new(callback));
        self
    }

    /// Receives an application-specific message from the active input method.
    /// Missing message data is delivered as an empty object. The editor does
    /// not interpret the action or change text or focus on its own.
    #[must_use]
    pub fn on_app_private_command(
        mut self,
        callback: impl Fn(&str, &serde_json::Map<String, serde_json::Value>) + 'static,
    ) -> Self {
        self.on_app_private_command = Some(Rc::new(callback));
        self
    }
    /// Enables rich keyboard content for the listed MIME types.
    #[must_use]
    pub fn content_insertion_configuration(
        mut self,
        configuration: crate::ContentInsertionConfiguration,
    ) -> Self {
        self.content_insertion = Some(configuration);
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

    /// Fills the incoming width by default. Set this to false to size the
    /// editor to its text and caret, within the parent's constraints.
    #[must_use]
    pub fn force_line(mut self, force_line: bool) -> Self {
        self.force_line = force_line;
        self
    }

    /// Selects how natural width is measured when `force_line` is false.
    /// Parent uses the unwrapped intrinsic width within the constraints;
    /// LongestLine uses the widest line after wrapping.
    #[must_use]
    pub fn text_width_basis(mut self, basis: incular_text::TextWidthBasis) -> Self {
        self.text_width_basis = basis;
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

    /// Flutter `obscuringCharacter`, default `•`. The mask must be one
    /// UTF-16 code unit so each edit position retains its visual slot.
    #[must_use]
    pub fn obscuring_character(mut self, character: char) -> Self {
        assert_eq!(
            character.len_utf16(),
            1,
            "obscuring character must be one UTF-16 code unit"
        );
        self.obscuring_character = character;
        self
    }

    #[must_use]
    pub fn get_obscuring_character(&self) -> char {
        self.obscuring_character
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

    /// The configured caret width in logical pixels.
    #[must_use]
    pub const fn configured_cursor_width(&self) -> f32 {
        self.cursor_width
    }

    /// The configured caret height in logical pixels, or `None` for line height.
    #[must_use]
    pub const fn configured_cursor_height(&self) -> Option<f32> {
        self.cursor_height
    }

    /// The configured circular corner radius; zero means a rectangular caret.
    #[must_use]
    pub const fn configured_cursor_radius(&self) -> f32 {
        self.cursor_radius
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

    /// Chooses whether the caret paints above the glyphs instead of behind them.
    #[must_use]
    pub fn paint_cursor_above_text(mut self, above: bool) -> Self {
        self.paint_cursor_above_text = above;
        self
    }

    /// Sets the caret paint color.
    #[must_use]
    pub fn cursor_color(mut self, color: Color) -> Self {
        self.cursor_color = color;
        self
    }

    /// Sets the regular caret color shown at the nearest text position
    /// while a floating caret is dragged far enough away from it.
    /// No background caret is painted unless a color is supplied.
    #[must_use]
    pub fn background_cursor_color(mut self, color: Color) -> Self {
        self.background_cursor_color = Some(color);
        self
    }

    /// Sets the selection highlight paint color. A raw editor does not
    /// paint a selection highlight unless a color is supplied.
    #[must_use]
    pub fn autocorrection_text_rect_color(mut self, color: Color) -> Self {
        self.autocorrection_text_rect_color = Some(color);
        self
    }

    #[must_use]
    pub fn selection_color(mut self, color: Color) -> Self {
        self.selection_color = Some(color);
        self
    }

    /// Suppresses the highlight without changing the logical selection.
    #[must_use]
    pub fn clear_selection_color(mut self) -> Self {
        self.selection_color = None;
        self
    }

    /// Selects the native keyboard/input method hint for this editor. The
    /// runtime still validates and owns the committed text value.
    #[must_use]
    pub fn input_type(mut self, input_type: TextInputTypeHint) -> Self {
        self.input_type = Some(input_type);
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

    /// Builds the touch selection handles (Flutter `TextSelectionControls.buildHandle`).
    /// Unset, the field shows none; Material text fields install a teardrop.
    #[must_use]
    pub fn selection_handles_builder(
        mut self,
        builder: impl for<'a> Fn(&crate::BuildContext<'a>, &crate::TextSelectionHandles) -> Widget
        + 'static,
    ) -> Self {
        self.selection_handles_builder = Some(Rc::new(builder));
        self
    }

    /// Replaces the selection handles builder; `None` removes the handles.
    #[must_use]
    pub fn selection_handles_builder_rc(
        mut self,
        builder: Option<crate::TextSelectionHandlesBuilder>,
    ) -> Self {
        self.selection_handles_builder = builder;
        self
    }

    /// Supplies the layout of a custom collapsed handle built by the
    /// selection-handles builder. Caret reveal and handle hit testing
    /// account for its actual box and minimum interactive area.
    #[must_use]
    pub fn collapsed_selection_handle_layout(
        mut self,
        layout: crate::TextSelectionHandleLayout,
    ) -> Self {
        self.collapsed_selection_handle_layout = Some(layout);
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

    /// Locale used for font shaping and spell checking, independent of
    /// keyboard language hints. Without it the editor inherits localization.
    #[must_use]
    pub fn locale(mut self, locale: incular_config::Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Flutter's `smartDashesType`. By default it is off for obscured text
    /// and on for other editors.
    #[must_use]
    pub fn smart_dashes_type(mut self, policy: crate::SmartDashesType) -> Self {
        self.smart_dashes_type = Some(policy);
        self
    }

    /// Flutter's `smartQuotesType`. By default it is off for obscured text
    /// and on for other editors.
    #[must_use]
    pub fn smart_quotes_type(mut self, policy: crate::SmartQuotesType) -> Self {
        self.smart_quotes_type = Some(policy);
        self
    }

    /// Flutter's optional `enableInlinePrediction`. Leaving it unset is
    /// different from explicitly enabling or disabling the host feature.
    #[must_use]
    pub fn enable_inline_prediction(mut self, enabled: impl Into<Option<bool>>) -> Self {
        self.enable_inline_prediction = enabled.into();
        self
    }

    /// Flutter's `hintLocales`: the requested languages for the soft keyboard.
    /// These do not change the editor's own locale. `None` leaves it up to the
    /// keyboard; an empty list explicitly requests no language hints.
    #[must_use]
    pub fn hint_locales(mut self, locales: Option<Vec<incular_config::Locale>>) -> Self {
        self.hint_locales = locales;
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
fn input_formatters_should_run(
    old: &incular_text::TextEditingValue,
    next: &incular_text::TextEditingValue,
) -> bool {
    old.text != next.text
        || (old.composing.is_some_and(|range| !range.is_empty())
            && next.composing.is_none_or(|range| range.is_empty()))
}

thread_local! {
    // Keep the default transform's identity across equivalent EditableText
    // descriptors. It is stateless and the predicate is allocated once.
    static SINGLE_LINE_EDIT_TRANSFORM: super::specs::EditTransform = {
        let formatter = crate::FilteringTextInputFormatter::single_line_formatter();
        Rc::new(move |old, next| {
            if input_formatters_should_run(old, next) && next.text.contains('\n') {
                crate::TextInputFormatter::format_edit_update(&formatter, old, next)
            } else {
                next.clone()
            }
        })
    };
}

/// Folds the default single-line filter and caller formatters in order, but
/// ignores selection-only and active-composition-range-only updates. The raw
/// `edit_transform` (if any) retains its separate semantics and runs last.
fn compose_edit_transform(
    single_line: bool,
    formatters: Vec<Rc<dyn crate::TextInputFormatter>>,
    transform: Option<super::specs::EditTransform>,
) -> Option<super::specs::EditTransform> {
    let default_filter = single_line.then(|| SINGLE_LINE_EDIT_TRANSFORM.with(Rc::clone));
    if formatters.is_empty() {
        match (&default_filter, &transform) {
            (None, _) => return transform,
            (_, None) => return default_filter,
            _ => {}
        }
    }
    Some(Rc::new(move |old, next| {
        let mut value = if input_formatters_should_run(old, next) {
            let initial = default_filter
                .as_ref()
                .map_or_else(|| next.clone(), |filter| filter(old, next));
            formatters.iter().fold(initial, |value, formatter| {
                formatter.format_edit_update(old, &value)
            })
        } else {
            next.clone()
        };
        if let Some(transform) = &transform {
            value = transform(old, &value);
        }
        value
    }))
}

impl From<EditableText> for Widget {
    fn from(value: EditableText) -> Self {
        // Numeric policy is already normalized by the setters above.
        use incular_platform::autofill::{AutofillKeyboardPlatform, infer_autofill_keyboard};
        let first_hint = value.autofill_hints.as_ref().and_then(|hints| hints.first()).map(String::as_str);
        let input_type = value.input_type.unwrap_or_else(|| {
            infer_autofill_keyboard(AutofillKeyboardPlatform::current(), first_hint, value.max_lines != Some(1))
        });
        let input_action = value.input_action;
        let spell_check_suppressed = incular_platform::autofill::is_password_input(
            value.obscure_text,
            input_type,
            value.autofill_hints.as_deref(),
        );
        let context_menu_builder = value.context_menu_builder.clone();
        let spell_toolbar_builder = value
            .spell_check
            .as_ref()
            .and_then(crate::SpellCheckConfiguration::suggestions_toolbar_builder)
            .cloned();
        let selection_handles_builder = value.selection_handles_builder.clone();
        let collapsed_selection_handle_layout = value.collapsed_selection_handle_layout;
        let legacy_selection_controls = context_menu_builder.is_none()
            && selection_handles_builder.is_none();
        let edit_transform = compose_edit_transform(
            value.max_lines == Some(1),
            value.input_formatters,
            value.edit_transform,
        );
        let can_select_with_pointer = !value.read_only || !value.obscure_text;
        // EditableText.selectionEnabled controls key actions. Its stricter
        // _userSelectionEnabled is used by native input and pointer handles.
        let keyboard_selection_enabled = value
            .enable_interactive_selection
            .unwrap_or(can_select_with_pointer);
        let content_mime_types = value.content_insertion.as_ref()
            .map(|configuration| configuration.allowed_mime_types.clone());
        let spec = TextFieldSpec {
            context_menu: None,
            has_context_menu_builder: context_menu_builder.is_some(),
            legacy_selection_controls,
            toolbar_options: value.toolbar_options,
            selection_handles: None,
            spell_check: value.spell_check,
            spell_check_suppressed,
            controller: value.controller,
            focus_node: value.focus_node,
            autofocus: value.autofocus,
            select_all_on_focus: value.select_all_on_focus,
            edit_transform,
            edit_changed: value.edit_changed,
            edit_selection_changed: value.edit_selection_changed,
            size: value.size,
            style: value.style,
            text_scaler: value.text_scaler,
            strut: value.strut,
            placeholder: value.placeholder,
            placeholder_color: value.placeholder_color,
            focused_border: value.focused_border,
            focused_border_outset: value.focused_border_outset,
            content_padding: value.content_padding,
            clip_behavior: value.clip_behavior,
            fixed_scroll_offset: value.fixed_scroll_offset,
            on_editing_complete: value.on_editing_complete,
            on_submit: value.on_submit,
            on_tap_outside: value.on_tap_outside,
            on_tap_up_outside: value.on_tap_up_outside,
            on_app_private_command: value.on_app_private_command,
            content_insertion: value.content_insertion,
            multiline: value.multiline,
            min_lines: value.min_lines,
            max_lines: value.max_lines,
            expands: value.expands,
            force_line: value.force_line,
            text_width_basis: value.text_width_basis,
            text_align: value.text_align,
            enabled: value.enabled,
            read_only: value.read_only,
            obscure_text: value.obscure_text,
            obscuring_character: value.obscuring_character,
            keyboard_selection_enabled,
            cursor_width: value.cursor_width,
            cursor_height: value.cursor_height,
            cursor_radius: value.cursor_radius,
            show_cursor: value.show_cursor.unwrap_or(!value.read_only),
            paint_cursor_above_text: value.paint_cursor_above_text,
            cursor_color: value.cursor_color,
            background_cursor_color: value.background_cursor_color,
            selection_color: value.selection_color,
            autocorrection_text_rect_color: value.autocorrection_text_rect_color,
            text_direction: value.text_direction,
            locale: value.locale,
            paragraph_layout: value.paragraph_layout,
            traits: incular_platform::TextInputTraits {
                autocorrect: value
                    .autocorrect
                    .unwrap_or_else(|| infer_autocorrect(value.autofill_hints.as_deref())),
                smart_dashes_type: value.smart_dashes_type.unwrap_or(if value.obscure_text {
                    crate::SmartDashesType::Disabled
                } else {
                    crate::SmartDashesType::Enabled
                }),
                smart_quotes_type: value.smart_quotes_type.unwrap_or(if value.obscure_text {
                    crate::SmartQuotesType::Disabled
                } else {
                    crate::SmartQuotesType::Enabled
                }),
                enable_inline_prediction: value.enable_inline_prediction,
                enable_suggestions: value.enable_suggestions,
                enable_interactive_selection: keyboard_selection_enabled && can_select_with_pointer,
                text_capitalization: value.text_capitalization,
                keyboard_appearance: value.keyboard_appearance,
                autofill_hints: value.autofill_hints,
                hint_locales: value.hint_locales,
                enable_ime_personalized_learning: value.enable_ime_personalized_learning,
                content_mime_types,
            },
            scroll_padding: value.scroll_padding,
            cursor_opacity_animates: value.cursor_opacity_animates,
        };
        let field = move |spec: TextFieldSpec| {
            Widget::from_kind(WidgetKind::TextField(Box::new(spec)))
                .with_text_input_hints(input_type, input_action)
        };
        let with_menu = move |spec: TextFieldSpec| {
            if context_menu_builder.is_none() && spell_toolbar_builder.is_none() {
                return field(spec);
            }
            crate::context_menu::with_editable_toolbars(
                move |state| {
                    let mut spec = spec.clone();
                    spec.context_menu = Some(state);
                    field(spec)
                },
                context_menu_builder.clone(),
                spell_toolbar_builder.clone(),
            )
        };
        match selection_handles_builder {
            Some(builder) => crate::text_selection_handles::with_text_selection_handles(
                move |state| {
                    let mut spec = spec.clone();
                    spec.selection_handles = Some(state);
                    with_menu(spec)
                },
                builder,
                collapsed_selection_handle_layout,
            ),
            None => with_menu(spec),
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
    pub(crate) alpha: f32,
    pub(crate) controller: Option<OpacityController>,
    pub(crate) always_include_semantics: bool,
    pub(crate) child: Widget,
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
    alignment: Option<crate::AlignmentGeometry>,
    transform_hit_tests: bool,
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
            transform_hit_tests: true,
            filter_quality: None,
            child: child.into(),
        }
    }
    /// Applies a matrix whose projection of the local XY plane is
    /// affine. Depth is not retained by this wgpu widget path. A
    /// variable perspective denominator is refused rather than flattened.
    #[must_use]
    pub fn from_matrix4(matrix: incular_core::Matrix4, child: impl Into<Widget>) -> Option<Self> {
        Some(Self::new(matrix.affine_xy_plane()?, child))
    }

    #[must_use]
    pub fn translation(offset: Offset, child: impl Into<Widget>) -> Self {
        Self::new(CoreTransform::translation(offset), child)
    }
    /// Flutter's `Transform.scale`: pivots about the center by default.
    #[must_use]
    pub fn scale(scale: f32, child: impl Into<Widget>) -> Self {
        Self::scale_non_uniform(scale, scale, child)
    }

    /// Scales X and Y independently about the child's center, corresponding
    /// to Flutter's `Transform.scale(scaleX:, scaleY:)`.
    #[must_use]
    pub fn scale_non_uniform(scale_x: f32, scale_y: f32, child: impl Into<Widget>) -> Self {
        let mut transform = Self::new(CoreTransform::scale_non_uniform(scale_x, scale_y), child);
        transform.centered = true;
        transform
    }
    /// Flutter's `Transform.rotate`: pivots about the center by default.
    /// The canonical quarter turns have exact, integral matrix entries.
    #[must_use]
    pub fn rotation(radians: f32, child: impl Into<Widget>) -> Self {
        use std::f32::consts::{FRAC_PI_2, PI};

        // Flutter builds integral quarter-turn matrices instead of leaking
        // trigonometric error into their pivot translations. Match the
        // canonical angles before promoting the rounded f32 input to f64.
        const THREE_QUARTERS: f32 = 3. * FRAC_PI_2;
        let raw = if radians == FRAC_PI_2 || radians == -THREE_QUARTERS {
            CoreTransform::from_coefficients([0., 1., -1., 0., 0., 0.])
        } else if radians == -FRAC_PI_2 || radians == THREE_QUARTERS {
            CoreTransform::from_coefficients([0., -1., 1., 0., 0., 0.])
        } else if radians == PI || radians == -PI {
            CoreTransform::scale(-1.)
        } else {
            CoreTransform::rotation(radians)
        };
        let mut transform = Self::new(raw, child);
        transform.centered = true;
        transform
    }

    /// Flutter's `Transform.flip`: mirrors the requested axes about the
    /// child's center. By default pointer hit testing follows the mirror.
    #[must_use]
    pub fn flip(flip_x: bool, flip_y: bool, child: impl Into<Widget>) -> Self {
        let scale_x = if flip_x { -1. } else { 1. };
        let scale_y = if flip_y { -1. } else { 1. };
        Self::scale_non_uniform(scale_x, scale_y, child)
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
    /// box, added to any [`Transform::origin`]. Directional alignments
    /// follow the nearest [`crate::Directionality`].
    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<crate::AlignmentGeometry>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }
    /// Whether pointer hit testing follows the painted transform. Defaults
    /// to true; false keeps the child's hit geometry at its layout position
    /// while leaving painting and visual bounds transformed.
    #[must_use]
    pub fn transform_hit_tests(mut self, enabled: bool) -> Self {
        self.transform_hit_tests = enabled;
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
        let alignment = match value.alignment {
            Some(crate::AlignmentGeometry::Physical(pivot)) => Some(pivot),
            Some(pivot) => {
                return crate::Builder::new(move |context| {
                    let direction = context
                        .text_direction()
                        .unwrap_or(incular_config::TextDirection::Ltr);
                    value.clone().alignment(pivot.resolve(direction)).into()
                })
                .into();
            }
            None => None,
        };
        match value.origin {
            Some(origin) => Widget::transform_around(value.transform, origin, value.child),
            None if value.centered => Widget::transform(value.transform, value.child),
            None => Widget::transform_around(value.transform, Offset::ZERO, value.child),
        }
        .with_transform_filter_quality(value.filter_quality)
        .with_transform_hit_geometry(alignment, value.transform_hit_tests)
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
    child: Option<Widget>,
}
impl ColorFiltered {
    #[must_use]
    pub fn new(filter: ColorFilter, child: impl Into<Widget>) -> Self {
        Self {
            filter,
            controller: None,
            child: Some(child.into()),
        }
    }
    /// Creates a filter with no child. It uses the smallest allowed size
    /// and paints nothing, even if the filter changes transparent pixels.
    #[must_use]
    pub fn without_child(filter: ColorFilter) -> Self {
        Self {
            filter,
            controller: None,
            child: None,
        }
    }
    #[must_use]
    pub fn controlled(controller: ColorFilterController, child: impl Into<Widget>) -> Self {
        Self {
            filter: controller.filter(),
            controller: Some(controller),
            child: Some(child.into()),
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
        let filter = value
            .controller
            .as_ref()
            .map_or(value.filter, ColorFilterController::filter);
        Widget::color_filtered_with_child(filter, value.controller, value.child)
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
