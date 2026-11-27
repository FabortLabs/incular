//! Flutter's `Image` over an `ImageProvider` (`widgets/image.dart`
//! `_ImageState`): resolves the provider for the ambient configuration,
//! follows the resulting [`ImageStream`], and builds through the frame,
//! loading and error builders.

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, hash_map::RandomState},
    hash::BuildHasher,
    rc::{Rc, Weak},
    sync::Arc,
};

use incular_core::Size;
use incular_image::{
    AnyImageProvider, ImageChunkEvent, ImageConfiguration, ImageError, ImageHandle, ImageInfo,
    ImageLoadState, ImagePlayback, ImagePlaybackKeepAlive, ImagePlaybackSubscription,
    ImageProvider, ImageResizeOptions, ImageStream, ResizeImage,
};

use crate::scroll_aware_image_provider::{ScrollAwareContext, ScrollAwareImageProvider};
use crate::tree::{ImageOptions, RetainedState, image_option_setters};
use crate::{
    BuildContext, DefaultAssetBundle, ExcludeSemantics, Image, Semantics, SizedBox, Widget,
};

/// Flutter's `ImageFrameBuilder`: wraps the image once per frame. `frame` is
/// `None` until the first frame is ready, then counts delivered frames
/// from zero. `was_synchronously_loaded` records whether the stream
/// provided any of those frames synchronously.
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
    source: ImageWidgetSource,
    options: ImageOptions,
    cache_resize: ImageResizeOptions,
    frame_builder: Option<ImageFrameBuilder>,
    loading_builder: Option<ImageLoadingBuilder>,
    error_builder: Option<ImageErrorBuilder>,
    gapless_playback: bool,
    semantic_label: Option<String>,
    exclude_from_semantics: bool,
}

#[derive(Clone)]
enum ImageWidgetSource {
    Provider(AnyImageProvider),
    Playback(ImagePlayback),
    AnimatedMemory {
        bytes: Arc<[u8]>,
        fallback: AnyImageProvider,
    },
}

impl std::fmt::Debug for ImageWidgetSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(provider) => provider.fmt(formatter),
            Self::Playback(playback) => playback.fmt(formatter),
            Self::AnimatedMemory { bytes, .. } => formatter
                .debug_struct("AnimatedMemory")
                .field("encoded_bytes", &bytes.len())
                .finish(),
        }
    }
}

