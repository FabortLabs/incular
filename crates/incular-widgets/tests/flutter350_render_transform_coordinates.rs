use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{Matrix4, Offset, Size};
use incular_widgets::{
    Padding, SizedBox, Transform, Widget,
    internal::{ElementId, Key, WidgetTree},
};

const KEY: Key = Key::Value(350);
const POINTS: [(f32, f32); 4] = [(0., 0.), (100., 100.), (25., 75.), (50., 50.)];

fn mount(matrix: Matrix4, padding: bool) -> (WidgetTree, ElementId) {
    let leaf = Widget::from(
        SizedBox::new()
            .width(if padding { 80. } else { 100. })
            .height(100.),
    )
    .with_key(KEY);
    let child = if padding {
        Padding::new(EdgeInsets::from_ltrb(20., 0., 0., 0.), leaf).into()
    } else {
        leaf
    };
    let transform = Transform::from_matrix4(matrix, child)
        .unwrap()
        .alignment(Alignment::CENTER);
    let mut tree = WidgetTree::new();
    tree.mount(transform.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let leaf = tree.element_with_key(&KEY).unwrap();
    (tree, leaf)
}

fn offset((x, y): (f32, f32)) -> Offset {
    Offset::new(x, y)
}
fn rounded(point: Offset) -> Offset {
    Offset::new(point.x.round(), point.y.round())
}

fn both_ways(
    matrix: Matrix4,
    padded: bool,
    inputs: [(f32, f32); 4],
    outputs: [(f32, f32); 4],
    round: bool,
) {
    let (tree, leaf) = mount(matrix, padded);
    for ((point, local), global) in POINTS.into_iter().zip(inputs).zip(outputs) {
        let to_local = tree
            .element_global_to_local(leaf, offset(point), None)
            .unwrap();
        let to_global = tree
            .element_local_to_global(leaf, offset(point), None)
            .unwrap();
        assert_eq!(
            if round { rounded(to_local) } else { to_local },
            offset(local)
        );
        assert_eq!(
            if round { rounded(to_global) } else { to_global },
            offset(global)
        );
    }
}

#[test]
fn identity_round_trips_the_four_view_coordinates() {
    both_ways(Matrix4::IDENTITY, false, POINTS, POINTS, false);
}

#[test]
fn identity_still_accounts_for_inner_padding() {
    both_ways(
        Matrix4::IDENTITY,
        true,
        [(-20., 0.), (80., 100.), (5., 75.), (30., 50.)],
        [(20., 0.), (120., 100.), (45., 75.), (70., 50.)],
        false,
    );
}

#[test]
fn translation_uses_opposite_signs_for_forward_and_inverse() {
    both_ways(
        Matrix4::translation(50., 200., 0.),
        false,
        [(-50., -200.), (50., -100.), (-25., -125.), (0., -150.)],
        [(50., 200.), (150., 300.), (75., 275.), (100., 250.)],
        false,
    );
}

#[test]
fn padding_precedes_the_parent_translation() {
    both_ways(
        Matrix4::translation(50., 200., 0.),
        true,
        [(-70., -200.), (30., -100.), (-45., -125.), (-20., -150.)],
        [(70., 200.), (170., 300.), (95., 275.), (120., 250.)],
        false,
    );
}

#[test]
fn center_rotation_maps_four_points_in_each_direction() {
    let half_turn = Matrix4::rotation_z(std::f64::consts::PI);
    let expected = [(100., 100.), (0., 0.), (75., 25.), (50., 50.)];
    both_ways(half_turn, false, expected, expected, true);
}

#[test]
fn center_rotation_preserves_the_child_coordinate_offset() {
    let half_turn = Matrix4::rotation_z(std::f64::consts::PI);
    let expected = [(80., 100.), (-20., 0.), (55., 25.), (30., 50.)];
    both_ways(half_turn, true, expected, expected, true);
}

#[test]
fn inverse_of_a_tilted_plane_unprojects_both_sides_of_the_center() {
    let (tree, leaf) = mount(Matrix4::rotation_x(std::f64::consts::FRAC_PI_4), false);
    let to_local = |y| {
        tree.element_global_to_local(leaf, Offset::new(25., y), None)
            .unwrap()
    };
    assert_eq!(rounded(to_local(50.)), Offset::new(25., 50.));
    let (low, high) = (to_local(17.).y, to_local(83.).y);
    assert!(low > 0. && low < 10.);
    assert!(high > 90. && high < 100.);
    assert_eq!(low.round(), 100. - high.round());
}

#[test]
fn no_inverse_exists_for_a_plane_parallel_to_the_view() {
    let exact = Matrix4::from_columns([
        1., 0., 0., 0., 0., 0., 1., 0., 0., -1., 0., 0., 0., 0., 0., 1.,
    ]);
    let (tree, leaf) = mount(exact, false);
    assert_eq!(
        tree.element_global_to_local(leaf, Offset::new(25., 50.), None)
            .unwrap(),
        Offset::ZERO
    );
}

#[test]
fn nearly_edge_on_projection_lands_at_the_center_line() {
    let (tree, leaf) = mount(Matrix4::rotation_x(std::f64::consts::PI * 0.4999), false);
    let to_view = |y| {
        tree.element_local_to_global(leaf, Offset::new(25., y), None)
            .unwrap()
    };
    assert_eq!(to_view(50.), Offset::new(25., 50.));
    assert_eq!(rounded(to_view(75.)), Offset::new(25., 50.));
    assert_eq!(rounded(to_view(100.)), Offset::new(25., 50.));
}

#[test]
fn true_perspective_is_not_silently_flattened() {
    let mut perspective = *Matrix4::IDENTITY.columns();
    perspective[7] = 0.01;
    assert!(
        Matrix4::from_columns(perspective)
            .affine_xy_plane()
            .is_none()
    );
    assert!(
        Transform::from_matrix4(Matrix4::from_columns(perspective), SizedBox::square(100.))
            .is_none()
    );
}
