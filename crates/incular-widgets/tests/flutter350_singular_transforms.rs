//! Invalid and singular affine transforms must not paint their child.
use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform, time::Instant};
use incular_rendering::{Canvas, DisplayList, PaintCommand};
use incular_widgets::{
    ClipRect, CustomPaint, CustomPainter, RepaintBoundary, Transform, Widget,
    internal::{Key, WidgetTree},
};

struct Counter(Rc<Cell<u32>>);

impl CustomPainter for Counter {
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

fn picture(count: Rc<Cell<u32>>) -> Widget {
    RepaintBoundary::new(
        Widget::from(CustomPaint::from_painter(
            Size::new(50., 50.),
            Counter(count),
        ))
        .with_key("transform paint target"),
    )
    .into()
}

fn frame(tree: &mut WidgetTree) -> DisplayList {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    tree.try_paint().unwrap()
}

fn shows_picture(scene: &DisplayList) -> bool {
    scene.commands().iter().any(|command| {
        matches!(
            command,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )
    })
}

#[test]
fn nan_and_both_infinities_short_circuit_child_paint_and_culling() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let count = Rc::new(Cell::new(0));
        let child = picture(count.clone());
        let transform = CoreTransform::from_coefficients([invalid, 0., 0., 1., 0., 0.]);
        // The ancestor clip invokes paint bounds culling before the invalid
        // child can publish a transform or picture to the renderer.
        let mut tree = WidgetTree::new();
        tree.mount(ClipRect::new(Transform::new(transform, child)).into())
            .unwrap();
        let display = frame(&mut tree);
        assert!(!shows_picture(&display), "invalid coefficient: {invalid}");
        assert_eq!(count.get(), 0, "invalid coefficient: {invalid}");
        assert!(display.commands().iter().all(|command| match command {
            PaintCommand::PushTransform { transform } =>
                transform.coefficients().iter().all(|v| v.is_finite()),
            _ => true,
        }));
    }
}

#[test]
fn a_zero_scale_skips_child_paint_but_small_and_quarter_turn_matrices_restore_it() {
    let count = Rc::new(Cell::new(0));
    let child = picture(count.clone());
    let scale = |x, y| Widget::from(Transform::scale_non_uniform(x, y, child.clone()));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scale(0., 0.)).unwrap();
    for axes in [(0., 0.), (0., 1.), (1., 0.)] {
        tree.update(root, scale(axes.0, axes.1)).unwrap();
        let hidden = frame(&mut tree);
        assert!(!shows_picture(&hidden));
        assert_eq!(count.get(), 0);
    }

    tree.update(root, scale(0.01, 0.01)).unwrap();
    let visible = frame(&mut tree);
    assert!(shows_picture(&visible));
    assert_eq!(count.get(), 1);
    let leaf = tree
        .element_with_key(&Key::from("transform paint target"))
        .unwrap();
    let render = tree.render_id(leaf);

    tree.update(root, scale(0., 0.)).unwrap();
    tree.mark_paint(leaf).unwrap();
    assert!(!shows_picture(&frame(&mut tree)));
    assert_eq!(count.get(), 1);
    tree.update(
        root,
        Transform::rotation(std::f32::consts::FRAC_PI_2, child).into(),
    )
    .unwrap();
    let rotated = frame(&mut tree);
    assert!(shows_picture(&rotated));
    assert_eq!(count.get(), 2);
    assert_eq!(tree.render_id(leaf), render);
    assert!(rotated.commands().iter().any(|command| matches!(
        command,
        PaintCommand::PushTransform { transform } if transform.coefficients()[..4] == [0., 1., -1., 0.]
    )));
}

#[test]
fn a_live_scale_keeps_the_leaf_dirty_while_zero_and_repaints_once_when_restored() {
    use incular_widgets::{ScaleController, ScaleTransition};

    let controller = ScaleController::new();
    let count = Rc::new(Cell::new(0));
    let mut tree = WidgetTree::new();
    tree.mount(ScaleTransition::new(controller.clone(), picture(count.clone())).into())
        .unwrap();
    assert!(shows_picture(&frame(&mut tree)));
    assert_eq!(count.get(), 1);
    let leaf = tree
        .element_with_key(&Key::from("transform paint target"))
        .unwrap();
    let render = tree.render_id(leaf);
    assert!(controller.set_scale(0.));
    tree.mark_paint(leaf).unwrap();
    assert!(!shows_picture(&frame(&mut tree)));
    assert_eq!(count.get(), 1);
    assert!(controller.set_scale(1.));
    assert!(shows_picture(&frame(&mut tree)));
    assert_eq!(count.get(), 2);
    assert_eq!(tree.render_id(leaf), render);
}

#[test]
fn a_non_finite_transform_cannot_crash_semantic_geometry_and_recovers() {
    use incular_widgets::{ClipRRect, Semantics, SizedBox};

    let mut tree = WidgetTree::new();
    let semantic_child = || {
        ClipRRect::new(
            4.,
            Semantics::new(SizedBox::expand())
                .container(true)
                .text_direction(incular_config::TextDirection::Ltr)
                .label("transformed label"),
        )
    };
    let scene = |matrix| Widget::from(Transform::new(matrix, semantic_child()));
    let root = tree.mount(scene(CoreTransform::IDENTITY)).unwrap();

    for affine in [
        CoreTransform::IDENTITY,
        CoreTransform::from_coefficients([f64::NAN, 0., 0., 1., 0., 0.]),
        CoreTransform::IDENTITY,
        CoreTransform::from_coefficients([f64::INFINITY, 0., 0., 1., 0., 0.]),
        CoreTransform::IDENTITY,
    ] {
        tree.update(root, scene(affine)).unwrap();
        tree.layout(Constraints::tight(Size::new(100., 80.)))
            .unwrap();
        tree.update_compositor(Instant::now()).unwrap();
        tree.update_semantics();
        tree.try_paint().unwrap();
        if affine.has_drawable_area() {
            let node = tree
                .semantics()
                .iter()
                .find_map(|(_, node)| {
                    (node.label.as_deref() == Some("transformed label")).then_some(node)
                })
                .expect("a drawable child must publish semantics again");
            assert_eq!(node.bounds.size, Size::new(100., 80.));
        } else {
            assert!(tree.semantics().iter().all(|(_, node)| {
                node.label.as_deref() != Some("transformed label") || node.bounds.size == Size::ZERO
            }));
        }
    }
}