impl std::fmt::Debug for ProviderImage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderImage")
            .field("source", &self.source)
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

    /// An image driven by a retained playback session. Prefer
    /// [`ImagePlayback::scheduled`] for the normal app frame clock.
    /// The attached codec controls decoded size; cache resize options
    /// apply to providers only.
    #[must_use]
    pub fn from_playback(playback: ImagePlayback) -> Self {
        Self::with_source(ImageWidgetSource::Playback(playback))
    }

    fn new_erased(provider: AnyImageProvider) -> Self {
        let source = match provider.animated_memory_webp() {
            Some(bytes) => ImageWidgetSource::AnimatedMemory {
                bytes,
                fallback: provider,
            },
            None => ImageWidgetSource::Provider(provider),
        };
        Self::with_source(source)
    }

    fn with_source(source: ImageWidgetSource) -> Self {
        Self {
            source,
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

    fn configuration(&self, context: &BuildContext<'_>) -> ImageConfiguration {
        let size = self
            .options
            .width
            .zip(self.options.height)
            .map(|(width, height)| Size::new(width, height));
        local_image_configuration(context, size)
    }

    /// The image, or an empty box of the explicit size while there is none
    /// (Flutter's `RawImage` with a null image).
    fn raw(&self, image: Option<ImageHandle>, scale_override: Option<f64>) -> Widget {
        match image {
            Some(image) => {
                let mut options = self.options.clone();
                // The provider's scale maps decoded pixels to logical pixels.
                // The widget builder's scale remains an additional explicit
                // multiplier, preserving existing custom providers.
                let scale = options.scale * scale_override.unwrap_or_else(|| image.scale()) as f32;
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
    fn decorate(
        &self,
        context: &BuildContext<'_>,
        mut result: Widget,
        frame: Option<u32>,
        synchronous: bool,
        progress: Option<ImageChunkEvent>,
    ) -> Widget {
        if self.exclude_from_semantics {
            result = ExcludeSemantics::new(result).into();
        } else if let Some(label) = &self.semantic_label {
            result = Semantics::new(result)
                .label(label.clone())
                .image(true)
                .into();
        }
        if let Some(builder) = &self.frame_builder {
            result = builder(context, result, frame, synchronous);
        }
        if let Some(builder) = &self.loading_builder {
            result = builder(context, result, progress);
        }
        result
    }
}

/// Flutter `createLocalImageConfiguration`, shared by Image and DecorationImage.
pub(crate) fn local_image_configuration(
    context: &BuildContext<'_>,
    size: Option<Size>,
) -> ImageConfiguration {
    let mut configuration = ImageConfiguration::new()
        .device_pixel_ratio(context.media_query_device_pixel_ratio().unwrap_or(1.0));
    if let Some(bundle) = DefaultAssetBundle::configured_of(context) {
        configuration = configuration.bundle(bundle);
    }
    if let Some(size) = size {
        configuration = configuration.size(size);
    }
    if let Some(direction) = context.text_direction() {
        configuration = configuration.text_direction(direction);
    }
    if let Some(locale) = context.locales().into_iter().next() {
        configuration = configuration.locale(locale);
    }
    configuration.normalized()
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
        requested_provider: &AnyImageProvider,
        context: &BuildContext<'_>,
        configuration: ImageConfiguration,
    ) -> ImageStream {
        let mut resolved = self.resolved.borrow_mut();
        let mut stream = self.stream.borrow_mut();
        if let (Some((provider, current, resize)), Some(stream)) =
            (resolved.as_ref(), stream.as_ref())
            && provider == requested_provider
            && *current == configuration
            && *resize == config.cache_resize
        {
            return stream.clone();
        }
        let key = (
            requested_provider.clone(),
            configuration,
            config.cache_resize,
        );
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
                    image_provider: requested_provider.clone(),
                    options: config.cache_resize,
                })
            } else {
                requested_provider.clone()
            };
        let next = ScrollAwareImageProvider::new(scroll_context, provider).resolve(&key.1);
        self.complete_on_subscribe.set(next.is_completed());
        *resolved = Some(key);
        *stream = Some(next.clone());
        next
    }

    fn build(
        &self,
        config: &ProviderImage,
        provider: &AnyImageProvider,
        context: &BuildContext<'_>,
    ) -> Widget {
        let stream = self.stream(config, provider, context, config.configuration(context));
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
        config.decorate(
            context,
            config.raw(self.image.borrow().clone(), None),
            self.frame.get(),
            self.synchronous.get(),
            progress,
        )
    }
}

#[derive(Default)]
struct PlaybackFrame {
    image: Option<ImageInfo>,
    frame: Option<u32>,
    synchronous: bool,
    error: Option<ImageError>,
}

struct PlaybackImageState {
    source: RefCell<Option<ImagePlayback>>,
    subscription: RefCell<Option<ImagePlaybackSubscription>>,
    keep_alive: RefCell<Option<ImagePlaybackKeepAlive>>,
    displayed: RefCell<PlaybackFrame>,
    paused: Cell<bool>,
    binding: Cell<bool>,
    generation: Cell<u64>,
    revision: RefCell<Rc<Cell<u64>>>,
}

impl RetainedState for PlaybackImageState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        *self.revision.borrow_mut() = revision;
    }
}

