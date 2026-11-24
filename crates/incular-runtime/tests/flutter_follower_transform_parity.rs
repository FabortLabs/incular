//! Flutter 3.50.0 linked layer geometry: unequal sizes and nonmatching
//! anchors with an offset in the leader's coordinate space.

use incular_config::{Alignment, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{Brush, DisplayList, LayerLink, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, CompositedTransformFollower, CompositedTransformTarget, Padding, Positioned, SizedBox,
    Stack, Transform, Widget, internal::Key,
};

const MARKER: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const LEADER: u64 = 1;
const FOLLOWER: u64 = 2;

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn painted_geometry(list: &DisplayList, marker_color: Color) -> (Rect, CoreTransform) {
    let mut transforms = vec![CoreTransform::IDENTITY];
    let mut marker = None;
    for command in list.commands() {
        let current = *transforms.last().expect("paint transform");
        let local = match command {
            PaintCommand::PushTransform { transform } => {
                transforms.push(current.then(*transform));
                None
            }
            PaintCommand::PopTransform => {
                transforms.pop();
                None
            }
            PaintCommand::Rect { rect, color } if *color == marker_color => Some(*rect),
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } if *color == marker_color => Some(rrect.rect),
            _ => None,
        };
        if let Some(local) = local {
            assert!(marker.replace((local, current)).is_none());
        }
    }
    marker.expect("flattened paint must emit the follower marker")
}

fn painted_marker(list: &DisplayList) -> Rect {
    let (local, transform) = painted_geometry(list, MARKER);
    transform.transform_rect_bbox(local)
}

fn measure(scale: f32) -> (Rect, Rect) {
    let link = LayerLink::new();
    let leader = CompositedTransformTarget::new(
        link.clone(),
        Widget::from(SizedBox::new().width(100.).height(60.)).with_key(LEADER),
    );
    let follower = CompositedTransformFollower::new(
        link,
        Widget::box_(Size::new(30., 20.), MARKER).with_key(FOLLOWER),
    )
    .target_anchor(Alignment::BOTTOM_RIGHT)
    .follower_anchor(Alignment::TOP_CENTER)
    .offset(Offset::new(7., 11.));
    let root = Stack::new([
        Positioned::new(Transform::scale(scale, leader).alignment(Alignment::TOP_LEFT))
            .left(100.)
            .top(90.),
        Positioned::new(follower).left(300.).top(200.),
    ]);
    let mut runtime = Runtime::new(root.into()).expect("mount");
    let (list, _) = runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .expect("linked layer frame");
    let tree = runtime.tree();
    let bounds = |key| {
        let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
        tree.element_bounds(element).expect("fixture bounds")
    };
    assert_eq!(bounds(LEADER), rect(100., 90., 100. * scale, 60. * scale));
    (painted_marker(&list), bounds(FOLLOWER))
}

#[test]
fn unscaled_follower_aligns_distinct_anchors_with_nonzero_offset() {
    let expected = rect(192., 161., 30., 20.);
    assert_eq!(measure(1.), (expected, expected));
}

#[test]
fn scaled_leader_transforms_follower_geometry_and_paint() {
    let expected = rect(284., 232., 60., 40.);
    assert_eq!(measure(2.), (expected, expected), "(paint, bounds)");
}

const ANCHORS: [(Alignment, Alignment); 3] = [
    (Alignment::TOP_LEFT, Alignment::TOP_LEFT),
    (Alignment::CENTER, Alignment::CENTER),
    (Alignment::BOTTOM_RIGHT, Alignment::TOP_RIGHT),
];

