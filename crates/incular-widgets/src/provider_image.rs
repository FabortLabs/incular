//! Flutter's `Image` over an `ImageProvider` (`widgets/image.dart`
//! `_ImageState`): resolves the provider for the ambient configuration,
//! follows the resulting [`ImageStream`], and builds through the frame,
//! loading and error builders.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_core::Size;
use incular_image::{
    AnyImageProvider, ImageChunkEvent, ImageConfiguration, ImageError, ImageHandle, ImageLoadState,
    ImageProvider, ImageResizeOptions, ImageStream, ResizeImage,
};

use crate::scroll_aware_image_provider::{ScrollAwareContext, ScrollAwareImageProvider};
use crate::tree::{ImageOptions, RetainedState, image_option_setters};
use crate::{
    BuildContext, DefaultAssetBundle, ExcludeSemantics, Image, Semantics, SizedBox, Widget,
};

/// Flutter's `ImageFrameBuilder`: wraps the image once per frame. `frame` is
/// `None` until the first frame is ready and `Some(0)` after it (decoded
/// images have one frame); `was_synchronously_loaded` is whether the image
/// was ready when the provider returned it.
pub type ImageFrameBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, Widget, Option<u32>, bool) -> Widget>;

/// Flutter's `ImageLoadingBuilder`: wraps the image with its loading
/// progress, `None` once loaded.
pub type ImageLoadingBuilder =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, Widget, Option<ImageChunkEvent>) -> Widget>;

/// Flutter's `ImageErrorWidgetBuilder`: replaces the image when loading
/// failed.
pub type ImageErrorBuilder = Rc<dyn for<'a> Fn(&BuildContext<'a>, &ImageError) -> Widget>;

/// An image loaded from an [`ImageProvider`], Flutter's
/// `Image(image: provider)`. Create one with [`Image::provider`].
///
/// - The provider resolves against the ambient device pixel ratio, text
///   direction, locale and explicit size (Flutter's
///   `createLocalImageConfiguration`). It resolves again when the provider
///   or that configuration changes; an equal provider keeps its stream.
/// - While the image loads, the widget is an empty box of the explicit size.
/// - A failure shows the error builder's widget, or the empty box without
///   one.
/// - With [`Self::gapless_playback`], a new provider keeps showing the
///   previous image until the new one is ready; otherwise the box empties at
///   once.
#[derive(Clone)]
pub struct ProviderImage {
    provider: AnyImageProvider,
    options: ImageOptions,
    cache_resize: ImageResizeOptions,
    frame_builder: Option<ImageFrameBuilder>,
    loading_builder: Option<ImageLoadingBuilder>,
    error_builder: Option<ImageErrorBuilder>,
    gapless_playback: bool,
    semantic_label: Option<String>,
    exclude_from_semantics: bool,
}

impl std::fmt::Debug for ProviderImage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderImage")
            .field("provider", &self.provider)
            .field("options", &self.options)
            .field("cache_resize", &self.cache_resize)
            .field("gapless_playback", &self.gapless_playback)
            .field("semantic_label", &self.semantic_label)
            .finish_non_exhaustive()
    }
}

impl ProviderImage {
    /// An image that loads from `provider`.
    #[must_use]
    pub fn new(provider: impl ImageProvider + 'static) -> Self {
        Self::new_erased(AnyImageProvider::new(provider))
    }

    /// An image over an already erased provider with the given options.
    pub(crate) fn from_erased(provider: AnyImageProvider, options: ImageOptions) -> Self {
        Self {
            options,
            ..Self::new_erased(provider)
        }
    }

    fn new_erased(provider: AnyImageProvider) -> Self {
        Self {
            provider,
            options: ImageOptions::default(),
            cache_resize: ImageResizeOptions::default(),
            frame_builder: None,
            loading_builder: None,
            error_builder: None,
            gapless_playback: false,
            semantic_label: None,
            exclude_from_semantics: false,
        }
    }

    pub(crate) fn with_error_builder(mut self, builder: ImageErrorBuilder) -> Self {
        self.error_builder = Some(builder);
        self
    }

    image_option_setters!();

    /// Requests a decoded width independently of the layout width.
    #[must_use]
    pub fn cache_width(mut self, width: u32) -> Self {
        self.cache_resize.width = Some(width);
        self
    }