impl PlaybackImageState {
    fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            source: RefCell::new(None),
            subscription: RefCell::new(None),
            keep_alive: RefCell::new(None),
            displayed: RefCell::new(PlaybackFrame::default()),
            paused: Cell::new(false),
            binding: Cell::new(false),
            generation: Cell::new(0),
            revision: RefCell::new(revision),
        }
    }

    fn unlisten(&self) {
        let previous = self.subscription.borrow_mut().take();
        drop(previous);
    }

    fn on_frame(&self, result: Result<ImageInfo, ImageError>, synchronous: bool) {
        let displayed_frame = {
            let mut displayed = self.displayed.borrow_mut();
            match result {
                Ok(image) => {
                    displayed.image = Some(image);
                    displayed.frame = Some(displayed.frame.map_or(0, |n| n.saturating_add(1)));
                    displayed.synchronous |= synchronous;
                    displayed.error = None;
                    true
                }
                Err(error) => {
                    displayed.error = Some(error);
                    false
                }
            }
        };
        if self.paused.get() && displayed_frame {
            // Stop before the completer asks the codec for a second frame.
            self.unlisten();
        }
        if !self.binding.get() {
            let revision = self.revision.borrow();
            revision.set(revision.get().wrapping_add(1));
        }
    }

    fn build(
        self: &Rc<Self>,
        config: &ProviderImage,
        source: &ImagePlayback,
        context: &BuildContext<'_>,
        provider_density: f64,
    ) -> Widget {
        let replacing = self.source.borrow().as_ref() != Some(source);
        if replacing {
            self.generation.set(self.generation.get().wrapping_add(1));
            self.unlisten();
            let previous = self.keep_alive.replace(source.keep_alive());
            drop(previous);
            self.source.replace(Some(source.clone()));
            let mut displayed = self.displayed.borrow_mut();
            let previous = if config.gapless_playback {
                displayed.image.take()
            } else {
                None
            };
            *displayed = PlaybackFrame {
                image: previous,
                ..PlaybackFrame::default()
            };
        }
        self.paused.set(
            !context.ticker_mode_enabled()
                || context
                    .media_query_disable_animations()
                    .unwrap_or_else(|| context.disable_animations()),
        );
        if self.paused.get() && self.displayed.borrow().frame.is_some() {
            self.unlisten();
        } else if self.subscription.borrow().is_none() {
            self.binding.set(true);
            let widget = Rc::downgrade(self);
            let generation = self.generation.get();
            let subscription = source.listen(move |result, synchronous| {
                if let Some(widget) = widget.upgrade()
                    && widget.generation.get() == generation
                {
                    widget.on_frame(result, synchronous);
                }
            });
            self.binding.set(false);
            if !self.paused.get() || self.displayed.borrow().frame.is_none() {
                *self.subscription.borrow_mut() = subscription;
            }
        }

        let (image, frame, synchronous, error) = {
            let displayed = self.displayed.borrow();
            (
                displayed.image.clone(),
                displayed.frame,
                displayed.synchronous,
                displayed.error.clone(),
            )
        };
        if let (Some(error), Some(builder)) = (&error, &config.error_builder) {
            return builder(context, error);
        }
        let scale = image.as_ref().map(|info| info.scale() * provider_density);
        let raw = config.raw(image.map(ImageInfo::into_image), scale);
        config.decorate(context, raw, frame, synchronous, None)
    }
}

type LoadEncodedImage = dyn Fn(Arc<[u8]>, ImageResizeOptions) -> ImagePlayback;

struct ResolvedMemoryPlayback {
    bytes: Arc<[u8]>,
    options: ImageResizeOptions,
    source: ImagePlayback,
}

impl ResolvedMemoryPlayback {
    fn matches(&self, bytes: &Arc<[u8]>, options: ImageResizeOptions) -> bool {
        self.options == options && (Arc::ptr_eq(&self.bytes, bytes) || &self.bytes == bytes)
    }
}

#[derive(Default)]
struct ActiveImages {
    entries: HashMap<(u64, ImageResizeOptions), Vec<Weak<ResolvedMemoryPlayback>>>,
    reads: u8,
}

/// A window-scoped worker capability and index of currently active image
/// resolutions. The index does not own encoded bytes, any pixel buffer,
/// or the decoder; those belong to mounted resolutions.
#[doc(hidden)]
pub struct ImagePlaybackLoader {
    load: Box<LoadEncodedImage>,
    hasher: RandomState,
    active: RefCell<ActiveImages>,
}

