//! Flutter's `ScrollAwareImageProvider`
//! (`widgets/scroll_aware_image_provider.dart`) and
//! `Scrollable.recommendDeferredLoadingForContext`
//! (`widgets/scrollable.dart`): image resolution waits while the nearest
//! scroll view moves too fast for the image to matter.

use std::{cell::Cell, fmt, rc::Rc};

use incular_image::{
    AnyImageProvider, ImageConfiguration, ImageError, ImageHandle, ImageProvider, ImageStream,
};
use incular_scroll::ScrollController;

use crate::{BuildContext, Scrollable};

/// Flutter's `_ScrollableScope`: every scroll view provides the controller
/// of its position to its descendants.
#[derive(Clone)]
pub(crate) struct ScrollableScope {
    controller: ScrollController,
}

impl ScrollableScope {
    pub(crate) fn new(controller: ScrollController) -> Self {
        Self { controller }
    }

    pub(crate) fn controller(&self) -> &ScrollController {
        &self.controller
    }
}

/// The longest side of the window in physical pixels, Flutter's
/// `View.of(context).physicalSize.longestSide`.
fn physical_longest_side(context: &BuildContext<'_>) -> f32 {
    let size = context.viewport();
    #[allow(clippy::cast_possible_truncation)]
    let scale = context.scale_factor() as f32;
    size.width.max(size.height) * scale
}

impl Scrollable {
    /// Flutter `Scrollable.recommendDeferredLoadingForContext`: whether the
    /// scroll view nearest to `context` moves fast enough that expensive
    /// work, such as resolving images, should wait. The answer comes from
    /// the scroll position's [`ScrollController::recommend_deferred_loading`]
    /// and is not tracked: a build that asks is not rebuilt when it changes.
    /// Without an enclosing scroll view it is `false`.
    ///
    /// Flutter's optional `axis` argument, which skips scroll views of the
    /// other axis, is not ported.
    #[must_use]
    pub fn recommend_deferred_loading_for_context(context: &BuildContext<'_>) -> bool {
        context.find::<ScrollableScope>().is_some_and(|scope| {
            scope
                .controller
                .recommend_deferred_loading(physical_longest_side(context))
        })
    }
}

/// What a [`ScrollAwareImageProvider`] keeps of the build context it was
/// made for, Flutter's `DisposableBuildContext`: the nearest scroll
/// position, the size of the window, and whether the widget that owns the
/// context has been disposed. Clones share the disposal.
#[derive(Clone)]
pub struct ScrollAwareContext {
    scrollable: Option<ScrollController>,
    physical_longest_side: f32,
    disposed: Rc<Cell<bool>>,
}

impl ScrollAwareContext {
    /// Captures `context`. The owner calls [`Self::dispose`] when it is
    /// disposed, as Flutter's `_ImageState.dispose` does.
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> Self {
        Self {
            scrollable: context
                .find::<ScrollableScope>()
                .map(|scope| scope.controller),
            physical_longest_side: physical_longest_side(context),
            disposed: Rc::new(Cell::new(false)),
        }
    }

    /// A context outside any scroll view, which never defers.
    #[must_use]
    pub fn detached() -> Self {
        Self {
            scrollable: None,
            physical_longest_side: 0.,
            disposed: Rc::new(Cell::new(false)),
        }
    }

    /// Flutter `DisposableBuildContext.dispose`: providers made for this
    /// context stop resolving.
    pub fn dispose(&self) {
        self.disposed.set(true);
    }

    /// Whether [`Self::dispose`] was called on this context or a clone.
    #[must_use]
    pub fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    /// [`Scrollable::recommend_deferred_loading_for_context`] for the
    /// captured context.
    #[must_use]
    pub fn recommend_deferred_loading(&self) -> bool {
        self.scrollable.as_ref().is_some_and(|controller| {
            controller.recommend_deferred_loading(self.physical_longest_side)
        })
    }
}

impl PartialEq for ScrollAwareContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.disposed, &other.disposed)
    }
}

impl Eq for ScrollAwareContext {}

impl fmt::Debug for ScrollAwareContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScrollAwareContext")
            .field("in_scrollable", &self.scrollable.is_some())
            .field("disposed", &self.is_disposed())
            .finish()
    }
}

/// Flutter's `ScrollAwareImageProvider`: resolves `image_provider`, but
/// while the scroll view around its context is moving fast it returns a
/// loading stream and tries again on each later frame, resolving once the
/// scroll slows down. A disposed context never resolves.
///
/// [`ProviderImage`](crate::ProviderImage), and so
/// [`FadeInImage`](crate::FadeInImage), wraps every provider it resolves in
/// one, as Flutter's `Image` does. Incular has no provider-keyed image
/// cache, so Flutter's shortcut for cached keys does not apply; an image
/// widget that keeps an equal provider keeps its stream and does not
/// resolve again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrollAwareImageProvider {
    context: ScrollAwareContext,
    image_provider: AnyImageProvider,
}

impl ScrollAwareImageProvider {
    /// Wraps `image_provider` for `context`.
    #[must_use]
    pub fn new(context: ScrollAwareContext, image_provider: AnyImageProvider) -> Self {
        Self {
            context,
            image_provider,
        }
    }

    /// The context the provider defers for.
    #[must_use]
    pub fn context(&self) -> &ScrollAwareContext {
        &self.context
    }

    /// The wrapped provider.
    #[must_use]
    pub fn image_provider(&self) -> &AnyImageProvider {
        &self.image_provider
    }

    /// `SchedulerBinding.scheduleFrameCallback` and the microtask after it:
    /// asks again on the next frame and hands `completer`'s stream to the
    /// real resolution once loading is no longer deferred.
    fn retry_next_frame(
        self,
        configuration: ImageConfiguration,
        completer: incular_image::ImageStreamCompleter,
    ) {
        incular_animation::scheduler::schedule_frame_callback(move |_| {
            if self.context.is_disposed() {
                return;
            }
            if self.context.recommend_deferred_loading() {
                self.retry_next_frame(configuration, completer);
                return;
            }
            completer.follow(&self.image_provider.resolve(&configuration));
        });
    }
}

impl ImageProvider for ScrollAwareImageProvider {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        self.load_with(&ImageConfiguration::new())
    }

    fn load_with(&self, configuration: &ImageConfiguration) -> Result<ImageHandle, ImageError> {
        self.image_provider.load_with(configuration)
    }

    /// Flutter `resolveStreamForKey`.
    fn resolve(&self, configuration: &ImageConfiguration) -> ImageStream {
        if self.context.is_disposed() {
            // Flutter returns without a completer: the stream never loads.
            return ImageStream::loading().0;
        }
        if !self.context.recommend_deferred_loading() {
            return self.image_provider.resolve(configuration);
        }
        let (stream, completer) = ImageStream::loading();
        self.clone()
            .retry_next_frame(configuration.clone(), completer);
        stream
    }
}
