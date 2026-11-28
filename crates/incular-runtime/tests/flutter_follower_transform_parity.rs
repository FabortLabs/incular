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

#[cfg(debug_assertions)]
#[test]
fn original_leader_inside_follower_reports_the_paint_order_error_and_then_recovers() {
    use incular_widgets::internal::TreeError;

    let link = LayerLink::new();
    let paint = || Widget::box_(Size::new(20., 20.), MARKER);
    let leader = CompositedTransformTarget::new(link.clone(), paint());
    let follower = CompositedTransformFollower::new(link.clone(), leader);
    let mut runtime = Runtime::new(Center::new(follower).into()).unwrap();
    let viewport = Constraints::tight(Size::new(800., 600.));
    for _ in 0..2 {
        let error = match runtime.run_frame(viewport) {
            Err(error) => error,
            Ok(_) => panic!("a follower cannot transform its own leader"),
        };
        assert_eq!(error, TreeError::LeaderAfterFollower);
        assert!(
            error
                .to_string()
                .contains("LeaderLayer anchor must come before FollowerLayer in paint order")
        );
    }
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(
            root,
            Center::new(CompositedTransformTarget::new(
                link.clone(),
                CompositedTransformFollower::new(link.clone(), paint()),
            ))
            .into(),
        )
        .unwrap();
    runtime.run_frame(viewport).unwrap();
    assert!(link.is_linked());
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

#[test]
fn a_zero_alpha_leader_unlinks_its_follower_for_paint_and_hits_until_visible_again() {
    use incular_widgets::Opacity;

    let link = LayerLink::new();
    let scene = |opacity| {
        let leader = CompositedTransformTarget::new(link.clone(), SizedBox::square(24.));
        let follower = CompositedTransformFollower::new(
            link.clone(),
            Widget::box_(Size::new(20., 20.), MARKER).with_key(FOLLOWER),
        )
        .show_when_unlinked(false);
        Widget::from(Stack::new([
            Positioned::new(Opacity::new(opacity, leader))
                .left(30.)
                .top(40.),
            Positioned::new(follower).left(100.).top(100.),
        ]))
    };
    let constraints = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(1.)).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut first = None;
    for (opacity, visible) in [(1., true), (0., false), (0.001, false), (0.5, true)] {
        runtime.schedule_update(root, scene(opacity)).unwrap();
        let (display, _) = runtime.run_frame(constraints).unwrap();
        let tree = runtime.tree();
        let follower = tree.element_with_key(&Key::from(FOLLOWER)).unwrap();
        let renderer = tree.render_id(follower).unwrap();
        assert_eq!(renderer, *first.get_or_insert(renderer));
        assert_eq!(link.is_linked(), visible, "opacity {opacity}");
        let paints_marker = display.commands().iter().any(|op| {
            matches!(
                op, PaintCommand::Rect { color, .. } if *color == MARKER
            )
        });
        assert_eq!(paints_marker, visible);
        assert_eq!(
            tree.hit_test(Offset::new(35., 45.)),
            visible.then_some(renderer)
        );
        if visible {
            assert_eq!(painted_marker(&display), rect(30., 40., 20., 20.));
        }
    }
}

#[test]
fn rebinding_both_layers_inside_layout_keeps_the_follower_at_the_same_point() {
    use incular_widgets::{Directionality, GlobalKey, LayoutBuilder, TextDirection};

    let key = GlobalKey::new();
    let link = LayerLink::new();
    let replacement = LayerLink::new();
    let scene = |link: LayerLink| {
        Directionality::new(
            TextDirection::Ltr,
            LayoutBuilder::new(move |_, _| {
                let target = CompositedTransformTarget::new(link.clone(), SizedBox::square(10.));
                let child = Widget::box_(Size::new(20., 20.), MARKER).with_key(key);
                let follower = CompositedTransformFollower::new(link.clone(), child)
                    .target_anchor(Alignment::CENTER)
                    .follower_anchor(Alignment::CENTER);
                Stack::new([
                    Positioned::new(target).left(123.).top(456.),
                    Positioned::new(follower).left(787.).top(343.),
                ])
                .into()
            }),
        )
        .into()
    };
    let viewport = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(scene(link.clone())).unwrap();
    runtime.run_frame(viewport).unwrap();
    let root = runtime.tree().root().unwrap();
    let child = runtime.tree().element_with_key(&key.into()).unwrap();
    let expected = rect(118., 451., 20., 20.);
    assert_eq!(runtime.tree().element_bounds(child), Some(expected));
    assert!(link.is_linked());

    runtime
        .schedule_update(root, scene(replacement.clone()))
        .unwrap();
    let (frame, _) = runtime.run_frame(viewport).unwrap();
    assert_eq!(runtime.tree().element_with_key(&key.into()), Some(child));
    assert_eq!(runtime.tree().element_bounds(child), Some(expected));
    assert_eq!(painted_marker(&frame), expected);
    assert!(!link.is_linked());
    assert!(replacement.is_linked());
}

