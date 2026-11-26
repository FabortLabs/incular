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

#[test]
fn directional_add_normalizes_empty_sides_and_layers_conflicting_colors() {
    use incular_widgets::{BoxBorderLayer, BorderStyle};

    let magenta = BorderSide::solid(Color::rgba(255, 0, 255, 255), 3.);
    let double = BorderSide::solid(magenta.color, 6.);
    let yellow = BorderSide::solid(Color::rgba(255, 255, 0, 255), 2.);
    let inactive = BorderSide::new(yellow.color, 0., BorderStyle::None);
    let zero = BorderDirectional::default();
    let directional = |top, start, end, bottom| BorderDirectional::only(top, start, end, bottom);
    let top = |top| BorderDirectional { top, ..zero };
    let start = |start| BorderDirectional { start, ..zero };
    let end = |end| BorderDirectional { end, ..zero };
    let bottom = |bottom| BorderDirectional { bottom, ..zero };
    let none = BorderSide::NONE;
    let assert_merged = |actual: BoxBorderStack, expected| {
        assert_eq!(actual.layers(), &[BoxBorderLayer::Directional(expected)]);
    };
    assert_merged(top(yellow) + end(magenta), directional(yellow, none, magenta, none));
    assert_merged(bottom(magenta) + bottom(magenta), bottom(double));
    assert_merged(
        directional(none, magenta, inactive, none) + end(yellow),
        directional(none, magenta, yellow, none),
    );
    assert_merged(zero + zero, zero);
    assert_eq!(
        (start(magenta) + start(yellow)).layers(),
        &[
            BoxBorderLayer::Directional(start(yellow)),
            BoxBorderLayer::Directional(start(magenta)),
        ]
    );
    assert_merged(top(magenta) + top(magenta), top(double));
    for actual in [top(inactive) + top(inactive), zero + zero, top(inactive) + zero, zero + top(inactive)] {
        assert_merged(actual.clone(), zero);
        assert_paint(&actual, Some(TextDirection::Ltr), &[]);
        assert_paint(&actual, Some(TextDirection::Rtl), &[]);
    }
}

