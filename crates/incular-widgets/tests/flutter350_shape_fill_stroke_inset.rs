use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{BorderSide, BorderStyle, CircleBorder, ShapeDecoration};
const FILL: Color = Color::rgba(255, 0, 0, 255);

#[test]
fn opaque_outline_only_insets_the_solid_fill_by_half_the_stroke_inset() {
    for (alpha, align, style, inset) in [
        (255, -1., BorderStyle::Solid, 5.),
        (255, 0., BorderStyle::Solid, 2.5),
        (255, 1., BorderStyle::Solid, 0.),
        (254, -1., BorderStyle::Solid, 0.),
        (255, -1., BorderStyle::None, 0.),
    ] {
        let side = BorderSide {
            stroke_align: align,
            ..BorderSide::new(Color::rgba(255, 255, 255, alpha), 10., style)
        };
        let size = Size::new(100., 100.);
        let decoration = ShapeDecoration::new(CircleBorder::new(side)).color(FILL);
        let fill = decoration.paint_fill(size);
        let [
            PaintCommand::FillPath {
                path,
                brush: Brush::Solid(FILL),
                ..
            },
        ] = fill.commands()
        else {
            panic!("expected a single solid circle fill: {:?}", fill.commands());
        };
        let bounds = path.bounds().unwrap();
        let expected = Rect::from_origin_size(
            Offset::new(inset, inset),
            Size::new(100. - 2. * inset, 100. - 2. * inset),
        );
        assert!(
            (bounds.origin.x - expected.origin.x).abs() < 0.001,
            "alpha={alpha}, align={align}, style={style:?}, bounds={bounds:?}"
        );
        assert!((bounds.origin.y - expected.origin.y).abs() < 0.001);
        assert!((bounds.size.width - expected.size.width).abs() < 0.001);
        assert!((bounds.size.height - expected.size.height).abs() < 0.001);
        let outline = decoration.paint_outline(size);
        if style == BorderStyle::Solid {
            let [PaintCommand::StrokePath { path, stroke, .. }] = outline.commands() else {
                panic!("expected the same single stroke: {:?}", outline.commands());
            };
            let bounds = path.bounds().unwrap();
            assert!((bounds.size.width - (100. + align * 10.)).abs() < 0.001);
            assert_eq!(stroke.width, 10.);
        } else {
            assert!(outline.commands().is_empty());
        }
    }
}

#[test]
fn gradient_only_fill_is_not_inset_by_the_same_opaque_stroke() {
    use incular_config::Alignment;
    use incular_rendering::{GradientStop, GradientStops};
    use incular_widgets::{AlignedShapeGradient, BoxLinearGradient};
    use std::sync::Arc;

    let side = BorderSide::new(Color::WHITE, 10., BorderStyle::Solid);
    let gradient = BoxLinearGradient::new(
        Alignment::CENTER_LEFT,
        Alignment::CENTER_RIGHT,
        GradientStops::new(vec![
            GradientStop {
                offset: 0.,
                color: FILL,
            },
            GradientStop {
                offset: 1.,
                color: Color::BLACK,
            },
        ]),
    );
    let decoration = ShapeDecoration::new(CircleBorder::new(side))
        .aligned_gradient(AlignedShapeGradient::Linear(Arc::new(gradient)));
    let size = Size::new(100., 100.);
    let fill = decoration.paint_fill(size);
    let [
        PaintCommand::FillPath {
            path,
            brush: Brush::LinearGradient(_),
            ..
        },
    ] = fill.commands()
    else {
        panic!("expected a gradient fill: {:?}", fill.commands());
    };
    let bounds = path.bounds().unwrap();
    assert!((bounds.origin.x).abs() < 0.001);
    assert!((bounds.origin.y).abs() < 0.001);
    assert!((bounds.size.width - 100.).abs() < 0.001);
    assert!((bounds.size.height - 100.).abs() < 0.001);
}
