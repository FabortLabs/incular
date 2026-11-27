use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{BlendMode, Brush, PaintCommand};
use incular_widgets::{ShaderCallback, ShaderMask, Widget, internal::WidgetTree};

#[test]
fn shader_callback_and_blend_mode_updates_are_visible_without_eager_shader_work() {
    let calls = Rc::new([Cell::new(0), Cell::new(0)]);
    let red = Color::rgba(255, 0, 0, 255);
    let blue = Color::rgba(0, 0, 255, 255);
    let shader = |index: usize, color| {
        let calls = calls.clone();
        ShaderCallback::new(move |_| {
            calls[index].set(calls[index].get() + 1);
            Brush::Solid(color)
        })
    };
    let original = shader(0, red);
    let replacement = shader(1, blue);
    let mask = ShaderMask::new(
        original.clone(),
        Widget::box_(Size::new(15., 20.), Color::WHITE),
    );
    assert_eq!(mask.shader(), &original);
    let next = mask.clone().shader_callback(replacement.clone());
    assert_eq!(next.shader(), &replacement);
    assert_eq!(next.mode(), BlendMode::Modulate);
    let next = next.blend_mode(BlendMode::ColorBurn);
    assert_eq!(next.mode(), BlendMode::ColorBurn);
    assert_eq!((calls[0].get(), calls[1].get()), (0, 0));

    let mut tree = WidgetTree::new();
    let root = tree.mount(mask.into()).unwrap();
    let render = tree.render_id(root);
    let view = Constraints::at_most(100., 100.);
    tree.layout(view).unwrap();
    assert_eq!((calls[0].get(), calls[1].get()), (0, 0));
    assert!(tree.paint().commands().iter().any(|op| matches!(
        op,
        PaintCommand::PushShaderMask { shader: Brush::Solid(color), blend_mode: BlendMode::Modulate, .. }
            if *color == red
    )));
    assert_eq!((calls[0].get(), calls[1].get()), (1, 0));

    tree.update(root, next.into()).unwrap();
    tree.layout(view).unwrap();
    assert_eq!(tree.render_id(root), render);
    assert_eq!((calls[0].get(), calls[1].get()), (1, 0));
    assert!(tree.paint().commands().iter().any(|op| matches!(
        op,
        PaintCommand::PushShaderMask { shader: Brush::Solid(color), blend_mode: BlendMode::ColorBurn, .. }
            if *color == blue
    )));
    assert_eq!((calls[0].get(), calls[1].get()), (1, 1));
}