#[test]
fn original_offset_only_anchors_are_exact() {
    for ((target, follower), wanted) in
        ANCHORS
            .into_iter()
            .zip([(123., 456.), (118., 451.), (113., 466.)])
    {
        let link = LayerLink::new();
        let leader =
            CompositedTransformTarget::new(link.clone(), SizedBox::new().width(10.).height(10.));
        let follower =
            CompositedTransformFollower::new(link, Widget::box_(Size::new(20., 20.), MARKER))
                .target_anchor(target)
                .follower_anchor(follower);
        let mut r = Runtime::new(
            Stack::new([
                Positioned::new(leader).left(123.).top(456.),
                Positioned::new(follower).left(787.).top(343.),
            ])
            .into(),
        )
        .unwrap();
        let (list, _) = r
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        assert_eq!(painted_marker(&list), rect(wanted.0, wanted.1, 20., 20.));
    }
}

#[test]
fn original_rotated_anchors_agree_and_the_axes_follow_the_leader() {
    rotated_anchors(false);
}

#[test]
fn original_nested_anchors_cancel_the_unlinked_parent_rotation_and_skew() {
    rotated_anchors(true);
}

#[test]
fn original_nested_target_and_follower_lay_out_at_zero_area() {
    let link = LayerLink::new();
    let follower = Widget::from(CompositedTransformFollower::new(
        link.clone(),
        SizedBox::new(),
    ))
    .with_key(FOLLOWER);
    let leader = Widget::from(CompositedTransformTarget::new(link, follower)).with_key(LEADER);
    let mut r = Runtime::new(Center::new(leader).into()).unwrap();
    r.run_frame(Constraints::tight(Size::ZERO)).unwrap();
    let tree = r.tree();
    for key in [LEADER, FOLLOWER] {
        let element = tree.element_with_key(&Key::from(key)).unwrap();
        assert_eq!(tree.element_bounds(element).unwrap().size, Size::ZERO);
    }
}

#[test]
fn original_leader_inside_follower_is_nonfatal_in_incular() {
    let link = LayerLink::new();
    let leader =
        CompositedTransformTarget::new(link.clone(), Widget::box_(Size::new(20., 20.), MARKER));
    let follower = CompositedTransformFollower::new(link, leader);
    let mut r = Runtime::new(follower.into()).unwrap();
    r.run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn rotated_anchors(nested: bool) {
    let leader_color = Color::rgba(3, 5, 211, 255);
    for (target_anchor, follower_anchor) in ANCHORS {
        let link = LayerLink::new();
        let leader = Transform::rotation(
            1.,
            CompositedTransformTarget::new(
                link.clone(),
                Widget::box_(Size::new(80., 10.), leader_color),
            ),
        );
        let follower =
            CompositedTransformFollower::new(link, Widget::box_(Size::new(40., 20.), MARKER))
                .target_anchor(target_anchor)
                .follower_anchor(follower_anchor);
        let child = if nested {
            let skew = Transform::skew(0.9, 1.1, Padding::all(20., follower))
                .alignment(Alignment::TOP_LEFT);
            Widget::from(Padding::all(
                20.,
                CompositedTransformFollower::new(LayerLink::new(), skew),
            ))
        } else {
            Widget::from(follower)
        };
        let follower = Transform::rotation(-0.3, child);
        let mut r = Runtime::new(
            Stack::new([
                Positioned::new(leader).left(456.).top(123.),
                Positioned::new(follower).left(343.).top(787.),
            ])
            .into(),
        )
        .unwrap();
        let (list, _) = r
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let (leader_rect, leader_transform) = painted_geometry(&list, leader_color);
        let (follower_rect, follower_transform) = painted_geometry(&list, MARKER);
        let target = leader_rect.origin + target_anchor.along_size(leader_rect.size);
        let follower = follower_rect.origin + follower_anchor.along_size(follower_rect.size);
        for axis in [Offset::ZERO, Offset::new(10., 0.), Offset::new(0., 10.)] {
            let a = leader_transform.transform_point(target + axis);
            let b = follower_transform.transform_point(follower + axis);
            assert!(
                (a.x - b.x).abs() < 0.001 && (a.y - b.y).abs() < 0.001,
                "{target_anchor:?}, {follower_anchor:?}, {axis:?}: {a:?} != {b:?}"
            );
        }
    }
}
