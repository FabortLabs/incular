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
