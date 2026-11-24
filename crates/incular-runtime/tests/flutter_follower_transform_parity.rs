//! Flutter 3.50.0 linked layer geometry: unequal sizes and nonmatching
//! anchors with an offset in the leader's coordinate space.

use incular_config::{Alignment, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{Brush, DisplayList, LayerLink, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    CompositedTransformFollower, CompositedTransformTarget, Positioned, SizedBox, Stack, Transform,
    Widget, internal::Key,
};

const MARKER: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const LEADER: u64 = 1;
const FOLLOWER: u64 = 2;

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn painted_marker(list: &DisplayList) -> Rect {
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
            PaintCommand::Rect { rect, color } if *color == MARKER => Some(*rect),
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } if *color == MARKER => Some(rrect.rect),
            _ => None,
        };
        if let Some(local) = local {
            assert!(marker.replace(current.transform_rect_bbox(local)).is_none());
        }
    }
    marker.expect("flattened paint must emit the follower marker")
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
