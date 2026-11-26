use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, DisplayList, FillRule, PaintCommand};
use incular_widgets::{
    Border, BorderRadius, BorderRadiusDirectional, BorderSide, BoxDecoration, BoxShape, Center,
    Container, Directionality, Radius, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

const GREEN: Color = Color::rgba(0, 255, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(w, h))
}

fn mounted(widget: Widget, bounds: Constraints) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).unwrap();
    tree.layout(bounds).unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    tree
}

fn picture(size: Size, decoration: BoxDecoration) -> DisplayList {
    mounted(
        Container::new()
            .width(size.width)
            .height(size.height)
            .decoration(decoration)
            .into(),
        Constraints::tight(size),
    )
    .paint()
}

fn border_commands(list: &DisplayList) -> Vec<&PaintCommand> {
    list.commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                PaintCommand::FillPath { .. }
                    | PaintCommand::StrokePath { .. }
                    | PaintCommand::Border { .. }
            )
        })
        .collect()
}

#[test]
fn a_ten_pixel_border_wraps_a_twenty_five_pixel_child_into_forty_five() {
    let key = Key::from("outerContainer");
    let content: Widget = Container::new()
        .decoration(BoxDecoration::new().border(Border::all(BorderSide::solid(Color::BLACK, 10.))))
        .child(SizedBox::square(25.))
        .into();
    let tree = mounted(
        Center::new(content.with_key(key.clone())).into(),
        Constraints::tight(Size::new(800., 600.)),
    );
    let element = tree.element_with_key(&key).unwrap();
    assert_eq!(
        tree.element_bounds(element).unwrap().size,
        Size::new(45., 45.)
    );
}

#[test]
fn a_uniform_circle_can_paint_a_translucent_border_without_an_extra_outline() {
    let color = Color::rgba(255, 0, 255, 128);
    let list = picture(
        Size::new(100., 100.),
        BoxDecoration::new()
            .shape(BoxShape::Circle)
            .color(Color::rgba(0, 137, 123, 255))
            .border(Border::all(BorderSide::solid(color, 10.))),
    );
    let commands = border_commands(&list);
    let strokes: Vec<_> = commands
        .into_iter()
        .filter_map(|command| match command {
            PaintCommand::StrokePath {
                path,
                brush,
                stroke,
            } => Some((path.bounds(), brush.clone(), stroke.width)),
            _ => None,
        })
        .collect();
    assert_eq!(
        strokes,
        [(Some(rect(5., 5., 90., 90.)), Brush::Solid(color), 10.)]
    );
}

#[test]
fn ordinary_box_borders_emit_only_the_requested_solid_sides_or_hairlines() {
    let size = Size::new(100., 50.);
    let commands = |border| picture(size, BoxDecoration::new().border(border));
    for (width, expected) in [
        (1., rect(0.5, 0.5, 99., 49.)),
        (0., rect(0., 0., 100., 50.)),
    ] {
        let list = commands(Border::all(BorderSide::solid(Color::BLACK, width)));
        let rendered = border_commands(&list);
        assert!(
            matches!(
                rendered.as_slice(),
                [PaintCommand::StrokePath { path, brush: Brush::Solid(Color::BLACK), stroke }]
                    if stroke.width == width && path.bounds() == Some(expected)
            ) || matches!(
                rendered.as_slice(),
                [PaintCommand::Border { rrect, border }]
                    if width > 0. && border.width == width && border.color == Color::BLACK
                        && rrect.rect == rect(0., 0., 100., 50.)
            ),
            "width {width}: {rendered:?}"
        );
    }

    let none = BorderSide::NONE;
    let green = BorderSide::solid(GREEN, 10.);
    for (sides, expected) in [
        ([green, none, none, none], rect(0., 0., 100., 10.)),
        ([none, green, none, none], rect(90., 0., 10., 50.)),
        ([none, none, green, none], rect(0., 40., 100., 10.)),
        ([none, none, none, green], rect(0., 0., 10., 50.)),
    ] {
        let [top, right, bottom, left] = sides;
        let list = commands(Border::only(top, right, bottom, left));
        assert!(matches!(
            border_commands(&list).as_slice(),
            [PaintCommand::FillPath { path, brush: Brush::Solid(GREEN), .. }]
                if path.bounds() == Some(expected)
        ));
    }

    let list = commands(Border::only(
        BorderSide::solid(BLUE, 0.),
        green,
        green,
        none,
    ));
    assert!(matches!(
        border_commands(&list).as_slice(),
        [
            PaintCommand::StrokePath { brush: Brush::Solid(BLUE), stroke, .. },
            PaintCommand::FillPath { brush: Brush::Solid(GREEN), .. },
            PaintCommand::FillPath { brush: Brush::Solid(GREEN), .. },
        ] if stroke.width == 0.
    ));
}

