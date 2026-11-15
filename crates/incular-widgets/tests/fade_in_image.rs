//! Flutter's `FadeInImage`: a synchronously available target shows at once;
//! one that arrives later replaces the placeholder with a fade-out, fade-in
//! sequence (300 ms ease-out, then 700 ms ease-in).

use std::time::{Duration, Instant};

use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::Size;
use incular_image::ImageHandle;
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::{FadeInImage, Widget};

fn image(shade: u8) -> ImageHandle {
    ImageHandle::from_rgba8(2, 2, vec![shade; 16]).expect("image")
}

struct Harness {
    tree: WidgetTree,
    root: ElementId,
    base: Instant,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self {
            tree,
            root,
            base: Instant::now(),
        };
        harness.frame(0);
        harness
    }

    fn frame(&mut self, millis: u64) {
        self.tree
            .update_compositor(self.base + Duration::from_millis(millis))
            .expect("frame");
        self.tree
            .layout(Constraints::loose(Size::new(100., 100.)))
            .expect("layout");
    }

    /// (images, opacities) in paint order.
    fn scan(&self) -> (usize, Vec<f32>) {
        let (mut images, mut opacities) = (0, Vec::new());
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            match self
                .tree
                .render_id(id)
                .and_then(|render| self.tree.render_object_kind(render))
            {
                Some(RenderKind::Opacity { alpha, .. }) => opacities.push(*alpha),
                Some(RenderKind::Image { .. }) => images += 1,
                _ => {}
            }
            let mut children = self.tree.children(id).unwrap_or_default().to_vec();
            children.reverse();
            stack.extend(children);
        }
        (images, opacities)
    }
}

#[test]
fn a_target_present_at_first_shows_without_fading() {
    let harness = Harness::new(FadeInImage::new(image(0), Some(image(255))).into());
    assert_eq!(harness.scan(), (1, vec![]));
}

#[test]
fn a_target_that_arrives_later_fades_in() {
    let mut harness = Harness::new(FadeInImage::new(image(0), None).into());
    // Only the placeholder, fully opaque.
    assert_eq!(harness.scan(), (1, vec![1.0]));
    harness
        .tree
        .update(
            harness.root,
            FadeInImage::new(image(0), Some(image(255))).into(),
        )
        .expect("update");
    // The rebuild restarts the animation, which starts on the next frame.
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    // 150 ms is half the fade-out: the target (below) is still hidden.
    let (images, opacities) = harness.scan();
    assert_eq!(images, 2);
    assert_eq!(opacities[0], 0.0);
    assert!((opacities[1] - (1. - Curve::EaseOut.apply(0.5))).abs() < 1e-4);
    harness.frame(1_650);
    // 650 ms is half the fade-in: the placeholder is gone from view.
    let (_, opacities) = harness.scan();
    assert!((opacities[0] - Curve::EaseIn.apply(0.5)).abs() < 1e-4);
    assert_eq!(opacities[1], 0.0);
    harness.frame(2_000);
    harness.frame(2_016);
    // Done: only the target remains.
    assert_eq!(harness.scan(), (1, vec![]));
}

/// A provider whose stream the test completes (paint-014).
#[derive(Clone, Debug)]
struct Deferred {
    id: u32,
    stream: incular_image::ImageStream,
}

impl PartialEq for Deferred {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Deferred {}

impl incular_image::ImageProvider for Deferred {
    fn load(&self) -> Result<ImageHandle, incular_image::ImageError> {
        Err(incular_image::ImageError::Io("resolves a stream".into()))
    }

    fn resolve(&self, _: &incular_image::ImageConfiguration) -> incular_image::ImageStream {
        self.stream.clone()
    }
}

fn deferred(id: u32) -> (Deferred, incular_image::ImageStreamCompleter) {
    let (stream, completer) = incular_image::ImageStream::loading();
    (Deferred { id, stream }, completer)
}

fn ready(id: u32, shade: u8) -> Deferred {
    let (provider, completer) = deferred(id);
    completer.complete(image(shade));
    provider
}

/// Flutter `FadeInImage(placeholder:, image:)`: the placeholder shows while
/// the target provider loads, and the target's first frame starts the fade.
#[test]
fn a_provider_target_fades_in_once_its_first_frame_is_ready() {
    let (target, completer) = deferred(2);
    let mut harness = Harness::new(FadeInImage::from_providers(ready(1, 0), target).into());
    // Flutter's frame builder wraps the empty target too: it sits, hidden,
    // above the opaque placeholder.
    assert_eq!(harness.scan(), (1, vec![0.0, 1.0]));
    completer.complete(image(255));
    harness.frame(1_000);
    harness.frame(1_000);
    harness.frame(1_150);
    let (images, opacities) = harness.scan();
    assert_eq!(images, 2);
    assert_eq!(opacities[0], 0.0);
    assert!((opacities[1] - (1. - Curve::EaseOut.apply(0.5))).abs() < 1e-4);
    harness.frame(2_000);
    harness.frame(2_016);
    assert_eq!(harness.scan(), (1, vec![]));
}

#[test]
fn a_synchronously_ready_provider_target_shows_at_once() {
    let harness = Harness::new(FadeInImage::from_providers(ready(1, 0), ready(2, 255)).into());
    assert_eq!(harness.scan(), (1, vec![]));
}

#[test]
fn the_image_error_builder_replaces_the_fade_when_the_target_fails() {
    let (target, completer) = deferred(2);
    let image = FadeInImage::from_providers(ready(1, 0), target)
        .image_error_builder(|_, _| Widget::box_(Size::new(7., 7.), incular_core::Color::WHITE));
    let mut harness = Harness::new(image.into());
    completer.fail(incular_image::ImageError::InvalidDimensions);
    harness.frame(16);
    assert_eq!(harness.scan(), (0, vec![]));
    assert_eq!(
        harness
            .tree
            .element_bounds(harness.root)
            .expect("bounds")
            .size,
        Size::new(7., 7.)
    );
}

#[test]
fn without_an_image_error_builder_a_failed_target_keeps_the_placeholder() {
    let (target, completer) = deferred(2);
    let mut harness = Harness::new(FadeInImage::from_providers(ready(1, 0), target).into());
    completer.fail(incular_image::ImageError::InvalidDimensions);
    harness.frame(16);
    assert_eq!(harness.scan(), (1, vec![0.0, 1.0]));
}

#[test]
fn the_placeholder_error_builder_replaces_a_failed_placeholder() {
    let (placeholder, completer) = deferred(1);
    let (target, _target_done) = deferred(2);
    let image =
        FadeInImage::from_providers(placeholder, target).placeholder_error_builder(|_, _| {
            Widget::box_(Size::new(5., 3.), incular_core::Color::WHITE)
        });
    let mut harness = Harness::new(image.into());
    completer.fail(incular_image::ImageError::InvalidDimensions);
    harness.frame(16);
    assert_eq!(
        harness
            .tree
            .element_bounds(harness.root)
            .expect("bounds")
            .size,
        Size::new(5., 3.)
    );
}
