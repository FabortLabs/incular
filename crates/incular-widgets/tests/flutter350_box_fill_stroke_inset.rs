//! Compare the actual retained compact painter and the reference geometry.

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{
    Border, BorderRadius, BorderSide, BorderStyle, BoxDecoration, DecoratedBox, Radius, SizedBox,
    internal::WidgetTree,
};

#[test]
fn compact_opaque_borders_inset_the_fill_but_not_the_stroke_or_layout() {
    let size = Size::new(250., 250.);
    let mut tree = WidgetTree::new();
    let scene = |alpha| {
        let decor = BoxDecoration::new()
            .color(Color::BLACK)
            .border_radius(BorderRadius::all(Radius::circular(75.)))
            .border(Border::all(BorderSide::new(
                Color::rgba(255, 255, 255, alpha),
                50.,
                BorderStyle::Solid,
            )));
        DecoratedBox::new(SizedBox::shrink())
            .decoration(decor)
            .into()
    };
    let root = tree.mount(scene(255)).unwrap();
    let render = tree.render_id(root).unwrap();
    let mut first_layouts = None;
    for (alpha, inset) in [(255, 25.), (254, 0.), (255, 25.)] {
        tree.update(root, scene(alpha)).unwrap();
        tree.layout(Constraints::tight(size)).unwrap();
        assert_eq!(tree.render_id(root), Some(render));
        assert_eq!(tree.root_layout_size(), Some(size));
        let layouts = tree.diagnostics().layouts;
        assert_eq!(layouts, *first_layouts.get_or_insert(layouts));
        let full = Rect::from_origin_size(Offset::ZERO, size);
        let expected = Rect::from_origin_size(
            Offset::new(inset, inset),
            Size::new(250. - 2. * inset, 250. - 2. * inset),
        );
        let list = tree.paint();
        let compact: Vec<_> = list
            .commands()
            .iter()
            .filter(|cmd| {
                matches!(
                    cmd,
                    PaintCommand::RRect { .. } | PaintCommand::Border { .. }
                )
            })
            .collect();
        let [
            PaintCommand::RRect {
                rrect: fill,
                brush: Brush::Solid(Color::BLACK),
            },
            PaintCommand::Border {
                rrect: outline,
                border,
            },
        ] = compact.as_slice()
        else {
            panic!("expected retained compact fill and border: {compact:?}");
        };
        assert_eq!(fill.rect, expected, "alpha={alpha}");
        assert_eq!(outline.rect, full);
        assert_eq!(fill.radii, outline.radii);
        assert_eq!(border.width, 50.);
        assert_eq!(border.color.alpha, alpha);
    }
}

#[test]
fn low_level_brush_border_preserves_its_original_fill() {
    let size = Size::new(250., 250.);
    let mut tree = WidgetTree::new();
    tree.mount(
        DecoratedBox::new(SizedBox::shrink())
            .background(Color::BLACK)
            .radius(75.)
            .border(incular_rendering::Border::new(50., Color::WHITE))
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    let list = tree.paint();
    assert!(list.commands().iter().any(|command| matches!(
        command, PaintCommand::RRect { rrect, .. } if rrect.rect == Rect::from_origin_size(Offset::ZERO, size)
    )));
}

#[test]
fn compact_gradient_keeps_the_shader_in_original_box_coordinates() {
    use incular_rendering::{GradientStop, GradientStops, LinearGradient};
    let size = Size::new(250., 250.);
    let gradient = LinearGradient::new(
        Offset::ZERO,
        Offset::new(250., 250.),
        GradientStops::new(vec![
            GradientStop {
                offset: 0.,
                color: Color::BLACK,
            },
            GradientStop {
                offset: 1.,
                color: Color::WHITE,
            },
        ]),
    );
    let decorate = BoxDecoration::new()
        .gradient(Brush::LinearGradient(gradient.clone()))
        .border_radius(BorderRadius::all(Radius::circular(75.)))
        .border(Border::all(BorderSide::new(
            Color::WHITE,
            50.,
            BorderStyle::Solid,
        )));
    let mut tree = WidgetTree::new();
    tree.mount(
        DecoratedBox::new(SizedBox::shrink())
            .decoration(decorate)
            .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    let list = tree.paint();
    let (fill, shader) = list
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::RRect {
                rrect,
                brush: Brush::LinearGradient(shader),
            } => Some((rrect, shader)),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        fill.rect,
        Rect::from_origin_size(Offset::new(25., 25.), Size::new(200., 200.))
    );
    assert_eq!(shader, &gradient);
}
