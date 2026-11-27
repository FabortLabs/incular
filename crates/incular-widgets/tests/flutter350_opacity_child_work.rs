//! Compare the first real child painter call, separate from pixel visibility.
use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, DisplayList};
use incular_widgets::{
    Center, CustomPaint, CustomPainter, Opacity, Text, TextDirection, internal::WidgetTree,
};
use std::{cell::RefCell, rc::Rc};

struct CountingPainter(Rc<RefCell<Vec<Size>>>);

impl CustomPainter for CountingPainter {
    fn paint(&self, size: Size) -> DisplayList {
        self.0.borrow_mut().push(size);
        let mut canvas = Canvas::default();
        canvas.rect(Rect::from_origin_size(Offset::ZERO, size), Color::WHITE);
        canvas.finish()
    }

    fn should_repaint(&self, _: &Self) -> bool {
        true
    }
}

fn first_frame(alpha: f32, keep_semantics: bool) -> Vec<Size> {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let child = CustomPaint::from_painter(Size::new(40., 20.), CountingPainter(calls.clone()));
    let mut tree = WidgetTree::new();
    tree.mount(
        Center::new(Opacity::new(alpha, child).always_include_semantics(keep_semantics)).into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
    calls.take()
}

#[test]
fn custom_painter_initial_call_obeys_effective_alpha() {
    let size = Size::new(40., 20.);
    let data = [
        (1., false, vec![size]),
        (0.1, false, vec![size]),
        (0., false, vec![]),
        (0., true, vec![]),
        (0.001, false, vec![]),
        (0.003, false, vec![size]),
    ];
    let expected: Vec<_> = data.iter().map(|(_, _, calls)| calls.clone()).collect();
    let actual: Vec<_> = data
        .iter()
        .map(|(alpha, keep, _)| first_frame(*alpha, *keep))
        .collect();
    assert_eq!(
        actual, expected,
        "initial painter call sizes across visible, hidden, and quantized opacities"
    );
}

#[test]
fn retained_opacity_toggles_child_label_at_the_original_thresholds() {
    let build = |alpha, keep| {
        Opacity::new(alpha, Text::new("a").text_direction(TextDirection::Rtl))
            .always_include_semantics(keep)
            .into()
    };
    let cases = [
        (1., false, true),
        (0., false, false),
        (0., true, true),
        (0., false, false),
        (0.1, false, true),
        (0.1, false, true),
        (0.1, true, true),
    ];
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(1., false)).unwrap();
    for (alpha, keep, visible) in cases {
        tree.update(root, build(alpha, keep)).unwrap();
        tree.layout(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let _ = tree.paint();
        tree.update_semantics();
        let label = tree
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("a"));
        assert_eq!(label, visible, "alpha={alpha} keep={keep}");
    }
}

fn child_label_count(tree: &mut WidgetTree) -> usize {
    tree.update_semantics();
    tree.semantics()
        .iter()
        .filter(|(_, node)| node.label.as_deref() == Some("opacity child"))
        .count()
}

#[test]
fn render_opacity_visits_the_child_only_while_effectively_visible() {
    let build = |alpha| {
        Opacity::new(
            alpha,
            Text::new("opacity child").text_direction(TextDirection::Ltr),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(1.)).unwrap();
    let bounds = Constraints::tight(Size::new(100., 60.));
    for (alpha, expected) in [
        (1., 1),
        (0.5, 1),
        (0.25, 1),
        (0.125, 1),
        (0., 0),
        (0.125, 1),
        (0., 0),
        (0.001, 0),
        (0.003, 1),
    ] {
        tree.update(root, build(alpha)).unwrap();
        tree.layout(bounds).unwrap();
        assert_eq!(child_label_count(&mut tree), expected, "alpha={alpha}");
    }
}

#[test]
fn mounted_animation_opacity_semantics_track_crossings_on_the_same_child() {
    use incular_animation::AnimationController;
    use incular_widgets::FadeTransition;
    use std::time::{Duration, Instant};

    let controller = AnimationController::new(Duration::from_millis(100));
    let fade = FadeTransition::from_animation(
        controller.clone(),
        Text::new("opacity child").text_direction(TextDirection::Ltr),
    );
    let mut tree = WidgetTree::new();
    assert_eq!(controller.value(), 0.);
    controller.set_value(0.2);
    assert_eq!(child_label_count(&mut tree), 0);
    controller.set_value(1.);
    assert_eq!(child_label_count(&mut tree), 0);
    tree.mount(fade).unwrap();
    let start = Instant::now();
    let bounds = Constraints::tight(Size::new(100., 60.));
    for (index, (alpha, expected)) in [
        (1., 1),
        (0.5, 1),
        (0.25, 1),
        (0.125, 1),
        (0., 0),
        (0.125, 1),
        (0., 0),
    ]
    .into_iter()
    .enumerate()
    {
        controller.set_value(alpha);
        let at = start + Duration::from_millis(index as u64);
        tree.tick_animations(at);
        tree.layout(bounds).unwrap();
        tree.update_compositor(at).unwrap();
        assert_eq!(child_label_count(&mut tree), expected, "alpha={alpha}");
    }
}
