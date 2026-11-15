//! Flutter's `PhysicalShape` (`RenderPhysicalShape.paint`) and
//! `PhysicalModel` (`RenderPhysicalModel.paint`): the surface paints in its
//! color, casts its elevation shadow, and clips the child only when a clip
//! behavior is requested. Flutter's default clip behavior is `Clip.none`.

use std::sync::Arc;

use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand, Path};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{BoxShape, PhysicalModel, PhysicalShape, SizedBox, Widget};

fn triangle() -> Arc<Path> {
    let mut builder = Path::builder();
    builder
        .move_to(Offset::ZERO)
        .line_to(Offset::new(120., 0.))
        .line_to(Offset::new(60., 120.))
        .close();
    Arc::new(builder.build())
}

fn laid_out(widget: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(300., 300.)))
        .expect("layout");
    tree
}

fn has_command(tree: &mut WidgetTree, matches: impl Fn(&PaintCommand) -> bool) -> bool {
    tree.paint().commands().iter().any(matches)
}

fn white_fill_bounds(tree: &mut WidgetTree) -> Option<Rect> {
    tree.paint()
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::FillPath {
                path,
                brush: Brush::Solid(color),
                ..
            } if *color == Color::WHITE => path.bounds(),
            _ => None,
        })
}

#[test]
fn physical_shape_paints_its_path_and_does_not_clip_by_default() {
    let child = SizedBox::new().width(40.).height(40.);
    let mut tree = laid_out(PhysicalShape::new(triangle(), Color::WHITE, child).into());

    // The fill is the triangle's own outline, not the child's box.
    let bounds = white_fill_bounds(&mut tree).expect("the shape fills in white");
    assert_eq!(bounds.size, Size::new(120., 120.));
    assert!(
        !has_command(&mut tree, |command| matches!(
            command,
            PaintCommand::PushClipPath { .. }
        )),
        "Clip.none leaves the child unclipped"
    );
}

#[test]
fn physical_shape_clips_its_child_only_when_asked() {
    let child = SizedBox::new().width(40.).height(40.);
    let mut tree = laid_out(
        PhysicalShape::new(triangle(), Color::WHITE, child)
            .clip_behavior(Clip::AntiAlias)
            .into(),
    );
    assert!(has_command(&mut tree, |command| matches!(
        command,
        PaintCommand::PushClipPath { .. }
    )));
}

#[test]
fn physical_shape_elevation_casts_a_shadow() {
    let child = SizedBox::new().width(40.).height(40.);
    let flat = laid_out(PhysicalShape::new(triangle(), Color::WHITE, child.clone()).into());
    let raised = laid_out(
        PhysicalShape::new(triangle(), Color::WHITE, child)
            .elevation(8.)
            .into(),
    );
    assert!(
        raised.compositor_debug_tree().contains("DropShadow"),
        "elevation lowers through the drop-shadow effect"
    );
    assert!(
        !flat.compositor_debug_tree().contains("DropShadow"),
        "no elevation, no shadow"
    );
}

#[test]
fn physical_model_circle_shape_and_clip_follow_the_shape() {
    let child = SizedBox::new().width(60.).height(60.);
    let mut rectangle =
        laid_out(PhysicalModel::new(Color::rgba(0, 0, 255, 255), child.clone()).into());
    let circle = laid_out(
        PhysicalModel::new(Color::rgba(0, 0, 255, 255), child)
            .shape(BoxShape::Circle)
            .into(),
    );
    assert!(
        circle.compositor_debug_tree() != rectangle.compositor_debug_tree(),
        "a circle is a different surface from a rectangle"
    );
    // The surface is a 120 x 120 blue circle: a rounded rectangle of radius
    // half the side (a `DecoratedBox` with a circle decoration).
    let mut circle_surface = laid_out(
        PhysicalModel::new(
            Color::rgba(0, 0, 255, 255),
            SizedBox::new().width(120.).height(120.),
        )
        .shape(BoxShape::Circle)
        .into(),
    );
    assert!(
        has_command(&mut circle_surface, |command| matches!(
            command,
            PaintCommand::RRect { .. } | PaintCommand::FillPath { .. }
        )),
        "the circle surface paints"
    );
    assert!(
        !has_command(&mut rectangle, |command| matches!(
            command,
            PaintCommand::PushClipPath { .. }
        )),
        "Clip.none leaves a model's child unclipped"
    );
}

#[test]
fn physical_model_clips_its_child_to_the_shape_when_asked() {
    let child = SizedBox::new().width(60.).height(60.);
    let clipped = laid_out(
        PhysicalModel::new(Color::rgba(0, 0, 255, 255), child)
            .shape(BoxShape::Circle)
            .clip_behavior(Clip::AntiAlias)
            .into(),
    );
    assert!(
        clipped.compositor_debug_tree().contains("ClipOval"),
        "a circular model clips its child with an oval clip"
    );
}
