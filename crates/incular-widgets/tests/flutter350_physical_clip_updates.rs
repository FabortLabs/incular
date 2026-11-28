use std::sync::Arc;

use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Size, time::Instant};
use incular_rendering::{ClipCoverage, PaintCommand, Path};
use incular_widgets::{
    BoxShape, PhysicalModel, PhysicalShape, Widget,
    internal::{Key, WidgetTree},
};

const SIZE: Size = Size {
    width: 100.,
    height: 40.,
};

#[derive(Clone, Copy)]
enum Surface {
    Model,
    OvalModel,
    Path,
}

impl Surface {
    fn widget(self, clip: Clip, child: Widget) -> Widget {
        match self {
            Self::Model => PhysicalModel::new(Color::WHITE, child)
                .clip_behavior(clip)
                .into(),
            Self::OvalModel => PhysicalModel::new(Color::WHITE, child)
                .shape(BoxShape::Circle)
                .clip_behavior(clip)
                .into(),
            Self::Path => PhysicalShape::new(inner_path(), Color::WHITE, child)
                .clip_behavior(clip)
                .into(),
        }
    }
}

fn inner_path() -> Arc<Path> {
    let mut path = Path::builder();
    path.move_to(Offset::new(20., 10.));
    path.line_to(Offset::new(80., 10.));
    path.line_to(Offset::new(80., 30.));
    path.line_to(Offset::new(20., 30.));
    path.close();
    Arc::new(path.build())
}

fn coverages(tree: &mut WidgetTree) -> Vec<ClipCoverage> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClipRRect { coverage, .. }
            | PaintCommand::PushClipOval { coverage, .. }
            | PaintCommand::PushClipPath { coverage, .. } => Some(*coverage),
            PaintCommand::PushClip { .. } => Some(ClipCoverage::HardEdge),
            _ => None,
        })
        .collect()
}

fn verify_replacement(surface: Surface) {
    let child_key = Key::from("physical-child");
    let child = Widget::box_(SIZE, Color::BLACK).with_key(child_key.clone());
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(surface.widget(Clip::None, child.clone()))
        .unwrap();
    let now = Instant::now();
    let bounds = Constraints::tight(SIZE);
    tree.layout(bounds).unwrap();
    tree.update_compositor(now).unwrap();
    let mounted_child = tree.element_with_key(&child_key).unwrap();
    let mounted_child_render = tree.render_id(mounted_child).unwrap();
    let mounted_render = tree.render_id(root).unwrap();
    assert!(coverages(&mut tree).is_empty());

    for (clip, expected) in [
        (Clip::AntiAlias, &[ClipCoverage::AntiAlias][..]),
        (
            Clip::AntiAliasWithSaveLayer,
            &[ClipCoverage::AntiAliasWithSaveLayer][..],
        ),
        (Clip::None, &[][..]),
    ] {
        tree.update(root, surface.widget(clip, child.clone()))
            .unwrap();
        tree.layout(bounds).unwrap();
        tree.update_compositor(now).unwrap();
        assert_eq!(tree.render_id(root), Some(mounted_render));
        assert_eq!(tree.element_with_key(&child_key), Some(mounted_child));
        assert_eq!(tree.render_id(mounted_child), Some(mounted_child_render));
        assert_eq!(coverages(&mut tree), expected, "{clip:?}");
    }
}

#[test]
fn physical_model_updates_clipping_on_the_existing_child() {
    verify_replacement(Surface::Model);
    verify_replacement(Surface::OvalModel);
}

#[test]
fn physical_shape_updates_clipping_on_the_existing_child() {
    verify_replacement(Surface::Path);
}

#[test]
fn only_the_custom_physical_shape_restricts_hit_testing_as_clip_mode_changes() {
    for (surface, hit_outside) in [(Surface::OvalModel, true), (Surface::Path, false)] {
        for clip in [Clip::None, Clip::AntiAlias] {
            let mut tree = WidgetTree::new();
            tree.mount(surface.widget(clip, Widget::box_(SIZE, Color::BLACK)))
                .unwrap();
            tree.layout(Constraints::tight(SIZE)).unwrap();
            assert_eq!(
                tree.hit_test(Offset::new(2., 2.)).is_some(),
                hit_outside,
                "{clip:?}"
            );
            assert!(tree.hit_test(Offset::new(50., 20.)).is_some(), "{clip:?}");
        }
    }
}

#[test]
fn physical_shape_matches_the_circle_tangent_in_an_eight_hundred_by_six_hundred_view() {
    use incular_core::Rect;
    use incular_widgets::{BorderSide, CircleBorder, ShapeBorder};

    let view = Size::new(800., 600.);
    let outline =
        CircleBorder::new(BorderSide::NONE).outer_path(Rect::from_origin_size(Offset::ZERO, view));
    let blue = Color::rgba(0, 0, 255, 255);
    let green = Color::rgba(0, 255, 0, 255);
    let mut tree = WidgetTree::new();
    tree.mount(
        PhysicalShape::new(
            outline,
            blue,
            Widget::box_(view, blue).with_key("physical-child"),
        )
        .shadow_color(green)
        .elevation(2.)
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(view)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let target = tree
        .render_id(tree.element_with_key(&Key::from("physical-child")).unwrap())
        .unwrap();
    for (x, y, expected) in [
        (99., 300., None),
        (100., 300., Some(target)),
        (100., 299., None),
        (100., 301., None),
    ] {
        assert_eq!(tree.hit_test(Offset::new(x, y)), expected, "({}, {})", x, y);
    }
}

#[test]
fn physical_shape_sends_its_circle_fill_shadow_color_and_elevation_to_painting() {
    use incular_core::Rect;
    use incular_rendering::Brush;
    use incular_widgets::{BorderSide, CircleBorder, ShapeBorder, SizedBox};

    let view = Size::new(800., 600.);
    let viewport = Rect::from_origin_size(Offset::ZERO, view);
    let circle = CircleBorder::new(BorderSide::NONE).outer_path(viewport);
    let blue = Color::rgba(0, 0, 255, 255);
    let green = Color::rgba(0, 255, 0, 255);
    let scene = |elevation| -> Widget {
        PhysicalShape::new(circle.clone(), blue, SizedBox::expand())
            .shadow_color(green)
            .elevation(elevation)
            .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(2.)).unwrap();
    for elevation in [2., 4., 0.] {
        tree.update(root, scene(elevation)).unwrap();
        tree.layout(Constraints::tight(view)).unwrap();
        tree.update_compositor(Instant::now()).unwrap();
        let display = tree.paint();
        let mut fills = display.commands().iter().filter_map(|op| match op {
            PaintCommand::FillPath {
                path,
                brush: Brush::Solid(color),
                ..
            } if *color == blue => Some(path),
            _ => None,
        });
        let fill = fills.next().unwrap();
        assert_eq!(fill.as_ref(), &circle);
        assert_eq!(
            fill.bounds(),
            Some(Rect::from_origin_size(
                Offset::new(100., 0.),
                Size::new(600., 600.)
            ))
        );
        assert!(fills.next().is_none());
        let shadow = display.commands().iter().find_map(|op| match op {
            PaintCommand::PushDropShadow { shadow, .. } => Some(shadow),
            _ => None,
        });
        if elevation == 0. {
            assert!(shadow.is_none());
        } else {
            let shadow = shadow.unwrap();
            assert_eq!(shadow.color, green);
            assert_eq!(shadow.offset, Offset::new(0., elevation * 0.18));
        }
    }
}
