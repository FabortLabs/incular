use std::{cell::Cell, rc::Rc, time::Duration};

use incular_animation::AnimationController;
use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, time::Instant};
use incular_rendering::{Canvas, DisplayList, LayerId, PaintCommand};
use incular_widgets::{
    ColoredBox, CustomPaint, CustomPainter, FadeTransition, Opacity, Widget, internal::WidgetTree,
};

const RED: Color = Color::rgba(255, 0, 0, 255);
const DURATION: Duration = Duration::from_secs(1);

struct PaintCounter(Rc<Cell<u32>>);

impl CustomPainter for PaintCounter {
    fn paint(&self, size: Size) -> DisplayList {
        self.0.set(self.0.get() + 1);
        let mut canvas = Canvas::default();
        canvas.rect(Rect::from_origin_size(Offset::ZERO, size), Color::WHITE);
        canvas.finish()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

fn child(paints: &Rc<Cell<u32>>) -> Widget {
    CustomPaint::from_painter(Size::new(20., 20.), PaintCounter(Rc::clone(paints))).into()
}

fn frame(tree: &mut WidgetTree, now: Instant) -> DisplayList {
    let _ = tree.tick_animations(now);
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.update_compositor(now).unwrap();
    tree.paint()
}

fn opacity_group(list: &DisplayList) -> Option<(LayerId, f32, u64)> {
    list.commands().iter().find_map(|command| match command {
        PaintCommand::PushOpacity {
            layer,
            alpha,
            generation,
            ..
        } => Some((*layer, *alpha, *generation)),
        _ => None,
    })
}

fn shows_child(list: &DisplayList) -> bool {
    list.commands().iter().any(|op| {
        matches!(
            op,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )
    })
}

#[test]
fn fade_animates_to_one_and_nonopaque_endpoints_without_repainting() {
    for end in [1., 0.99] {
        let counts = Rc::new(Cell::new(0));
        let controller = AnimationController::with_bounds(DURATION, 0., end);
        let mut tree = WidgetTree::new();
        tree.mount(
            ColoredBox::builder()
                .color(RED)
                .child(FadeTransition::from_animation(
                    controller.clone(),
                    child(&counts),
                ))
                .build()
                .into(),
        )
        .unwrap();
        let start = Instant::now();
        let zero = frame(&mut tree, start);
        assert_eq!(counts.get(), 0);
        assert!(!shows_child(&zero));

        let _ = controller.forward(start);
        let _ = frame(&mut tree, start);
        let mid = frame(&mut tree, start + Duration::from_millis(500));
        assert_eq!(counts.get(), 1);
        assert!(shows_child(&mid));
        let (layer, mid_alpha, generation) = opacity_group(&mid).unwrap();
        assert!((mid_alpha - end / 2.).abs() < 0.001);

        let finished = frame(&mut tree, start + DURATION);
        assert_eq!(counts.get(), 1);
        assert_eq!(opacity_group(&finished), Some((layer, end, generation)));
        assert!(shows_child(&finished));
        controller.stop();
        assert_eq!(
            opacity_group(&frame(&mut tree, start + DURATION)),
            Some((layer, end, generation))
        );
        assert_eq!(counts.get(), 1);
    }
}

#[test]
fn static_opacity_reuses_the_child_and_layer_at_one() {
    let counts = Rc::new(Cell::new(0));
    let child = child(&counts);
    let scene = |alpha| {
        ColoredBox::builder()
            .color(RED)
            .child(Opacity::new(alpha, child.clone()))
            .build()
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(0.)).unwrap();
    let now = Instant::now();
    let _ = frame(&mut tree, now);
    assert_eq!(counts.get(), 0);
    tree.update(root, scene(0.1)).unwrap();
    let visible = frame(&mut tree, now);
    let (layer, _, generation) = opacity_group(&visible).unwrap();
    assert_eq!(counts.get(), 1);
    tree.update(root, scene(1.)).unwrap();
    let fully_opaque = frame(&mut tree, now);
    assert_eq!(opacity_group(&fully_opaque), Some((layer, 1., generation)));
    assert_eq!(counts.get(), 1);

    tree.update(root, scene(0.)).unwrap();
    let zero = frame(&mut tree, now);
    assert_eq!(counts.get(), 1);
    assert!(!shows_child(&zero));
    assert_eq!(
        opacity_group(&zero),
        Some((layer, 0., 0)),
        "retained empty marker is intentional"
    );
}

#[test]
fn fading_to_zero_omits_child_pixels_without_repainting() {
    let counts = Rc::new(Cell::new(0));
    let controller = AnimationController::with_bounds(DURATION, 0., 0.99);
    controller.set_value(0.99);
    let mut tree = WidgetTree::new();
    tree.mount(
        ColoredBox::builder()
            .color(RED)
            .child(FadeTransition::from_animation(
                controller.clone(),
                child(&counts),
            ))
            .build()
            .into(),
    )
    .unwrap();
    let start = Instant::now();
    let visible = frame(&mut tree, start);
    let (layer, _, _) = opacity_group(&visible).unwrap();
    assert!(shows_child(&visible));
    assert_eq!(counts.get(), 1);
    let _ = controller.reverse(start);
    let _ = frame(&mut tree, start);
    let _ = frame(&mut tree, start + Duration::from_secs(2));
    assert_eq!(counts.get(), 1);
    controller.stop();
    let zero = frame(&mut tree, start + Duration::from_secs(2));
    assert!(!shows_child(&zero));
    assert_eq!(opacity_group(&zero), Some((layer, 0., 0)));
}