#[test]
fn mixed_add_merges_compatible_edges_and_preserves_both_layer_orders() {
    use incular_widgets::BoxBorderLayer;

    let side1 = BorderSide::solid(Color::rgba(17, 17, 17, 17), 1.);
    let side2 = BorderSide::solid(Color::rgba(34, 34, 34, 34), 1.);
    let double1 = BorderSide::solid(side1.color, 2.);
    let double2 = BorderSide::solid(side2.color, 2.);
    let blank = BorderSide::NONE;
    let physical = |top, right, bottom, left| Border::only(top, right, bottom, left);
    let logical = |top, start, end, bottom| BorderDirectional::only(top, start, end, bottom);
    let single = |actual: BoxBorderStack, expected| assert_eq!(actual.layers(), &[expected]);
    let b_left = physical(blank, blank, blank, side1);
    let d_top = logical(side2, blank, blank, blank);
    let b_top = physical(side2, blank, blank, blank);
    let d_start = logical(blank, side1, blank, blank);
    assert_eq!(physical(blank, blank, blank, blank).to_string(),
        "Border.all(BorderSide(width: 0.0, style: none))");
    assert_eq!(logical(blank, blank, blank, blank).to_string(), "BorderDirectional()");
    for actual in [b_left + d_top, d_top + b_left] {
        single(actual, BoxBorderLayer::Physical(physical(side2, blank, blank, side1)));
    }
    for actual in [d_start + b_top, b_top + d_start] {
        single(actual, BoxBorderLayer::Directional(logical(side2, side1, blank, blank)));
    }

    for (b_conflict, d_conflict, b_match, d_match, doubled) in [
        (
            physical(side1, blank, blank, blank),
            logical(side2, blank, blank, blank),
            physical(side1, blank, blank, blank),
            logical(side1, blank, blank, blank),
            physical(double1, blank, blank, blank),
        ),
        (
            physical(blank, blank, side1, blank),
            logical(blank, blank, blank, side2),
            physical(blank, blank, side1, blank),
            logical(blank, blank, blank, side1),
            physical(blank, blank, double1, blank),
        ),
    ] {
        for actual in [b_conflict + d_conflict, d_conflict + b_conflict] {
            assert_eq!(actual.layers().len(), 2);
            assert!(actual.to_string().contains(" + "));
        }
        for actual in [b_match + d_match, d_match + b_match] {
            single(actual, BoxBorderLayer::Physical(doubled));
        }
    }

    for (left, right) in [(side1, blank), (blank, side1), (blank, blank)] {
        for (start, end) in [(side1, blank), (blank, side1), (blank, blank)] {
            let physical = physical(side2, right, side2, left);
            let directional = logical(side2, start, end, side2);
            let lhs = physical + directional;
            let rhs = directional + physical;
            if (left != blank || right != blank) && (start != blank || end != blank) {
                assert_eq!(
                    lhs.layers(),
                    [BoxBorderLayer::Directional(directional), BoxBorderLayer::Physical(physical)]
                );
                assert_eq!(
                    rhs.layers(),
                    [BoxBorderLayer::Physical(physical), BoxBorderLayer::Directional(directional)]
                );
                assert_eq!(lhs.to_string(), format!("{physical} + {directional}"));
                assert_eq!(rhs.to_string(), format!("{directional} + {physical}"));
            } else {
                let expected = if start == blank && end == blank {
                    BoxBorderLayer::Physical(Border { top: double2, bottom: double2, ..physical })
                } else {
                    BoxBorderLayer::Directional(BorderDirectional {
                        top: double2,
                        bottom: double2,
                        ..directional
                    })
                };
                assert_eq!(lhs.layers(), [expected]);
                assert_eq!(rhs.layers(), [expected]);
                let expected_description = match expected {
                    BoxBorderLayer::Physical(value) => value.to_string(),
                    BoxBorderLayer::Directional(value) => value.to_string(),
                };
                assert_eq!(lhs.to_string(), expected_description);
                assert_eq!(rhs.to_string(), expected_description);
            }

            for (direction, logical_left, logical_right) in [
                (TextDirection::Ltr, start, end),
                (TextDirection::Rtl, end, start),
            ] {
                let dimensions = EdgeInsets::only(
                    left.width + logical_left.width, 2., right.width + logical_right.width, 2.,
                );
                assert_eq!(lhs.dimensions(Some(direction)), dimensions);
                assert_eq!(rhs.dimensions(Some(direction)), dimensions);
            }
        }
    }
}

#[test]
fn border_hashes_follow_the_painted_edge_instead_of_only_the_side_contents() {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    let make_top = || BorderDirectional {
        top: BorderSide::solid(Color::BLACK, 2.),
        ..BorderDirectional::default()
    };
    let top = make_top();
    let bottom = BorderDirectional { bottom: top.top, ..BorderDirectional::default() };
    assert_eq!(top, make_top());
    assert_eq!(hasher.hash_one(top), hasher.hash_one(make_top()));
    assert_ne!(hasher.hash_one(top), hasher.hash_one(bottom));
    assert_ne!(
        hasher.hash_one(Border { top: top.top, ..Border::default() }),
        hasher.hash_one(Border { bottom: bottom.bottom, ..Border::default() })
    );

    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        for (border, upper, lower) in [(top, true, false), (bottom, false, true)] {
            let mut canvas = Canvas::default();
            BoxBorderStack::from(border).paint_with_direction(&mut canvas, rect(), Some(direction));
            let list = canvas.finish();
            let [PaintCommand::FillPath { path, brush: Brush::Solid(Color::BLACK), fill_rule }] =
                list.commands()
            else {
                panic!("expected just one painted edge: {list:?}");
            };
            assert_eq!(path.contains(Offset::new(150., 15.5), *fill_rule), upper);
            assert_eq!(path.contains(Offset::new(150., 174.5), *fill_rule), lower);
        }
    }
}
