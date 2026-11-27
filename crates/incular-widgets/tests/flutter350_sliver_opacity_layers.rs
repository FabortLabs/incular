use std::time::Duration;

use incular_animation::AnimationController;
use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size, time::Instant};
use incular_rendering::{DisplayList, LayerId, PaintCommand};
use incular_widgets::{
    CustomScrollView, Directionality, Sliver, SliverFadeTransition, SliverOpacity,
    SliverToBoxAdapter, Widget, internal::WidgetTree,
};

fn child(color: Color) -> SliverToBoxAdapter {
    SliverToBoxAdapter::new(Widget::box_(Size::new(1., 1.), color))
}

fn scene(sliver: impl Sliver + 'static) -> Widget {
    Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>]),
    )
    .into()
}

fn paint(tree: &mut WidgetTree, at: Instant) -> DisplayList {
    tree.layout(Constraints::tight(Size::new(10., 10.)))
        .unwrap();
    tree.update_compositor(at).unwrap();
    tree.paint()
}

fn opacity(display: &DisplayList) -> (LayerId, f32, u64) {
    let records: Vec<_> = display
        .commands()
        .iter()
        .filter_map(|op| match op {
            PaintCommand::PushOpacity {
                layer,
                alpha,
                generation,
                ..
            } => Some((*layer, *alpha, *generation)),
            _ => None,
        })
        .collect();
    assert_eq!(records.len(), 1);
    records[0]
}

fn amount(widget: Widget, byte: u8) {
    let mut tree = WidgetTree::new();
    tree.mount(widget).unwrap();
    let display = paint(&mut tree, Instant::now());
    let (_, alpha, generation) = opacity(&display);
    assert_eq!(alpha, f32::from(byte) / 255.);
    assert_eq!(
        display.commands().iter().any(|op| matches!(
            op,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )),
        byte > 0
    );
    assert_eq!(generation != 0, byte > 0);
}

#[test]
fn sliver_opacity_zero_has_no_source_draw_and_full_opacity_does() {
    amount(scene(SliverOpacity::new(0., child(Color::WHITE))), 0);
    amount(scene(SliverOpacity::new(1., child(Color::WHITE))), 255);
}

#[test]
fn animated_sliver_opacity_zero_partial_and_full_match_compositing() {
    for (value, byte) in [(0., 0_u8), (0.5, 128), (1., 255)] {
        let animation = AnimationController::new(Duration::from_secs(1));
        animation.set_value(value);
        amount(
            scene(SliverFadeTransition::from_animation(
                animation,
                child(Color::WHITE),
            )),
            byte,
        );
    }
}

fn repaint_keeps_layer(build: impl Fn(Color) -> Widget) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(build(Color::WHITE)).unwrap();
    let at = Instant::now();
    let first = opacity(&paint(&mut tree, at));
    tree.update(root, build(Color::BLACK)).unwrap();
    let changed = paint(&mut tree, at);
    let second = opacity(&changed);
    assert_eq!(first.0, second.0);
    assert_eq!(second.1, 128. / 255.);
    assert_ne!(first.2, second.2);
    assert!(changed.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::BLACK,
            ..
        }
    )));
}

#[test]
fn static_sliver_opacity_layer_survives_a_dirty_child_repaint() {
    repaint_keeps_layer(|color| scene(SliverOpacity::new(0.5, child(color))));
}

#[test]
fn animated_sliver_opacity_layer_survives_a_dirty_child_repaint() {
    let animation = AnimationController::new(Duration::from_secs(1));
    animation.set_value(0.5);
    repaint_keeps_layer(|color| {
        scene(SliverFadeTransition::from_animation(
            animation.clone(),
            child(color),
        ))
    });
}