    /// Requests a decoded height independently of the layout height.
    #[must_use]
    pub fn cache_height(mut self, height: u32) -> Self {
        self.cache_resize.height = Some(height);
        self
    }

    /// Interprets cache dimensions as logical pixels, rather than the
    /// default physical pixels. Uses the active MediaQuery density.
    #[must_use]
    pub fn use_logical_cache_size(mut self, use_logical: bool) -> Self {
        self.cache_resize.use_logical_size = use_logical;
        self
    }

    /// Flutter `frameBuilder`.
    #[must_use]
    pub fn frame_builder<W: Into<Widget>>(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, Widget, Option<u32>, bool) -> W + 'static,
    ) -> Self {
        self.frame_builder = Some(Rc::new(move |context, child, frame, synchronous| {
            builder(context, child, frame, synchronous).into()
        }));
        self
    }

    /// Flutter `loadingBuilder`.
    #[must_use]
    pub fn loading_builder<W: Into<Widget>>(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, Widget, Option<ImageChunkEvent>) -> W + 'static,
    ) -> Self {
        self.loading_builder = Some(Rc::new(move |context, child, progress| {
            builder(context, child, progress).into()
        }));
        self
    }

    /// Flutter `errorBuilder`.
    #[must_use]
    pub fn error_builder<W: Into<Widget>>(
        mut self,
        builder: impl for<'a> Fn(&BuildContext<'a>, &ImageError) -> W + 'static,
    ) -> Self {
        self.error_builder = Some(Rc::new(move |context, error| {
            builder(context, error).into()
        }));
        self
    }

    /// Flutter `gaplessPlayback`: keep showing the previous image while a
    /// new provider loads. Defaults to `false`.
    #[must_use]
    pub fn gapless_playback(mut self, gapless: bool) -> Self {
        self.gapless_playback = gapless;
        self
    }

    /// Flutter `semanticLabel`: the image's description for assistive
    /// technology.
    #[must_use]
    pub fn semantic_label(mut self, label: impl Into<String>) -> Self {
        self.semantic_label = Some(label.into());
        self
    }

    /// Flutter `excludeFromSemantics`. Defaults to `false`.
    #[must_use]
    pub fn exclude_from_semantics(mut self, exclude: bool) -> Self {
        self.exclude_from_semantics = exclude;
        self
    }

    /// Flutter `createLocalImageConfiguration`: the configuration the
    /// provider resolves against.
    fn configuration(&self, context: &BuildContext<'_>) -> ImageConfiguration {
        let mut configuration = ImageConfiguration::new()
            .device_pixel_ratio(context.media_query_device_pixel_ratio().unwrap_or(1.0));
        if let Some(bundle) = DefaultAssetBundle::configured_of(context) {
            configuration = configuration.bundle(bundle);
        }
        if let (Some(width), Some(height)) = (self.options.width, self.options.height) {
            configuration = configuration.size(Size::new(width, height));
        }
        if let Some(direction) = context.text_direction() {
            configuration = configuration.text_direction(direction);
        }
        if let Some(locale) = context.locales().into_iter().next() {
            configuration = configuration.locale(locale);
        }
        configuration.normalized()
    }

    /// The image, or an empty box of the explicit size while there is none
    /// (Flutter's `RawImage` with a null image).
    fn raw(&self, image: Option<ImageHandle>) -> Widget {
        match image {
            Some(image) => {
                let mut options = self.options.clone();
                // The provider's scale maps decoded pixels to logical pixels.
                // The widget builder's scale remains an additional explicit
                // multiplier, preserving existing custom providers.
                let scale = options.scale * image.scale() as f32;
                options.scale = if scale.is_finite() && scale > 0.0 {
                    scale
                } else {
                    1.0
                };
                Image::with_options(image, options).into()
            }
            None => {
                let mut empty = SizedBox::new();
                if let Some(width) = self.options.width {
                    empty = empty.width(width);
                }
                if let Some(height) = self.options.height {
                    empty = empty.height(height);
                }
                empty.into()
            }
        }
    }
}