#[test]
fn uniform_border_ring_geometry_respects_the_declared_box_shape() {
    let size = Size::new(100., 100.);
    let white = BorderSide::solid(Color::WHITE, 10.);
    let border = Border::all(white);
    let grey_top = BorderSide::solid(Color::rgba(238, 238, 238, 255), 10.);
    let varied = picture(
        size,
        BoxDecoration::new().border(Border::only(grey_top, white, white, white)),
    );
    assert_eq!(border_commands(&varied).len(), 4);
    assert!(
        border_commands(&varied)
            .iter()
            .all(|command| matches!(command, PaintCommand::FillPath { .. }))
    );

    for decoration in [
        BoxDecoration::new().border(border),
        BoxDecoration::new().border(border).shape(BoxShape::Circle),
    ] {
        let list = picture(size, decoration);
        let rendered = border_commands(&list);
        assert!(
            matches!(
                rendered.as_slice(),
                [PaintCommand::StrokePath { path, stroke, .. }]
                    if stroke.width == 10. && path.bounds() == Some(rect(5., 5., 90., 90.))
            ) || matches!(
                rendered.as_slice(),
                [PaintCommand::Border { rrect, border }]
                    if border.width == 10. && rrect.rect == rect(0., 0., 100., 100.)
            ),
            "{rendered:?}"
        );
    }

    let radius = BorderRadius::only(
        Radius::circular(0.),
        Radius::circular(10.),
        Radius::circular(0.),
        Radius::circular(0.),
    );
    let rounded = picture(
        size,
        BoxDecoration::new().border(border).border_radius(radius),
    );
    let rendered = border_commands(&rounded);
    assert!(
        matches!(
            rendered.as_slice(),
            [PaintCommand::FillPath { path, fill_rule: FillRule::EvenOdd, .. }]
                if path.bounds() == Some(rect(0., 0., 100., 100.))
                    && path.contains(Offset::new(50., 5.), FillRule::EvenOdd)
                    && !path.contains(Offset::new(50., 50.), FillRule::EvenOdd)
                    && !path.contains(Offset::new(99., 1.), FillRule::EvenOdd)
        ) || matches!(
            rendered.as_slice(),
            [PaintCommand::Border { rrect, border }]
                if rrect.rect == rect(0., 0., 100., 100.)
                    && rrect.radii.top_right == 10.
                    && border.width == 10.
                    && rrect.contains(Offset::new(50., 5.))
                    && rrect.inset(border.width).contains(Offset::new(50., 50.))
                    && !rrect.contains(Offset::new(99., 1.))
        ),
        "{rendered:?}"
    );
}

#[test]
fn logical_corner_radii_resolve_for_a_mounted_uniform_border() {
    let mut tree = mounted(
        Directionality::new(
            TextDirection::Ltr,
            Container::new().decoration(
                BoxDecoration::new()
                    .border(Border::all(BorderSide::solid(Color::BLACK, 1.)))
                    .border_radius_geometry(BorderRadiusDirectional::all(Radius::circular(1.))),
            ),
        )
        .into(),
        Constraints::tight(Size::new(100., 100.)),
    );
    let list = tree.paint();
    assert_eq!(border_commands(&list).len(), 1);
}
