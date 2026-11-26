//! Flutter's `FadeInImage` (`widgets/fade_in_image.dart`): a placeholder
//! image that fades out as the target image, once loaded, fades in.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::{AnimationController, Curve};
use incular_config::Alignment;
use incular_core::Color;
use incular_image::{AnyImageProvider, ImageError, ImageHandle, ImageProvider};
use incular_rendering::{BlendMode, FilterQuality};

use crate::provider_image::ImageErrorBuilder;
use crate::tree::{ImageFit, ImageOptions, ImageRepeat, RetainedState};
use crate::{BuildContext, Image, Opacity, ProviderImage, Semantics, Stack, StackFit, Widget};

/// Where a [`FadeInImage`] image comes from.
#[derive(Clone)]
enum Source {
    /// A decoded image the application supplies (`None`: not loaded yet).
    Handle(Option<ImageHandle>),
    /// A provider the widget resolves, as Flutter's `FadeInImage` does.
    Provider(AnyImageProvider),
}

/// The options one of the two images paints with.
#[derive(Clone, Default)]
struct Look {
    fit: Option<ImageFit>,
    filter_quality: Option<FilterQuality>,
    color: Option<Color>,
    color_blend_mode: Option<BlendMode>,
    error_builder: Option<ImageErrorBuilder>,
}

/// Flutter's `FadeInImage`: a placeholder shows while the target image
/// loads, then fades out as the target fades in.
///
/// [`Self::from_providers`] follows Flutter: both images come from
/// providers, the target's first frame starts the fade, and a target that
/// was ready synchronously shows at once. With [`Self::new`] the application
/// decodes the images itself: a target present from the first build shows
/// at once, and one supplied later fades in.
#[derive(Clone)]
pub struct FadeInImage {
    placeholder: Source,
    image: Source,
    fade_out_duration: Duration,
    fade_out_curve: Curve,
    fade_in_duration: Duration,
    fade_in_curve: Curve,
    width: Option<f32>,
    height: Option<f32>,
    alignment: Alignment,
    repeat: ImageRepeat,
    match_text_direction: bool,
    image_look: Look,
    placeholder_look: Look,
    image_semantic_label: Option<String>,
    exclude_from_semantics: bool,
}

impl FadeInImage {
    /// A fade from a decoded placeholder to a decoded target, which the
    /// application supplies once its own loading completes.
    #[must_use]
    pub fn new(placeholder: ImageHandle, image: Option<ImageHandle>) -> Self {
        Self::with_sources(Source::Handle(Some(placeholder)), Source::Handle(image))
    }

    /// Flutter's `FadeInImage(placeholder:, image:)`: both images resolve
    /// from providers; the target fades in once its first frame is ready.
    #[must_use]
    pub fn from_providers(
        placeholder: impl ImageProvider + 'static,
        image: impl ImageProvider + 'static,
    ) -> Self {
        Self::with_sources(
            Source::Provider(AnyImageProvider::new(placeholder)),
            Source::Provider(AnyImageProvider::new(image)),
        )
    }

    fn with_sources(placeholder: Source, image: Source) -> Self {
        Self {
            placeholder,
            image,
            fade_out_duration: Duration::from_millis(300),
            fade_out_curve: Curve::EaseOut,
            fade_in_duration: Duration::from_millis(700),
            fade_in_curve: Curve::EaseIn,
            width: None,
            height: None,
            alignment: Alignment::CENTER,
            repeat: ImageRepeat::NoRepeat,
            match_text_direction: false,
            image_look: Look::default(),
            placeholder_look: Look::default(),
            image_semantic_label: None,
            exclude_from_semantics: false,
        }
    }

    /// Defaults to 300 ms.
    #[must_use]
    pub fn fade_out_duration(mut self, duration: Duration) -> Self {
        self.fade_out_duration = duration;
        self
    }

    /// Defaults to ease-out.
    #[must_use]
    pub fn fade_out_curve(mut self, curve: Curve) -> Self {
        self.fade_out_curve = curve;
        self
    }

    /// Defaults to 700 ms.
    #[must_use]
    pub fn fade_in_duration(mut self, duration: Duration) -> Self {
        self.fade_in_duration = duration;
        self
    }