/// `_ImageState`.
#[derive(Default)]
struct ProviderImageState {
    /// The provider and configuration of the current stream.
    resolved: RefCell<Option<(AnyImageProvider, ImageConfiguration, ImageResizeOptions)>>,
    stream: RefCell<Option<ImageStream>>,
    /// `_imageInfo`: the image last shown, kept across providers when the
    /// playback is gapless.
    image: RefCell<Option<ImageHandle>>,
    /// `_frameNumber`.
    frame: Cell<Option<u32>>,
    /// `_wasSynchronouslyLoaded`.
    synchronous: Cell<bool>,
    /// The stream was complete when this state subscribed to it (Flutter's
    /// `synchronousCall`).
    complete_on_subscribe: Cell<bool>,
    /// `_scrollAwareContext` of the current resolution.
    scroll_context: RefCell<Option<ScrollAwareContext>>,
}

impl Drop for ProviderImageState {
    /// `_ImageState.dispose` disposes `_scrollAwareContext`, so a deferred
    /// resolution never starts.
    fn drop(&mut self) {
        if let Some(context) = self.scroll_context.get_mut() {
            context.dispose();
        }
    }
}

impl RetainedState for ProviderImageState {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl ProviderImageState {
    /// `_resolveImage` and `_updateSourceStream`: a new stream when the
    /// provider or configuration changed.
    fn stream(
        &self,
        config: &ProviderImage,
        context: &BuildContext<'_>,
        configuration: ImageConfiguration,
    ) -> ImageStream {
        let mut resolved = self.resolved.borrow_mut();
        let mut stream = self.stream.borrow_mut();
        if let (Some((provider, current, resize)), Some(stream)) =
            (resolved.as_ref(), stream.as_ref())
            && provider == &config.provider
            && *current == configuration
            && *resize == config.cache_resize
        {
            return stream.clone();
        }
        let key = (config.provider.clone(), configuration, config.cache_resize);
        // `_updateSourceStream`: without gapless playback the old image goes
        // at once.
        if !config.gapless_playback {
            *self.image.borrow_mut() = None;
        }
        self.frame.set(None);
        self.synchronous.set(false);
        // `_resolveImage` wraps the provider in a `ScrollAwareImageProvider`.
        // A new resolution disposes the previous context, so a deferred
        // resolution of a replaced provider never starts.
        let scroll_context = ScrollAwareContext::of(context);
        if let Some(previous) = self
            .scroll_context
            .borrow_mut()
            .replace(scroll_context.clone())
        {
            previous.dispose();
        }
        let provider =
            if config.cache_resize.width.is_some() || config.cache_resize.height.is_some() {
                AnyImageProvider::new(ResizeImage {
                    image_provider: config.provider.clone(),
                    options: config.cache_resize,
                })
            } else {
                config.provider.clone()
            };
        let next = ScrollAwareImageProvider::new(scroll_context, provider).resolve(&key.1);
        self.complete_on_subscribe.set(next.is_completed());
        *resolved = Some(key);
        *stream = Some(next.clone());
        next
    }

    fn build(&self, config: &ProviderImage, context: &BuildContext<'_>) -> Widget {
        let stream = self.stream(config, context, config.configuration(context));
        let (progress, error) = match stream.state() {
            ImageLoadState::Loading(progress) => (progress, None),
            ImageLoadState::Ready(image) => {
                // `_handleImageFrame`.
                let first =
                    self.frame.get().is_none() || self.image.borrow().as_ref() != Some(&image);
                if first {
                    *self.image.borrow_mut() = Some(image);
                    self.frame.set(Some(0));
                    self.synchronous.set(self.complete_on_subscribe.get());
                }
                (None, None)
            }
            ImageLoadState::Failed(error) => (None, Some(error)),
        };
        if let (Some(error), Some(builder)) = (&error, &config.error_builder) {
            return builder(context, error);
        }
        let mut result = config.raw(self.image.borrow().clone());
        if config.exclude_from_semantics {
            result = ExcludeSemantics::new(result).into();
        } else if let Some(label) = &config.semantic_label {
            result = Semantics::new(result)
                .label(label.clone())
                .image(true)
                .into();
        }
        if let Some(builder) = &config.frame_builder {
            result = builder(context, result, self.frame.get(), self.synchronous.get());
        }
        if let Some(builder) = &config.loading_builder {
            result = builder(context, result, progress);
        }
        result
    }
}

impl From<ProviderImage> for Widget {
    fn from(value: ProviderImage) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |_| ProviderImageState::default(),
            move |state, context, _| state.build(&config, context),
            false,
        )
    }
}
