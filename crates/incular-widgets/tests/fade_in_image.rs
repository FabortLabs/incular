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
