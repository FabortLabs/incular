//! World transforms and pivot coordinates from the pinned widget tests.
use incular_config::Constraints;
use incular_core::{Color, Offset, Size, Transform as Affine};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::{
    Center, ClipRect, ColoredBox, RepaintBoundary, SizedBox, Transform, Widget,
    internal::WidgetTree,
};

fn child() -> RepaintBoundary {
    RepaintBoundary::new(ColoredBox::empty(Color::WHITE))
}

fn paint(root: impl Into<Widget>) -> DisplayList {
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.try_paint().unwrap()
}

fn painted_matrix(display: &DisplayList) -> Affine {
    let mut transforms = vec![Affine::IDENTITY];
    let mut result = None;
    for command in display.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                transforms.push(transforms.last().unwrap().then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            } => {
                assert!(result.replace(*transforms.last().unwrap()).is_none());
            }
            _ => {}
        }
    }
    assert_eq!(transforms.len(), 1);
    result.expect("one visible composited picture")
}

#[test]
fn quarter_turn_uses_the_exact_centered_matrix_for_an_800_by_600_child() {
    let list = paint(Transform::rotation(std::f32::consts::FRAC_PI_2, child()));
    assert_eq!(
        painted_matrix(&list).coefficients(),
        [0., 1., -1., 0., 700., -100.]
    );
}

#[test]
fn double_scale_keeps_the_original_400_by_300_visual_pivot() {
    let list = paint(Transform::scale(2., child()));
    let world = painted_matrix(&list);
    assert_eq!(world.coefficients(), [2., 0., 0., 2., -400., -300.]);
    assert_eq!(
        world.transform_point(Offset::new(400., 300.)),
        Offset::new(400., 300.)
    );
}

#[test]
fn translation_moves_the_child_origin_without_inserted_resampling() {
    let delta = Offset::new(100., 50.);
    let list = paint(Transform::translation(delta, child()));
    let world = painted_matrix(&list);
    assert!(world.is_translation());
    assert_eq!(world.transform_point(Offset::ZERO), delta);
    assert!(
        !list
            .commands()
            .iter()
            .any(|op| matches!(op, PaintCommand::PushMatrixFilter { .. }))
    );
}

#[test]
fn a_half_scale_in_a_centered_clip_starts_at_the_clip_origin() {
    let list =
        paint(Center::new(SizedBox::new().width(400.).height(300.).child(
            ClipRect::new(Transform::new(Affine::scale(0.5), child())),
        )));
    let world = painted_matrix(&list);
    let clip_origin = Offset::new(200., 150.);
    assert_eq!(world.transform_point(Offset::ZERO), clip_origin);
    assert_eq!(
        world.transform_point(Offset::new(400., 300.)),
        Offset::new(400., 300.)
    );
    let adjustment = world.translation_offset() - Affine::scale(0.5).transform_vector(clip_origin);
    assert_eq!(adjustment, Offset::new(100., 75.));
}