    /// Defaults to ease-in.
    #[must_use]
    pub fn fade_in_curve(mut self, curve: Curve) -> Self {
        self.fade_in_curve = curve;
        self
    }

    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    #[must_use]
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.image_look.fit = Some(fit);
        self
    }

    /// Defaults to `fit`.
    #[must_use]
    pub fn placeholder_fit(mut self, fit: ImageFit) -> Self {
        self.placeholder_look.fit = Some(fit);
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Flutter `repeat`, applied to both the target and the placeholder.
    #[must_use]
    pub fn repeat(mut self, repeat: ImageRepeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Flutter `matchTextDirection`, applied to both images.
    #[must_use]
    pub fn match_text_direction(mut self, matches: bool) -> Self {
        self.match_text_direction = matches;
        self
    }

    /// Flutter `imageFilterQuality`; `None` keeps the default sampling.
    #[must_use]
    pub fn image_filter_quality(mut self, quality: FilterQuality) -> Self {
        self.image_look.filter_quality = Some(quality);
        self
    }

    /// Flutter `placeholderFilterQuality`; `None` keeps the default sampling.
    #[must_use]
    pub fn placeholder_filter_quality(mut self, quality: FilterQuality) -> Self {
        self.placeholder_look.filter_quality = Some(quality);
        self
    }

    /// Flutter `color`: a tint over the target image.
    #[must_use]
    pub fn image_color(mut self, color: Color) -> Self {
        self.image_look.color = Some(color);
        self
    }

    /// Flutter `colorBlendMode`, for the target image's tint.
    #[must_use]
    pub fn image_color_blend_mode(mut self, mode: BlendMode) -> Self {
        self.image_look.color_blend_mode = Some(mode);
        self
    }

    /// Flutter `placeholderColor`: a tint over the placeholder.
    #[must_use]
    pub fn placeholder_color(mut self, color: Color) -> Self {
        self.placeholder_look.color = Some(color);
        self
    }

    /// Flutter `placeholderColorBlendMode`, for the placeholder's tint.
    #[must_use]
    pub fn placeholder_color_blend_mode(mut self, mode: BlendMode) -> Self {
        self.placeholder_look.color_blend_mode = Some(mode);
        self
    }

    /// Flutter `imageErrorBuilder`: replaces the whole fade when the target
    /// provider fails. Without one, the placeholder stays.
    #[must_use]
    pub fn image_error_builder<W: Into<Widget>>(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, &ImageError) -> W + 'static,
    ) -> Self {
        self.image_look.error_builder = Some(Rc::new(move |context, error| {
            builder(context, error).into()
        }));
        self
    }

    /// Flutter `placeholderErrorBuilder`: replaces the placeholder when its
    /// provider fails.
    #[must_use]
    pub fn placeholder_error_builder<W: Into<Widget>>(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, &ImageError) -> W + 'static,
    ) -> Self {
        self.placeholder_look.error_builder = Some(Rc::new(move |context, error| {
            builder(context, error).into()
        }));
        self
    }

    #[must_use]
    pub fn image_semantic_label(mut self, label: impl Into<String>) -> Self {
        self.image_semantic_label = Some(label.into());
        self
    }

    #[must_use]
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.exclude_from_semantics = exclude;
        self
    }

    /// The image options for one of the images. The placeholder's fit
    /// defaults to the target's.
    fn options(&self, look: &Look, fit: Option<ImageFit>) -> ImageOptions {
        ImageOptions {
            width: self.width,
            height: self.height,
            fit,
            repeat: self.repeat,
            alignment: self.alignment.into(),
            sampling: look
                .filter_quality
                .map_or(ImageOptions::default().sampling, Into::into),
            match_text_direction: self.match_text_direction,
            color: look.color,
            color_blend_mode: look.color_blend_mode,
            ..ImageOptions::default()
        }
    }

    fn placeholder_options(&self) -> ImageOptions {
        self.options(
            &self.placeholder_look,
            self.placeholder_look.fit.or(self.image_look.fit),
        )
    }

    fn image_options(&self) -> ImageOptions {
        self.options(&self.image_look, self.image_look.fit)
    }

    /// Flutter `_image(placeholder, placeholderErrorBuilder, ..)`.
    fn placeholder(&self) -> Widget {
        match &self.placeholder {
            Source::Handle(handle) => match handle {
                Some(handle) => {
                    Image::with_options(handle.clone(), self.placeholder_options()).into()
                }
                None => crate::SizedBox::new().into(),
            },
            Source::Provider(provider) => {
                let mut image =
                    ProviderImage::from_erased(provider.clone(), self.placeholder_options());
                if let Some(builder) = &self.placeholder_look.error_builder {
                    image = image.with_error_builder(builder.clone());
                }
                image.into()
            }
        }
    }
}

