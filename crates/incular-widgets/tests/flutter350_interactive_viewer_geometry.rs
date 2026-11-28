use incular_config::Constraints;
use incular_core::{Offset, Size};
use incular_widgets::{
    Center, InteractiveViewer as Viewer, InteractiveViewerViewport as Quad, SizedBox, Text,
    TextDirection, Widget, internal::WidgetTree,
};

fn p(x: f32, y: f32) -> Offset {
    Offset::new(x, y)
}

fn quad(coords: [(f32, f32); 4]) -> Quad {
    Quad {
        corners: coords.map(|(x, y)| p(x, y)),
    }
}

fn approximately(actual: Offset, expected: Offset) {
    assert!(
        (actual.x - expected.x).abs() <= 0.1,
        "{actual:?} vs {expected:?}"
    );
    assert!(
        (actual.y - expected.y).abs() <= 0.1,
        "{actual:?} vs {expected:?}"
    );
}

#[test]
fn nearest_segment_projection_is_inclusive_and_leaves_inputs_unchanged() {
    let point = p(5., 5.);
    let first = p(0., 0.);
    let last = p(10., 0.);
    assert_eq!(Viewer::nearest_point_on_line(point, first, last), p(5., 0.));
    assert_eq!((point, first, last), (p(5., 5.), p(0., 0.), p(10., 0.)));

    let first = p(0., 0.);
    let last = p(5., 5.);
    for (point, expected) in [
        (p(0., 5.), p(2.5, 2.5)),
        (p(-1., -1.), first),
        (p(6., 6.), last),
        (p(2., 2.), p(2., 2.)),
    ] {
        assert_eq!(Viewer::nearest_point_on_line(point, first, last), expected);
    }
    approximately(
        Viewer::nearest_point_on_line(p(-436.9, 433.6), p(-1114., -60.3), p(288.8, 432.7)),
        p(-356.8, 205.8),
    );
    assert_eq!(
        Viewer::nearest_point_on_line(p(12., 9.), first, first),
        first
    );
}

#[test]
fn viewport_bounds_cover_all_original_quads_in_clockwise_order() {
    let square = quad([(0., 0.), (10., 0.), (10., 10.), (0., 10.)]);
    for (input, expected) in [
        (square, square),
        (quad([(0., 5.), (5., 10.), (10., 5.), (5., 0.)]), square),
        (
            quad([(0., 1.), (1., 11.), (11., 9.), (9., -1.)]),
            quad([(0., -1.), (11., -1.), (11., 11.), (0., 11.)]),
        ),
        (
            quad([
                (-462.7, 165.9),
                (690.6, -576.7),
                (1188.1, 196.),
                (34.9, 938.6),
            ]),
            quad([
                (-462.7, -576.7),
                (1188.1, -576.7),
                (1188.1, 938.6),
                (-462.7, 938.6),
            ]),
        ),
    ] {
        assert_eq!(Viewer::axis_aligned_bounding_box(input), expected);
        assert_eq!(input.bounds(), expected.bounds());
    }
}

#[test]
fn closest_viewport_point_is_unchanged_inside_or_on_edge_and_projects_outside() {
    let square = quad([(0., 0.), (0., 10.), (10., 10.), (10., 0.)]);
    assert!(Viewer::point_is_inside(p(5., 5.), square));
    assert!(!Viewer::point_is_inside(p(12., 0.), square));
    assert!(Viewer::point_is_inside(p(0., 0.), square));
    assert_eq!(Viewer::nearest_point_inside(p(5., 5.), square), p(5., 5.));
    assert_eq!(Viewer::nearest_point_inside(p(5., 15.), square), p(5., 10.));

    let skew = quad([(0., 0.), (2., 10.), (12., 12.), (10., 2.)]);
    approximately(Viewer::nearest_point_inside(p(5., 15.), skew), p(5.8, 10.8));
    assert!(Viewer::point_is_inside(p(6., 6.), skew));

    let empty = quad([(0., 0.); 4]);
    assert!(!Viewer::point_is_inside(p(6., 6.), empty));
    assert_eq!(Viewer::nearest_point_inside(p(6., 6.), empty), Offset::ZERO);
}

#[test]
fn viewer_can_mount_at_zero_area_without_synthesizing_nonzero_geometry() {
    let viewer = Viewer::new(Text::new("X").text_direction(TextDirection::Ltr));
    let keyed = Widget::from(viewer).with_key(1970_u64);
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(SizedBox::shrink().child(keyed)).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let id = tree.element_with_key(&1970_u64.into()).unwrap();
    assert_eq!(tree.element_layout_size(id), Some(Size::ZERO));
}
