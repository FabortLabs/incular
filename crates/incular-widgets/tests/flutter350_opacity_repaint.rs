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
        let (middle, final_alpha) = if end == 1. {
            (128. / 255., 1.)
        } else {
            (126. / 255., 252. / 255.)
        };
        assert_eq!(mid_alpha, middle);

        let finished = frame(&mut tree, start + DURATION);
        assert_eq!(counts.get(), 1);
        assert_eq!(
            opacity_group(&finished),
            Some((layer, final_alpha, generation))
        );
        assert!(shows_child(&finished));
        controller.stop();
        assert_eq!(
            opacity_group(&frame(&mut tree, start + DURATION)),
            Some((layer, final_alpha, generation))
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

#[test]
fn a_two_second_fade_updates_compositing_without_reentering_child_build() {
    use incular_widgets::{Builder, Placeholder};

    let builds = Rc::new(Cell::new(0));
    let controller = AnimationController::new(Duration::from_secs(2));
    let child_key = incular_widgets::internal::Key::from("fade build counter");
    let builder = Widget::from(Builder::new({
        let builds = builds.clone();
        move |_| {
            builds.set(builds.get() + 1);
            Placeholder::new().into()
        }
    }))
    .with_key(child_key.clone());
    let mut tree = WidgetTree::new();
    tree.mount(FadeTransition::from_animation(controller.clone(), builder))
        .unwrap();
    let start = Instant::now();
    frame(&mut tree, start);
    assert_eq!(builds.get(), 1);
    let child = tree.element_with_key(&child_key).unwrap();
    let render = tree.render_id(child);
    frame(&mut tree, start);
    assert_eq!(builds.get(), 1);

    let _ = controller.forward(start);
    for (milliseconds, expected) in [
        (0, 0.),
        (500, 64. / 255.),
        (1_000, 128. / 255.),
        (2_000, 1.),
    ] {
        let list = frame(&mut tree, start + Duration::from_millis(milliseconds));
        assert_eq!(
            opacity_group(&list).map(|(_, alpha, _)| alpha),
            Some(expected)
        );
        assert_eq!(builds.get(), 1);
        assert_eq!(tree.element_with_key(&child_key), Some(child));
        assert_eq!(tree.render_id(child), render);
    }
}

#[test]
fn requesting_paint_between_alpha_changes_neither_throws_nor_exposes_hidden_pixels() {
    let paints = Rc::new(Cell::new(0));
    let controller = AnimationController::new(Duration::from_secs(2)).with_value(1.);
    let leaf_key = incular_widgets::internal::Key::from("fade invalidated leaf");
    let mut tree = WidgetTree::new();
    tree.mount(FadeTransition::from_animation(
        controller.clone(),
        child(&paints).with_key(leaf_key.clone()),
    ))
    .unwrap();
    let start = Instant::now();
    let initial = frame(&mut tree, start);
    assert!(shows_child(&initial));
    assert_eq!(paints.get(), 1);
    let leaf = tree.element_with_key(&leaf_key).unwrap();
    let render = tree.render_id(leaf);

    controller.set_value(0.5);
    tree.mark_paint(leaf).unwrap();
    controller.set_value(0.);
    let hidden = frame(&mut tree, start + Duration::from_millis(16));
    assert!(!shows_child(&hidden));
    assert_eq!(paints.get(), 1);

    controller.set_value(1.);
    let visible = frame(&mut tree, start + Duration::from_millis(32));
    assert!(shows_child(&visible));
    assert_eq!(paints.get(), 2, "paint dirtiness survives the hidden frame");
    assert_eq!(tree.element_with_key(&leaf_key), Some(leaf));
    assert_eq!(tree.render_id(leaf), render);
}

#[test]
fn a_visible_fade_under_tight_zero_constraints_finishes_at_zero_area() {
    use incular_config::TextDirection;
    use incular_widgets::{Center, Directionality, Placeholder, internal::Key};

    let key = Key::from("zero-area fade");
    let controller = AnimationController::new(Duration::from_secs(2)).with_value(1.);
    let fade = FadeTransition::from_animation(controller, Placeholder::new()).with_key(key.clone());
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Center::new(fade)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    tree.try_paint().unwrap();
    let element = tree.element_with_key(&key).unwrap();
    assert_eq!(tree.element_layout_size(element), Some(Size::ZERO));
}