#[test]
fn a_leader_updates_padding_then_survives_an_independent_sibling_repaint() {
    use incular_config::EdgeInsets;
    use incular_widgets::{ClipRect, ColoredBox, RepaintBoundary};

    let link = LayerLink::new();
    let clear_red = Color::rgba(255, 0, 0, 0);
    let clear_green = Color::rgba(0, 255, 0, 0);
    let build = |left, sibling_color| {
        let layer = CompositedTransformTarget::new(
            link.clone(),
            RepaintBoundary::new(ClipRect::new(ColoredBox::new(
                clear_red,
                SizedBox::expand(),
            ))),
        );
        let target = Padding::new(EdgeInsets::only(left, 0., 0., 0.), layer);
        let sibling = RepaintBoundary::new(ColoredBox::new(sibling_color, SizedBox::expand()));
        Widget::from(Stack::new([
            Widget::from(target),
            Positioned::fill(sibling).into(),
        ]))
    };
    let viewport = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(build(10., Color::BLACK)).unwrap();
    let root = runtime.tree().root().unwrap();
    for (left, color) in [(10., Color::BLACK), (0., Color::BLACK), (0., clear_green)] {
        runtime.schedule_update(root, build(left, color)).unwrap();
        let (paint, _) = runtime.run_frame(viewport).unwrap();
        assert_eq!(
            link.leader_transform(),
            Some(CoreTransform::translation(Offset::new(left, 0.)))
        );
        assert_eq!(link.leader_size(), Some(Size::new(800. - left, 600.)));
        assert_eq!(
            paint
                .commands()
                .iter()
                .filter(|op| matches!(op, PaintCommand::Rect { color: fill, .. } if *fill == color))
                .count(),
            1
        );
    }
}

#[test]
fn a_follower_cannot_read_an_anchor_from_an_independent_runtime() {
    use incular_semantics::Role;
    use incular_widgets::internal::ExplicitSemantics;

    const LABEL: &str = "follower in its own view";
    let viewport = Constraints::tight(Size::new(800., 600.));
    for show in [false, true] {
        let link = LayerLink::new();
        let leader = |x, y| {
            Positioned::new(CompositedTransformTarget::new(
                link.clone(),
                SizedBox::square(24.),
            ))
            .left(x)
            .top(y)
        };
        let follower = || {
            let paint = Widget::box_(Size::new(20., 20.), MARKER)
                .semantics(ExplicitSemantics::new(Role::GenericContainer).label(LABEL))
                .with_key(FOLLOWER);
            Positioned::new(
                CompositedTransformFollower::new(link.clone(), paint)
                    .show_when_unlinked(show)
                    .offset(Offset::new(10., 0.)),
            )
            .left(80.)
            .top(70.)
        };
        let mut foreign = Runtime::new(Stack::new([leader(31., 17.)]).into()).unwrap();
        let _ = foreign.run_frame(viewport).unwrap();
        assert!(link.is_linked());
        let mut local = Runtime::new(Stack::new([follower()]).into()).unwrap();
        let (frame, _) = local.run_frame(viewport).unwrap();
        let tree = local.tree();
        let marker = tree.element_with_key(&Key::from(FOLLOWER)).unwrap();
        let renderer = tree.render_id(marker).unwrap();
        assert_ne!(tree.hit_test(Offset::new(45., 20.)), Some(renderer));
        assert_eq!(tree.hit_test(Offset::new(83., 73.)) == Some(renderer), show);
        let published = tree
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(LABEL))
            .map(|(_, node)| node.bounds);
        assert_eq!(published, show.then(|| rect(80., 70., 20., 20.)));
        if show {
            assert_eq!(painted_marker(&frame), rect(80., 70., 20., 20.));
        } else {
            assert!(
                !frame
                    .commands()
                    .iter()
                    .any(|op| matches!(op, PaintCommand::Rect { color, .. } if *color == MARKER))
            );
        }

        let root = local.tree().root().unwrap();
        local
            .schedule_update(root, Stack::new([leader(51., 27.), follower()]).into())
            .unwrap();
        let (frame, _) = local.run_frame(viewport).unwrap();
        assert_eq!(painted_marker(&frame), rect(61., 27., 20., 20.));
        let root = foreign.tree().root().unwrap();
        foreign
            .schedule_update(root, Stack::new([leader(131., 117.)]).into())
            .unwrap();
        let _ = foreign.run_frame(viewport).unwrap();
        let tree = local.tree();
        let marker = tree.element_with_key(&Key::from(FOLLOWER)).unwrap();
        let renderer = tree.render_id(marker).unwrap();
        assert_eq!(tree.hit_test(Offset::new(65., 30.)), Some(renderer));
        assert_ne!(tree.hit_test(Offset::new(45., 20.)), Some(renderer));
    }
}
