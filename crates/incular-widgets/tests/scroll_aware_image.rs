//! Flutter's `ScrollAwareImageProvider` and
//! `Scrollable.recommendDeferredLoadingForContext` (scroll-008): an image
//! resolved while its scroll view moves faster than the window's longest
//! physical side waits, and resolves on a later frame once the scroll slows.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use incular_animation::{Curve, scheduler};
use incular_config::{Constraints, RuntimeEnvironment};
use incular_core::{Color, Size};
use incular_image::{ImageError, ImageHandle};
use incular_scroll::ScrollController;
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::{
    AnyImageProvider, BuildContext, Builder, Column, Image, ImageConfiguration, ImageLoadState,
    ImageProvider, ImageStream, ScrollAwareContext, ScrollAwareImageProvider, Scrollable,
    SingleChildScrollView, SizedBox, Widget,
};

/// A provider that loads synchronously and counts its resolutions.
#[derive(Clone, Debug)]
struct Counted {
    id: u32,
    image: ImageHandle,
    resolutions: Rc<Cell<u32>>,
}

impl PartialEq for Counted {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Counted {}

impl ImageProvider for Counted {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        Ok(self.image.clone())
    }

    fn resolve(&self, configuration: &ImageConfiguration) -> ImageStream {
        self.resolutions.set(self.resolutions.get() + 1);
        ImageStream::from_result(self.load_with(configuration))
    }
}

fn counted(id: u32) -> Counted {
    Counted {
        id,
        image: ImageHandle::from_rgba8(4, 3, vec![255; 48]).expect("image"),
        resolutions: Rc::default(),
    }
}

/// A 200 x 100 window at scale 1: deferral starts above 200 px/s.
fn environment() -> RuntimeEnvironment {
    RuntimeEnvironment {
        viewport: Size::new(200., 100.),
        scale_factor: 1.,
        ..RuntimeEnvironment::default()
    }
}

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    controller: ScrollController,
}

fn scrolled(controller: &ScrollController, provider: &Counted) -> Widget {
    let content = Column::new([
        Widget::box_(Size::new(200., 2000.), Color::WHITE),
        Image::provider(provider.clone()).into(),
    ]);
    let view: Widget = SingleChildScrollView::new(content)
        .controller(controller.clone())
        .into();
    SizedBox::from_dimensions(Some(200.), Some(100.), Some(view)).into()
}

impl Harness {
    fn new(provider: &Counted) -> Self {
        let controller = ScrollController::new();
        let mut tree = WidgetTree::new();
        tree.set_environment(environment());
        let root = tree.mount(scrolled(&controller, provider)).expect("mount");
        let mut harness = Self {
            tree,
            root,
            controller,
        };
        harness.layout();
        harness
    }

    fn layout(&mut self) {
        self.tree
            .layout(Constraints::tight(Size::new(200., 100.)))
            .expect("layout");
    }

    fn update(&mut self, provider: &Counted) {
        self.tree
            .update(self.root, scrolled(&self.controller, provider))
            .expect("update");
        self.layout();
    }

    /// The image's current leaf element: the only leaf narrower than the
    /// window.
    fn find_image(&self) -> ElementId {
        let mut found = None;
        let mut work = vec![self.root];
        while let Some(id) = work.pop() {
            let children = self
                .tree
                .children(id)
                .map(<[_]>::to_vec)
                .unwrap_or_default();
            if children.is_empty()
                && self
                    .tree
                    .element_bounds(id)
                    .is_some_and(|b| b.size.width < 200.)
            {
                found = Some(id);
            }
            work.extend(children);
        }
        found.expect("image element")
    }

    fn image_size(&self) -> Size {
        self.tree
            .element_bounds(self.find_image())
            .expect("bounds")
            .size
    }

    /// Starts a fast `animate_to` and pumps it into its fast middle.
    fn fling(&mut self, base: Instant) {
        assert!(
            self.controller
                .animate_to(1900., Duration::from_millis(400), Curve::Linear)
        );
        self.tree.pump_scroll_flings(base, false);
        self.tree
            .pump_scroll_flings(base + Duration::from_millis(16), false);
        assert!(self.controller.activity_velocity() > 4000.);
    }

    /// Pumps the animation to its end.
    fn settle(&mut self, base: Instant) {
        let mut frame = 2;
        while self
            .tree
            .pump_scroll_flings(base + Duration::from_millis(16 * frame), false)
        {
            frame += 1;
            assert!(frame < 60, "the animation ends");
        }
        assert_eq!(self.controller.activity_velocity(), 0.);
    }
}

