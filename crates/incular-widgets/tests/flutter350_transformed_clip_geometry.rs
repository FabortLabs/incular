//! Retained geometry counterparts to Flutter's rotated clip goldens.
use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{ClipCoverage, FillRule, LayerId, LayerKind, PaintCommand};
use incular_widgets::{
    BorderRadius, Center, ClipRRect, ClipRect, ColoredBox, Padding, Positioned, Radius,
    RepaintBoundary, SizedBox, Stack, Transform, Widget, internal::WidgetTree,
};

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn corner_radii() -> BorderRadius {
    BorderRadius::only(
        Radius::elliptical(10., 20.),
        Radius::elliptical(5., 30.),
        Radius::elliptical(15., 6.),
        Radius::elliptical(2.5, 12.),
    )
}

fn shade(color: Color) -> Widget {
    ColoredBox::empty(color).into()
}

#[test]
fn overlay_sibling_is_not_painted_under_the_previous_rect_clip() {
    let scene = RepaintBoundary::new(Stack::new([
        Positioned::from_rect(rect(0., 0., 100., 100.), ClipRect::new(shade(BLUE))),
        Positioned::from_rect(
            rect(50., 50., 100., 100.),
            Transform::rotation(1., shade(RED)),
        ),
    ]));
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let display = tree.try_paint().unwrap();
    let mut clips = Vec::new();
    let mut transforms = vec![CoreTransform::IDENTITY];
    let mut draws = Vec::new();
    for op in display.commands() {
        match op {
            PaintCommand::PushClip { rect } => clips.push(*rect),
            PaintCommand::PopClip => {
                clips.pop().expect("matching rectangle clip");
            }
            PaintCommand::PushTransform { transform } => {
                transforms.push(transforms.last().unwrap().then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, color } | PaintCommand::HardEdgeRect { rect, color } => {
                draws.push((*color, *rect, clips.clone(), *transforms.last().unwrap()));
            }
            // This scene has no rounded/path clip, external isolation, or
            // decoration border; the visible edge is the two boxes only.
            other => panic!("unexpected overlay paint operation: {other:?}"),
        }
    }
    assert_eq!(transforms.len(), 1);
    assert!(clips.is_empty());
    let [blue, red] = draws.as_slice() else {
        panic!("two straight-color draws, got {draws:?}");
    };
    assert_eq!(blue.0, BLUE);
    assert_eq!(blue.2, [rect(0., 0., 100., 100.)]);
    assert_eq!(red.0, RED);
    assert!(red.2.is_empty(), "the first sibling must restore its clip");
    assert!(!red.3.is_translation());
    let local_center = red.1.origin + Offset::new(50., 50.);
    assert_eq!(red.3.transform_point(local_center), Offset::new(100., 100.));
}

#[test]
fn a_rotated_rrect_preserves_both_axes_of_each_corner() {
    let scene = Center::new(RepaintBoundary::new(ColoredBox::new(
        Color::WHITE,
        Padding::new(
            incular_config::EdgeInsets::all(100.),
            SizedBox::square(100.).child(Transform::rotation(
                1.,
                ClipRRect::new(
                    corner_radii(),
                    ColoredBox::new(
                        RED,
                        ColoredBox::new(
                            Color::WHITE,
                            RepaintBoundary::new(Center::new(Widget::box_(
                                Size::new(10., 10.),
                                Color::BLACK,
                            ))),
                        ),
                    ),
                ),
            )),
        ),
    )));
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let display = tree.try_paint().unwrap();
    let Some((path, rule, coverage)) = display.commands().iter().find_map(|op| match op {
        PaintCommand::PushClipPath {
            path,
            fill_rule,
            coverage,
        } => Some((path, *fill_rule, *coverage)),
        _ => None,
    }) else {
        panic!("the transformed rounded clip must use its shape, not its bounding box");
    };
    assert_eq!(rule, FillRule::NonZero);
    assert_eq!(coverage, ClipCoverage::AntiAlias);
    let world = CoreTransform::translation(Offset::new(400., 300.))
        .then(CoreTransform::rotation(1.))
        .then(CoreTransform::translation(Offset::new(-50., -50.)));
    for (x, y, inside) in [
        (3., 3., false),
        (2.75, 4.25, false),
        (5., 5., true),
        (98., 2., false),
        (97., 10., true),
        (50., 0.5, true),
        (50., 50., true),
    ] {
        let point = world.transform_point(Offset::new(x, y));
        assert_eq!(path.contains(point, rule), inside, "local point ({x}, {y})");
    }
}

fn retained_clip(tree: &WidgetTree) -> LayerId {
    let layers = tree.compositor_layers();
    let mut work = vec![layers.root().unwrap()];
    let mut clip = None;
    while let Some(id) = work.pop() {
        let (kind, children) = layers.layer(id).unwrap();
        if matches!(
            kind,
            LayerKind::ClipRRect { .. } | LayerKind::ClipPath { .. }
        ) {
            assert!(clip.replace(id).is_none());
        }
        work.extend(children);
    }
    clip.unwrap()
}

#[test]
fn changing_only_vertical_radii_switches_the_existing_clip_representation() {
    let rect = rect(0., 0., 100., 100.);
    let mut tree = WidgetTree::new();
    let circular = |clip| ClipRRect::new(10., shade(BLUE)).clip_behavior(clip).into();
    let rounded = |clip| {
        ClipRRect::new(BorderRadius::all(Radius::elliptical(10., 20.)), shade(BLUE))
            .clip_behavior(clip)
            .into()
    };
    let root = tree.mount(circular(Clip::AntiAlias)).unwrap();
    let child = tree.children(root).unwrap()[0];
    let frame = |tree: &mut WidgetTree| {
        tree.layout(Constraints::tight(rect.size)).unwrap();
        tree.try_paint().unwrap()
    };
    let _ = frame(&mut tree);
    let clip = retained_clip(&tree);
    tree.update(root, rounded(Clip::AntiAlias)).unwrap();
    let ellipse = frame(&mut tree);
    let found = ellipse
        .commands()
        .iter()
        .find_map(|op| match op {
            PaintCommand::PushClipPath {
                path,
                coverage: ClipCoverage::AntiAlias,
                ..
            } => Some(path.id()),
            _ => None,
        })
        .expect("a change on the vertical axis must not be ignored");
    assert_eq!(retained_clip(&tree), clip);

    tree.update(root, rounded(Clip::HardEdge)).unwrap();
    let hard = frame(&mut tree);
    assert!(hard.commands().iter().any(|op| matches!(op,
        PaintCommand::PushClipPath { path, coverage: ClipCoverage::HardEdge, .. } if path.id() == found)));
    assert_eq!(retained_clip(&tree), clip);

    tree.update(root, circular(Clip::AntiAlias)).unwrap();
    let restored = frame(&mut tree);
    assert!(restored.commands().iter().any(|op| matches!(
        op,
        PaintCommand::PushClipRRect {
            coverage: ClipCoverage::AntiAlias,
            ..
        }
    )));
    assert_eq!(retained_clip(&tree), clip);
    assert_eq!(tree.children(root).unwrap(), [child]);
}

#[test]
fn direction_change_moves_both_axes_of_each_retained_corner() {
    use incular_config::TextDirection;
    use incular_widgets::{BorderRadiusDirectional, Directionality};

    let directional = BorderRadiusDirectional::horizontal(
        Radius::elliptical(10., 20.),
        Radius::elliptical(10., 5.),
    );
    let scene = |direction| {
        Directionality::new(
            direction,
            ClipRRect::new(0., shade(BLUE)).directional_radius(directional),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Ltr)).unwrap();
    let mut layer = None;
    for (direction, inside_left) in [(TextDirection::Ltr, false), (TextDirection::Rtl, true)] {
        tree.update(root, scene(direction)).unwrap();
        tree.layout(Constraints::tight(Size::new(100., 100.)))
            .unwrap();
        let list = tree.try_paint().unwrap();
        let (path, rule) = list
            .commands()
            .iter()
            .find_map(|op| match op {
                PaintCommand::PushClipPath {
                    path, fill_rule, ..
                } => Some((path, *fill_rule)),
                _ => None,
            })
            .unwrap();
        assert_eq!(path.contains(Offset::new(3., 3.), rule), inside_left);
        assert_eq!(path.contains(Offset::new(97., 3.), rule), !inside_left);
        let current = retained_clip(&tree);
        assert_eq!(current, *layer.get_or_insert(current));
    }
}