/// `_AnimatedFadeOutFadeIn`: the placeholder fades out over the fade-out
/// share of the timeline once the target has loaded, then the target fades
/// in over the rest.
struct FadeState {
    config: RefCell<Option<Rc<FadeInImage>>>,
    controller: AnimationController,
    /// `wasSynchronouslyLoaded`: the target was there on the first build.
    synchronous: Cell<bool>,
    /// The target opacity `_AnimatedFadeOutFadeIn` animates toward.
    loaded: Cell<bool>,
    listening: Cell<bool>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for FadeState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl FadeState {
    /// `synchronous` is the frame builder's `wasSynchronouslyLoaded`; `None`
    /// takes it from whether the target was loaded on the first build.
    fn update(&self, config: &Rc<FadeInImage>, loaded: bool, synchronous: Option<bool>) {
        let first = self.config.replace(Some(config.clone())).is_none();
        self.controller
            .set_duration(config.fade_out_duration + config.fade_in_duration);
        if first {
            self.synchronous.set(synchronous.unwrap_or(loaded));
            self.loaded.set(loaded);
            return;
        }
        if synchronous == Some(true) {
            self.synchronous.set(true);
        }
        // `ImplicitlyAnimatedWidget.didUpdateWidget`: a new target restarts
        // the animation from zero.
        if loaded != self.loaded.replace(loaded) && !self.synchronous.get() {
            self.controller.set_value(0.0);
            self.controller.forward_on_next_tick();
        }
    }

    /// The `TweenSequence`s of `_AnimatedFadeOutFadeInState.didUpdateTweens`.
    fn opacities(&self, config: &FadeInImage) -> (f32, f32) {
        if !self.loaded.get() {
            return (1., 0.);
        }
        let out = config.fade_out_duration.as_secs_f32();
        let total = out + config.fade_in_duration.as_secs_f32();
        let split = if total > 0. { out / total } else { 0. };
        let t = self.controller.value();
        let placeholder = if t < split {
            1. - config.fade_out_curve.apply(t / split)
        } else {
            0.
        };
        let target = if t <= split {
            0.
        } else {
            config.fade_in_curve.apply((t - split) / (1. - split))
        };
        (placeholder, target)
    }

    fn build(&self, placeholder: Widget, target: Option<Widget>) -> Widget {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the image updates before it builds");
        let result: Widget =
            if self.synchronous.get() || (self.loaded.get() && self.controller.is_completed()) {
                target.unwrap_or(placeholder)
            } else {
                let (placeholder_opacity, target_opacity) = self.opacities(&config);
                let mut children: Vec<Widget> = Vec::new();
                if let Some(target) = target {
                    children.push(Opacity::new(target_opacity, target).into());
                }
                children.push(Opacity::new(placeholder_opacity, placeholder).into());
                Stack::new(children)
                    .fit(StackFit::Passthrough)
                    .alignment(Alignment::CENTER)
                    .into()
            };
        Widget::animation_ticker_with_revision(
            self.controller.clone(),
            false,
            self.revision.borrow().clone(),
            result,
        )
    }
}

/// The fade between `placeholder` and `target` (`_AnimatedFadeOutFadeIn`).
fn fade(
    config: Rc<FadeInImage>,
    target: Option<Widget>,
    loaded: bool,
    synchronous: Option<bool>,
) -> Widget {
    let placeholder = config.placeholder();
    Widget::retained_builder(
        |revision| FadeState {
            config: RefCell::new(None),
            controller: AnimationController::new(Duration::ZERO),
            synchronous: Cell::new(false),
            loaded: Cell::new(false),
            listening: Cell::new(false),
            revision: RefCell::new(revision),
        },
        move |state, _, _| {
            if !state.listening.replace(true) {
                let weak = Rc::downgrade(state);
                state.controller.add_listener(move |_| {
                    if let Some(state) = weak.upgrade() {
                        let revision = state.revision.borrow().clone();
                        revision.set(revision.get().wrapping_add(1));
                    }
                });
            }
            state.update(&config, loaded, synchronous);
            state.build(placeholder.clone(), target.clone())
        },
        false,
    )
}

impl From<FadeInImage> for Widget {
    fn from(value: FadeInImage) -> Self {
        let config = Rc::new(value);
        let result = match &config.image {
            Source::Handle(handle) => {
                let target = handle
                    .clone()
                    .map(|handle| Image::with_options(handle, config.image_options()).into());
                fade(config.clone(), target, handle.is_some(), None)
            }
            // Flutter builds the fade from the target `Image`'s frame
            // builder: the first frame (or a synchronous load) marks the
            // target loaded.
            Source::Provider(provider) => {
                let mut image =
                    ProviderImage::from_erased(provider.clone(), config.image_options());
                if let Some(builder) = &config.image_look.error_builder {
                    image = image.with_error_builder(builder.clone());
                }
                let fade_config = config.clone();
                image
                    .frame_builder(move |_, child, frame, synchronous| {
                        fade(
                            fade_config.clone(),
                            Some(child),
                            frame.is_some() || synchronous,
                            Some(synchronous),
                        )
                    })
                    .into()
            }
        };
        if config.exclude_from_semantics {
            return result;
        }
        let mut semantics = Semantics::new(result).image(true);
        if let Some(label) = &config.image_semantic_label {
            semantics = semantics.label(label.clone());
        }
        semantics.into()
    }
}