#[test]
fn an_image_resolved_during_a_fast_scroll_waits_for_the_scroll_to_slow() {
    let first = counted(1);
    let mut harness = Harness::new(&first);
    assert_eq!(
        first.resolutions.get(),
        1,
        "an idle scroll resolves at once"
    );
    assert_eq!(harness.image_size(), Size::new(4., 3.));

    let base = Instant::now();
    harness.fling(base);
    let second = counted(2);
    harness.update(&second);
    assert_eq!(second.resolutions.get(), 0, "deferred while moving fast");
    assert_eq!(harness.image_size(), Size::ZERO, "loading: an empty box");

    // Each frame asks again while the scroll is still fast.
    scheduler::begin_frame(base + Duration::from_millis(32));
    assert_eq!(second.resolutions.get(), 0);
    assert!(
        scheduler::has_scheduled_frame(),
        "the retry keeps frames coming"
    );

    harness.settle(base);
    scheduler::begin_frame(base + Duration::from_secs(1));
    assert_eq!(
        second.resolutions.get(),
        1,
        "resolved once the scroll stopped"
    );
    assert!(harness.tree.has_dirty_dependencies());
    harness.layout();
    assert_eq!(harness.image_size(), Size::new(4., 3.));
    scheduler::end_frame(base + Duration::from_secs(1));
}

#[test]
fn a_disposed_image_never_resolves_its_deferred_provider() {
    let first = counted(1);
    let mut harness = Harness::new(&first);
    let base = Instant::now();
    harness.fling(base);
    let second = counted(2);
    harness.update(&second);
    assert_eq!(second.resolutions.get(), 0);

    // Replacing the provider disposes the deferred resolution's context.
    let third = counted(3);
    harness.update(&third);
    harness.settle(base);
    scheduler::begin_frame(base + Duration::from_secs(1));
    scheduler::end_frame(base + Duration::from_secs(1));
    assert_eq!(
        second.resolutions.get(),
        0,
        "the replaced provider never resolves"
    );
    assert_eq!(third.resolutions.get(), 1);
}

#[test]
fn the_nearest_scroll_view_answers_for_its_descendants() {
    let answers: Rc<Cell<Option<bool>>> = Rc::default();
    let probe = {
        let answers = answers.clone();
        Builder::new(move |context: &BuildContext<'_>| {
            answers.set(Some(Scrollable::recommend_deferred_loading_for_context(
                context,
            )));
            Widget::box_(Size::new(10., 10.), Color::WHITE)
        })
    };
    let mut tree = WidgetTree::new();
    tree.set_environment(environment());
    tree.mount(probe.into()).expect("mount");
    tree.layout(Constraints::tight(Size::new(200., 100.)))
        .expect("layout");
    assert_eq!(answers.get(), Some(false), "no scroll view: never defers");

    // A detached context never defers and a disposed one never resolves.
    let provider = AnyImageProvider::new(counted(4));
    let detached = ScrollAwareImageProvider::new(ScrollAwareContext::detached(), provider.clone());
    assert!(matches!(
        detached.resolve(&ImageConfiguration::new()).state(),
        ImageLoadState::Ready(_)
    ));
    let disposed = ScrollAwareContext::detached();
    disposed.dispose();
    let stream =
        ScrollAwareImageProvider::new(disposed, provider).resolve(&ImageConfiguration::new());
    assert_eq!(stream.state(), ImageLoadState::Loading(None));
}

struct Captured {
    _tree: WidgetTree,
    context: ScrollAwareContext,
    base: Instant,
}

impl Captured {
    fn moving_to(destination: f32, duration: Duration) -> Self {
        Self::with_motion(Some((destination, duration)))
    }

    fn with_motion(motion: Option<(f32, Duration)>) -> Self {
        let controller = ScrollController::new();
        let captured = Rc::new(std::cell::RefCell::new(None));
        let probe = Builder::new({
            let captured = captured.clone();
            move |context| {
                *captured.borrow_mut() = Some(ScrollAwareContext::of(context));
                Widget::box_(Size::new(100.0, 2000.0), Color::WHITE)
            }
        });
        let mut tree = WidgetTree::new();
        tree.set_environment(environment());
        tree.mount(
            SingleChildScrollView::new(probe)
                .controller(controller.clone())
                .into(),
        )
        .unwrap();
        tree.layout(Constraints::tight(Size::new(200.0, 100.0)))
            .unwrap();
        let base = Instant::now();
        if let Some((destination, duration)) = motion {
            assert!(controller.animate_to(destination, duration, Curve::Linear));
            tree.pump_scroll_flings(base, false);
            tree.pump_scroll_flings(base + Duration::from_millis(16), false);
        }
        let context = captured.borrow_mut().take().unwrap();
        Self {
            _tree: tree,
            context,
            base,
        }
    }

