use incular_config::{EdgeInsets, EdgeInsetsGeometry, TextDirection};
use incular_core::{Color, Lerp, Offset, Rect, Size};
use incular_rendering::{Brush, Canvas, PaintCommand};
use incular_widgets::{Border, BorderDirectional, BorderSide, BoxBorderStack, ShapeBorder};

const GREEN: Color = Color::rgba(0, 255, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn rect() -> Rect {
    Rect::from_origin_size(Offset::new(11., 15.), Size::new(288., 160.))
}

fn inset(distance: f32) -> Rect {
    Rect::from_origin_size(
        Offset::new(11. + distance, 15. + distance),
        Size::new(288. - 2. * distance, 160. - 2. * distance),
    )
}

fn diagnostics(directional: bool, green: &str, blue: &str, extras: &str) -> String {
    let side = format!(
        "BorderSide(color: Color(alpha: 1.0000, red: 0.0000, green: {green}, blue: {blue}, colorSpace: ColorSpace.sRGB){extras})"
    );
    if directional {
        format!("BorderDirectional(top: {side}, start: {side}, end: {side}, bottom: {side})")
    } else {
        format!("Border.all({side})")
    }
}

fn assert_paint(stack: &BoxBorderStack, direction: Option<TextDirection>, colors: &[Color]) {
    let mut canvas = Canvas::default();
    stack.paint_with_direction(&mut canvas, rect(), direction);
    let list = canvas.finish();
    assert_eq!(list.commands().len(), colors.len(), "{:?}", list.commands());
    for (index, (command, color)) in list.commands().iter().zip(colors).enumerate() {
        let PaintCommand::StrokePath {
            path,
            brush: Brush::Solid(actual),
            stroke,
        } = command
        else {
            panic!("expected a solid rectangular stroke: {command:?}");
        };
        assert_eq!(path.bounds(), Some(inset(index as f32 + 0.5)));
        assert_eq!(*actual, *color);
        assert_eq!(stroke.width, 1.);
    }
}

fn compare(
    stack: BoxBorderStack,
    reversed: BoxBorderStack,
    merged: BoxBorderStack,
    triple: BoxBorderStack,
    directional: bool,
) {
    let direction = directional.then_some(TextDirection::Rtl);
    let describe = |g, b, extras| diagnostics(directional, g, b, extras);
    let g = |extras| describe("1.0000", "0.0000", extras);
    let b = |extras| describe("0.0000", "1.0000", extras);

    assert_eq!(stack.to_string(), format!("{} + {}", g(""), b("")));
    assert_eq!(
        merged.to_string(),
        format!("{} + {}", g(""), b(", width: 2.0"))
    );
    assert_eq!(
        stack.scale(3.).to_string(),
        format!("{} + {}", g(", width: 3.0"), b(", width: 3.0"))
    );
    let zero = stack.scale(0.);
    assert_eq!(
        zero.to_string(),
        format!(
            "{} + {}",
            g(", width: 0.0, style: none"),
            b(", width: 0.0, style: none")
        )
    );
    assert_paint(&zero, direction, &[]);

    assert_eq!(reversed.lerp(&stack, 0.), reversed);
    assert_eq!(reversed.lerp(&stack, 1.), stack);
    for (t, low, high, byte) in [
        (0.20, "0.2000", "0.8000", 51),
        (1. / 3., "0.3333", "0.6667", 85),
    ] {
        let value = reversed.lerp(&stack, t);
        assert_eq!(
            value.to_string(),
            format!("{} + {}", describe(low, high, ""), describe(high, low, ""))
        );
        assert_paint(
            &value,
            direction,
            &[
                Color::rgba(0, 255 - byte, byte, 255),
                Color::rgba(0, byte, 255 - byte, 255),
            ],
        );
    }

    let geometry = |width| {
        if directional {
            EdgeInsetsGeometry::from_steb(width, width, width, width)
        } else {
            EdgeInsetsGeometry::all(width)
        }
    };
    assert_eq!(stack.dimensions_geometry(), geometry(2.));
    assert_eq!(triple.dimensions_geometry(), geometry(3.));
    assert_eq!(
        stack.inner_path_with_direction(rect(), direction).bounds(),
        Some(inset(2.))
    );
    assert_eq!(
        triple.inner_path_with_direction(rect(), direction).bounds(),
        Some(inset(3.))
    );
    assert_paint(&stack, direction, &[BLUE, GREEN]);
    assert_paint(&triple, direction, &[GREEN, BLUE, GREEN]);
}

#[test]
fn physical_compound_borders_compose_and_paint_outside_first() {
    let green = Border::new(1., GREEN);
    let blue = Border::new(1., BLUE);
    let merged = (green + blue) + blue;
    assert_eq!(merged, green + (blue + blue));
    compare(
        (green + blue).into(),
        (blue + green).into(),
        merged,
        (green + blue) + green,
        false,
    );
}

#[test]
fn directional_compound_borders_preserve_logical_dimensions_and_paint_in_rtl() {
    let all = |side| BorderDirectional::only(side, side, side, side);
    let green = all(BorderSide::solid(GREEN, 1.));
    let blue = all(BorderSide::solid(BLUE, 1.));
    let merged = (green + blue) + blue;
    assert_eq!(merged, green + (blue + blue));
    compare(
        green + blue,
        blue + green,
        merged,
        (green + blue) + green,
        true,
    );
}

#[test]
fn mixed_compound_insets_do_not_flip_physical_sides() {
    let physical = Border::only(
        BorderSide::NONE,
        BorderSide::NONE,
        BorderSide::NONE,
        BorderSide::solid(GREEN, 4.),
    );
    let directional = BorderDirectional::only(
        BorderSide::NONE,
        BorderSide::solid(BLUE, 2.),
        BorderSide::solid(BLUE, 3.),
        BorderSide::NONE,
    );
    let stack = BoxBorderStack::from(physical) + directional;
    assert_eq!(
        stack.dimensions(Some(TextDirection::Ltr)),
        EdgeInsets::only(6., 0., 3., 0.)
    );
    assert_eq!(
        stack.dimensions(Some(TextDirection::Rtl)),
        EdgeInsets::only(7., 0., 2., 0.)
    );
    assert_eq!(
        stack
            .inner_path_with_direction(rect(), Some(TextDirection::Rtl))
            .bounds(),
        Some(Rect::from_origin_size(
            Offset::new(18., 15.),
            Size::new(279., 160.)
        ))
    );
}

#[test]
fn mixed_borders_merge_only_if_they_do_not_compete_for_horizontal_edges() {
    let green = BorderSide::solid(GREEN, 1.);
    let left = Border::only(green, BorderSide::NONE, green, green);
    let top = BorderDirectional::only(green, BorderSide::NONE, BorderSide::NONE, BorderSide::NONE);
    let start = BorderDirectional::only(green, green, BorderSide::NONE, BorderSide::NONE);
    let physical_top = Border::only(green, BorderSide::NONE, green, BorderSide::NONE);

    let merged = left + top;
    assert_eq!(merged.layers().len(), 1);
    assert_eq!(
        merged.dimensions_geometry(),
        EdgeInsetsGeometry::from_ltrb(1., 2., 0., 1.)
    );
    let merged = start + physical_top;
    assert_eq!(merged.layers().len(), 1);
    assert_eq!(
        merged.dimensions_geometry(),
        EdgeInsetsGeometry::from_steb(1., 2., 0., 1.)
    );
    assert_eq!((left + start).layers().len(), 2);
}
