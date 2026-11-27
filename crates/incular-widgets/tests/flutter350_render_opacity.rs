use incular_animation::AnimationController;
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{DisplayList, LayerId, PaintCommand};
use incular_widgets::{
    ColoredBox, FadeTransition, Opacity, RepaintBoundary, SizedBox, Widget, internal::WidgetTree,
};

fn widget(alpha: f32, color: Color) -> Widget {
    Opacity::new(
        alpha,
        RepaintBoundary::new(ColoredBox::new(color, SizedBox::square(1.))),
    )
    .into()
}

fn paint(tree: &mut WidgetTree) -> DisplayList {
    tree.layout(Constraints::tight(Size::new(10., 10.)))
        .unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    tree.paint()
}

fn layer(list: &DisplayList) -> (LayerId, f32, u64) {
    list.commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::PushOpacity {
                layer,
                alpha,
                generation,
                ..
            } => Some((*layer, *alpha, *generation)),
            _ => None,
        })
        .unwrap()
}

#[test]
fn opacity_only_has_source_draw_work_when_its_flutter_byte_is_nonzero() {
    for (alpha, byte) in [
        (0., 0_u8),
        (0.49 / 255., 0),
        (0.51 / 255., 1),
        (0.1, 26),
        (0.5, 128),
        (0.7, 179),
        (0.9, 230),
        (0.99, 252),
        (1., 255),
    ] {
        let mut tree = WidgetTree::new();
        let parent = tree
            .mount(Opacity::new(alpha, ColoredBox::new(Color::WHITE, SizedBox::square(1.))).into())
            .unwrap();
        let list = paint(&mut tree);
        let (_, reported, generation) = layer(&list);
        assert_eq!(reported, f32::from(byte) / 255.);
        let child = tree.children(parent).unwrap()[0];
        assert_eq!(
            tree.paints_child(
                tree.render_id(parent).unwrap(),
                tree.render_id(child).unwrap()
            ),
            byte != 0
        );
        assert_eq!(
            list.commands().iter().any(|cmd| matches!(
                cmd,
                PaintCommand::Rect {
                    color: Color::WHITE,
                    ..
                }
            )),
            byte != 0
        );
        if byte == 0 {
            assert_eq!(generation, 0);
        }
    }
}

#[test]
fn changing_child_paint_reuses_the_same_half_opacity_layer() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget(0.5, Color::WHITE)).unwrap();
    let first = layer(&paint(&mut tree));
    tree.update(root, widget(0.5, Color::BLACK)).unwrap();
    let changed = paint(&mut tree);
    let second = layer(&changed);
    assert_eq!((first.0, first.1), (second.0, second.1));
    assert_ne!(first.2, second.2);
    assert!(changed.commands().iter().any(|cmd| matches!(
        cmd,
        PaintCommand::Rect {
            color: Color::BLACK,
            ..
        }
    )));
}

#[test]
fn animated_opacity_composites_only_when_the_layer_has_a_visible_source() {
    for (value, byte) in [(0., 0_u8), (1., 255), (0.5, 128)] {
        let controller = AnimationController::new(std::time::Duration::from_secs(1));
        controller.set_value(value);
        let fade = FadeTransition::from_animation(
            controller,
            ColoredBox::new(Color::WHITE, SizedBox::square(1.)),
        );
        let mut tree = WidgetTree::new();
        tree.mount(fade.into()).unwrap();
        let list = paint(&mut tree);
        let (_, alpha, generation) = layer(&list);
        assert_eq!(alpha, f32::from(byte) / 255.);
        let drawn = list.commands().iter().any(|op| {
            matches!(
                op,
                PaintCommand::Rect {
                    color: Color::WHITE,
                    ..
                }
            )
        });
        assert_eq!(drawn, byte != 0);
        if byte == 0 {
            assert_eq!(generation, 0);
        }
    }
}

#[test]
fn tiny_animated_opacity_reports_child_eligibility_without_raster_work() {
    let controller = AnimationController::new(std::time::Duration::from_secs(1));
    controller.set_value(0.49 / 255.);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            FadeTransition::from_animation(
                controller,
                ColoredBox::new(Color::WHITE, SizedBox::square(1.)),
            )
            .into(),
        )
        .unwrap();
    let list = paint(&mut tree);
    assert_eq!(layer(&list).1, 0.);
    assert!(!list.commands().iter().any(|op| matches!(
        op,
        PaintCommand::Rect {
            color: Color::WHITE,
            ..
        }
    )));
    let child = tree.children(root).unwrap()[0];
    assert!(tree.paints_child(
        tree.render_id(root).unwrap(),
        tree.render_id(child).unwrap()
    ));
}