    fn fast() -> Self {
        let scope = Self::moving_to(1500.0, Duration::from_millis(400));
        assert!(scope.context.recommend_deferred_loading());
        scope
    }
}

#[test]
fn slow_scrolling_does_not_delay_a_cold_provider() {
    let scroll = Captured::moving_to(50.0, Duration::from_secs(1));
    assert!(!scroll.context.recommend_deferred_loading());
    let raw = counted(5);
    let provider =
        ScrollAwareImageProvider::new(scroll.context, AnyImageProvider::new(raw.clone()));
    assert!(matches!(
        provider.resolve(&ImageConfiguration::new()).state(),
        ImageLoadState::Ready(_)
    ));
    assert_eq!(raw.resolutions.get(), 1);
}

#[test]
fn scroll_provider_equality_uses_both_context_and_provider() {
    let context = ScrollAwareContext::detached();
    let image = counted(1);
    let a = ScrollAwareImageProvider::new(context.clone(), AnyImageProvider::new(image.clone()));
    let b = ScrollAwareImageProvider::new(context.clone(), AnyImageProvider::new(counted(2)));
    let c =
        ScrollAwareImageProvider::new(ScrollAwareContext::detached(), AnyImageProvider::new(image));
    assert_eq!(a.clone(), a);
    assert_ne!(a, b);
    assert_ne!(a, c);
}

const PNG: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);

#[test]
fn warm_image_resolves_while_scrolling_fast_and_promotes_lru_even_after_disposal() {
    use incular_assets::{AssetBundle, AssetError};
    use incular_image::{ExactAssetImage, ImageCacheLimits, shared_bundle_image_cache};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone, Default)]
    struct Counts(Arc<AtomicUsize>);
    impl AssetBundle for Counts {
        fn load(&self, _: &str) -> Result<Arc<[u8]>, AssetError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Arc::from(PNG))
        }
    }
    let counts = Counts::default();
    let bundle = Arc::new(counts.clone());
    let image = |name| ExactAssetImage::new(name).bundle(bundle.clone());
    let cache = shared_bundle_image_cache();
    let initial = cache.limits();
    cache.clear();
    cache.set_limits(ImageCacheLimits::new(2, 1024));
    let configuration = ImageConfiguration::new();

    let a = image("a");
    let b = image("b");
    let hot = a.load().unwrap();
    b.load().unwrap();
    let scroll = Captured::fast();
    let provider =
        ScrollAwareImageProvider::new(scroll.context.clone(), AnyImageProvider::new(a.clone()));
    assert_eq!(
        provider.resolve(&configuration).state(),
        ImageLoadState::Ready(hot.clone())
    );
    assert_eq!(counts.0.load(Ordering::Relaxed), 2);

    scroll.context.dispose();
    let c = image("c");
    let cold =
        ScrollAwareImageProvider::new(scroll.context.clone(), AnyImageProvider::new(c.clone()));
    assert_eq!(
        cold.resolve(&configuration).state(),
        ImageLoadState::Loading(None)
    );
    assert_eq!(
        provider.resolve(&configuration).state(),
        ImageLoadState::Ready(hot)
    );
    c.load().unwrap();
    assert!(cache.contains_key(&a.obtain_key(&configuration)));
    assert!(!cache.contains_key(&b.obtain_key(&configuration)));
    assert!(cache.contains_key(&c.obtain_key(&configuration)));
    assert_eq!(counts.0.load(Ordering::Relaxed), 3);
    cache.clear();
    cache.set_limits(initial);
}