impl ImagePlaybackLoader {
    #[must_use]
    pub fn new(load: impl Fn(Arc<[u8]>, ImageResizeOptions) -> ImagePlayback + 'static) -> Self {
        Self {
            load: Box::new(load),
            hasher: RandomState::new(),
            active: RefCell::new(ActiveImages::default()),
        }
    }

    fn resolve(
        &self,
        bytes: &Arc<[u8]>,
        options: ImageResizeOptions,
    ) -> Rc<ResolvedMemoryPlayback> {
        let key = (self.hasher.hash_one(bytes.as_ref()), options);
        {
            let mut active = self.active.borrow_mut();
            active.reads = active.reads.wrapping_add(1);
            if active.reads % 32 == 0 {
                active.entries.retain(|_, bucket| {
                    bucket.retain(|entry| entry.strong_count() != 0);
                    !bucket.is_empty()
                });
            }
            if let Some(bucket) = active.entries.get_mut(&key) {
                bucket.retain(|entry| entry.strong_count() != 0);
                if let Some(found) = bucket
                    .iter()
                    .filter_map(Weak::upgrade)
                    .find(|candidate| candidate.matches(bytes, options))
                {
                    return found;
                }
            }
        }
        let resolution = Rc::new(ResolvedMemoryPlayback {
            bytes: bytes.clone(),
            options,
            source: (self.load)(bytes.clone(), options),
        });
        self.active
            .borrow_mut()
            .entries
            .entry(key)
            .or_default()
            .push(Rc::downgrade(&resolution));
        resolution
    }
}

struct AnimatedMemoryState {
    resolution: RefCell<Option<Rc<ResolvedMemoryPlayback>>>,
    playback: Rc<PlaybackImageState>,
    fallback: ProviderImageState,
}

impl RetainedState for AnimatedMemoryState {
    fn rebind_revision(&self, revision: Rc<Cell<u64>>) {
        self.playback.rebind_revision(revision);
    }
}

impl AnimatedMemoryState {
    fn new(revision: Rc<Cell<u64>>) -> Self {
        Self {
            resolution: RefCell::new(None),
            playback: Rc::new(PlaybackImageState::new(revision)),
            fallback: ProviderImageState::default(),
        }
    }

    fn build(
        &self,
        config: &ProviderImage,
        bytes: &Arc<[u8]>,
        fallback: &AnyImageProvider,
        context: &BuildContext<'_>,
    ) -> Widget {
        let Some(loader) = context.find_shared::<ImagePlaybackLoader>() else {
            return self.fallback.build(config, fallback, context);
        };
        let options = if config.cache_resize.use_logical_size {
            config.cache_resize.physical(&config.configuration(context))
        } else {
            config.cache_resize
        };
        let previous = self
            .resolution
            .borrow()
            .as_ref()
            .filter(|resolution| resolution.matches(bytes, options))
            .cloned();
        let resolution = previous.unwrap_or_else(|| {
            let resolution = loader.resolve(bytes, options);
            self.resolution.replace(Some(resolution.clone()));
            resolution
        });
        let density = fallback
            .downcast_ref::<incular_image::MemoryImage>()
            .map_or(1., incular_image::MemoryImage::image_scale);
        self.playback
            .build(config, &resolution.source, context, density)
    }
}

impl From<ProviderImage> for Widget {
    fn from(value: ProviderImage) -> Self {
        let config = Rc::new(value);
        match &config.source {
            ImageWidgetSource::Provider(provider) => {
                let provider = provider.clone();
                Widget::retained_builder(
                    |_| ProviderImageState::default(),
                    move |state, context, _| state.build(&config, &provider, context),
                    false,
                )
            }
            ImageWidgetSource::AnimatedMemory { bytes, fallback } => {
                let bytes = bytes.clone();
                let fallback = fallback.clone();
                Widget::mount_scoped_builder(AnimatedMemoryState::new, move |state, context| {
                    state.build(&config, &bytes, &fallback, context)
                })
            }
            ImageWidgetSource::Playback(playback) => {
                let playback = playback.clone();
                Widget::mount_scoped_builder(PlaybackImageState::new, move |state, context| {
                    state.build(&config, &playback, context, 1.)
                })
            }
        }
    }
}
