use incular_config::{AlignmentDirectional, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{
    Brush, DisplayList, GradientStop, GradientStops, LinearGradient, PaintCommand, RRect,
    linear_gradient_t, sample_gradient_stops,
};
use incular_widgets::{
    BorderRadiusDirectional, BoxDecoration, BoxLinearGradient, Container, DecoratedBox,
    Directionality, Radius, SizedBox, Widget, internal::WidgetTree,
};

fn stops(a: Color, b: Color) -> GradientStops {
    GradientStops::new(vec![
        GradientStop {
            offset: 0.0,
            color: a,
        },
        GradientStop {
            offset: 1.0,
            color: b,
        },
    ])
}

fn gradient(list: &DisplayList) -> (&LinearGradient, RRect) {
    let mut fills = list.commands().iter().filter_map(|command| match command {
        PaintCommand::RRect {
            rrect,
            brush: Brush::LinearGradient(brush),
        } => Some((brush, *rrect)),
        _ => None,
    });
    let result = fills.next().expect("one aligned gradient fill");
    assert!(fills.next().is_none());
    result
}

fn dominance(gradient: &LinearGradient, width: f32) -> [bool; 2] {
    [10.0, width - 10.0].map(|x| {
        let t = linear_gradient_t(gradient.start, gradient.end, Offset::new(x, 20.0));
        let rgba = sample_gradient_stops(&gradient.stops, t);
        rgba[0] > rgba[2]
    })
}

#[test]
fn box_decoration_resolves_logical_linear_gradient_to_the_given_rectangle() {
    let colors = stops(Color::BLACK, Color::WHITE);
    let source = BoxLinearGradient::new(
        AlignmentDirectional::CENTER_START,
        AlignmentDirectional::BOTTOM_END,
        colors.clone(),
    );
    let decoration = BoxDecoration::new()
        .color(Color::BLACK)
        .aligned_gradient(source.clone());
    let size = Size::new(1000.0, 1000.0);
    let buffer = decoration
        .paint_background_for_direction(size, TextDirection::Rtl)
        .unwrap();
    let (brush, rrect) = gradient(&buffer);
    assert_eq!(rrect.rect, Rect::from_origin_size(Offset::ZERO, size));
    assert!(rrect.radii.is_zero());
    assert_eq!(brush.start, Offset::new(1000.0, 500.0));
    assert_eq!(brush.end, Offset::new(0.0, 1000.0));
    assert_eq!(brush.stops.id(), colors.id());
    let offset = source.resolve(
        Rect::from_origin_size(Offset::new(5.0, 11.0), size),
        TextDirection::Ltr,
    );
    assert_eq!(offset.start, Offset::new(5.0, 511.0));
    assert_eq!(offset.end, Offset::new(1005.0, 1011.0));
}

fn retained_decoration(with_radius: bool) -> BoxDecoration {
    let value = BoxDecoration::new().aligned_gradient(BoxLinearGradient::new(
        AlignmentDirectional::CENTER_START,
        AlignmentDirectional::CENTER_END,
        stops(Color::rgba(255, 0, 0, 255), Color::rgba(0, 0, 255, 255)),
    ));
    if with_radius {
        value.directional_border_radius(BorderRadiusDirectional::only(
            Radius::circular(18.0),
            Radius::ZERO,
            Radius::ZERO,
            Radius::ZERO,
        ))
    } else {
        value
    }
}

fn root_widget(direction: TextDirection, child: Widget) -> Widget {
    Directionality::new(direction, child).into()
}

fn widget(decoration: BoxDecoration, container: bool) -> Widget {
    if container {
        Container::new().decoration(decoration).into()
    } else {
        DecoratedBox::new(SizedBox::shrink())
            .decoration(decoration)
            .into()
    }
}

fn check_frame(
    tree: &mut WidgetTree,
    width: f32,
    expected: [bool; 2],
    corners: Option<(f32, f32)>,
) {
    tree.layout(Constraints::tight(Size::new(width, 40.0)))
        .unwrap();
    let buffer = tree.paint();
    let (brush, rrect) = gradient(&buffer);
    assert_eq!(rrect.rect.size, Size::new(width, 40.0));
    assert_eq!(dominance(brush, width), expected);
    if let Some((left, right)) = corners {
        assert_eq!((rrect.radii.top_left, rrect.radii.top_right), (left, right));
    }
}

#[test]
fn retained_gradient_refreshes_after_resize_or_replacing_the_decoration() {
    for (container, with_radius) in [(false, false), (true, false), (false, true), (true, true)] {
        let decoration = retained_decoration(with_radius);
        let child = widget(decoration.clone(), container);
        let direction = |direction| root_widget(direction, child.clone());
        let mut tree = WidgetTree::new();
        let root = tree.mount(direction(TextDirection::Ltr)).unwrap();
        let ltr = with_radius.then_some((18.0, 0.0));
        let rtl = with_radius.then_some((0.0, 18.0));

        check_frame(&mut tree, 100.0, [true, false], ltr);
        tree.update(root, direction(TextDirection::Rtl)).unwrap();
        check_frame(&mut tree, 100.0, [true, false], rtl);

        // Retaining the same painter at a new rectangle picks up RTL.
        check_frame(&mut tree, 120.0, [false, true], rtl);

        // A fresh painter also picks up RTL at the initial rectangle.
        let mut fresh = WidgetTree::new();
        fresh
            .mount(root_widget(
                TextDirection::Rtl,
                widget(decoration.clone(), container),
            ))
            .unwrap();
        check_frame(&mut fresh, 100.0, [false, true], rtl);

        // Replacing decoration properties invalidates the shader even if
        // the next paint has the same rectangle as the previous one.
        tree.update(
            root,
            root_widget(
                TextDirection::Ltr,
                widget(decoration.color(Color::WHITE), container),
            ),
        )
        .unwrap();
        check_frame(&mut tree, 120.0, [true, false], ltr);
    }
}