#[test]
fn deferred_image_becoming_warm_is_attached_once_on_the_next_frame() {
    use incular_assets::MemoryAssetBundle;
    use incular_image::ExactAssetImage;
    use std::sync::Arc;
    let asset = ExactAssetImage::new("warm.png")
        .bundle(Arc::new(MemoryAssetBundle::new().with("warm.png", PNG)));
    let scroll = Captured::fast();
    let provider =
        ScrollAwareImageProvider::new(scroll.context.clone(), AnyImageProvider::new(asset.clone()));
    let stream = provider.resolve(&ImageConfiguration::new());
    assert_eq!(stream.state(), ImageLoadState::Loading(None));
    let preloaded = asset.load().unwrap();
    assert_eq!(
        stream.state(),
        ImageLoadState::Loading(None),
        "preloading alone does not run the scheduled frame callback"
    );
    assert!(scroll.context.recommend_deferred_loading());
    let time = scroll.base + Duration::from_millis(32);
    scheduler::begin_frame(time);
    assert_eq!(stream.state(), ImageLoadState::Ready(preloaded));
    scheduler::end_frame(time);
}

#[test]
fn a_pending_asset_is_joined_while_scrolling_instead_of_starting_another_read() {
    use incular_assets::{AssetBundle, AssetError, AssetLoadCallback};
    use incular_image::ExactAssetImage;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Default)]
    struct AsyncSource {
        read: AtomicUsize,
        callback: Mutex<Option<AssetLoadCallback>>,
    }
    impl AssetBundle for AsyncSource {
        fn load(&self, _: &str) -> Result<Arc<[u8]>, AssetError> {
            panic!("callback-only test source")
        }
        fn load_async(&self, _: &str, done: AssetLoadCallback) {
            self.read.fetch_add(1, Ordering::Relaxed);
            assert!(self.callback.lock().unwrap().replace(done).is_none());
        }
    }
    let bundle = Arc::new(AsyncSource::default());
    let raw = ExactAssetImage::new("image.png").bundle(bundle.clone());
    let cfg = ImageConfiguration::new();
    let first = raw.resolve(&cfg);
    let scroll = Captured::fast();
    let second =
        ScrollAwareImageProvider::new(scroll.context, AnyImageProvider::new(raw)).resolve(&cfg);
    assert_eq!(first.state(), ImageLoadState::Loading(None));
    assert_eq!(second.state(), ImageLoadState::Loading(None));
    assert_eq!(bundle.read.load(Ordering::Relaxed), 1);
    let done = bundle.callback.lock().unwrap().take().unwrap();
    done(Ok(Arc::from(PNG)));
    while scheduler::run_one_scheduled_task() {}
    assert!(matches!(first.state(), ImageLoadState::Ready(_)));
    assert_eq!(first.state(), second.state());
    assert_eq!(bundle.read.load(Ordering::Relaxed), 1);
}

#[test]
fn unscrolled_idle_and_slow_contexts_start_io_before_the_first_frame() {
    use incular_assets::{AssetBundle, AssetError, AssetLoadCallback};
    use incular_image::{ExactAssetImage, shared_bundle_image_cache};
    use std::sync::{Arc, Mutex};

    struct Deferred(Mutex<Option<AssetLoadCallback>>);
    impl AssetBundle for Deferred {
        fn load(&self, _: &str) -> Result<Arc<[u8]>, AssetError> {
            panic!("unexpected blocking load")
        }
        fn load_async(&self, key: &str, done: AssetLoadCallback) {
            assert_eq!(key, "deferred.png");
            assert!(self.0.lock().unwrap().replace(done).is_none());
        }
    }
    let idle = Captured::with_motion(None);
    let slow = Captured::moving_to(50.0, Duration::from_secs(1));
    let configuration = ImageConfiguration::empty();
    let cache = shared_bundle_image_cache();
    for context in [ScrollAwareContext::detached(), idle.context, slow.context] {
        assert!(!context.recommend_deferred_loading());
        cache.clear();
        let bytes = Arc::new(Deferred(Mutex::new(None)));
        let image = ExactAssetImage::new("deferred.png").bundle(bytes.clone());
        let key = image.obtain_key(&configuration);
        assert!(!cache.contains_key(&key));
        let stream = ScrollAwareImageProvider::new(context, AnyImageProvider::new(image))
            .resolve(&configuration);
        assert_eq!(stream.state(), ImageLoadState::Loading(None));
        assert!(cache.contains_key(&key));
        assert_eq!(cache.retained(), (0, 0));
        let done = bytes
            .0
            .lock()
            .unwrap()
            .take()
            .expect("read must already have started");
        done(Ok(Arc::from(PNG)));
        while scheduler::run_one_scheduled_task() {}
        assert!(matches!(stream.state(), ImageLoadState::Ready(_)));
        assert_eq!(cache.retained(), (1, 256));
    }
}
