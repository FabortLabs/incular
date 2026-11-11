//! `BoxDecoration` shadows paint outside the decorated box, as Flutter's
//! decoration painter does: nothing clips them to the box, and the blur layer
//! that softens them covers everything they paint, including the part an
//! offset pushes past the box.

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::PaintCommand;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BorderRadius, BoxDecoration, BoxShadow, BoxShape, Container, Positioned, Stack, Widget,
};

fn painted(decoration: BoxDecoration) -> Vec<PaintCommand> {
    let boxed = Container::new().decoration(decoration);
    let root: Widget = Stack::new([Widget::from(
        Positioned::new(boxed)
            .left(40.)
            .top(40.)
            .width(80.)
            .height(80.),
    )])
    .into();
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    for _ in 0..2 {
        tree.layout(Constraints::tight(Size::new(160., 160.)))
            .expect("layout");
        tree.update_compositor(incular_core::time::Instant::now())
            .expect("composite");
    }
    tree.paint().commands().to_vec()
}

fn shadowed() -> BoxDecoration {
    BoxDecoration::new()
        .color(Color::WHITE)
        .border_radius(BorderRadius::circular(8.))
        .box_shadow([BoxShadow::new(Color::BLACK, Offset::new(0., 4.), 16., 0.)])
}

#[test]
fn a_box_shadow_is_not_clipped_to_the_box() {
    let commands = painted(shadowed());
    let box_rect = Rect::from_origin_size(Offset::new(40., 40.), Size::new(80., 80.));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, PaintCommand::PushClip { rect } if *rect == box_rect)),
        "the decoration clips its own shadow: {commands:#?}"
    );
}

#[test]
fn the_shadow_blur_covers_the_offset_shadow() {
    let commands = painted(shadowed());
    let bounds = commands
        .iter()
        .find_map(|command| match command {
            PaintCommand::PushBlur { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .expect("a blurred shadow");
    // The shadow is the 80x80 box moved down by 4.
    assert!(
        bounds.origin.y <= 40. && bounds.origin.y + bounds.size.height >= 124.,
        "blur bounds {bounds:?} cut off the offset shadow"
    );
}

#[test]
fn rounded_and_circular_decorations_paint_as_rounded_rects() {
    for decoration in [
        shadowed(),
        BoxDecoration::new()
            .color(Color::WHITE)
            .shape(BoxShape::Circle)
            .box_shadow([BoxShadow::new(Color::BLACK, Offset::ZERO, 4., 0.)]),
    ] {
        let commands = painted(decoration);
        let rrects = commands
            .iter()
            .filter(|command| matches!(command, PaintCommand::RRect { .. }))
            .count();
        assert_eq!(rrects, 2, "shadow and background: {commands:#?}");
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, PaintCommand::FillPath { .. })),
            "circular corners need no path"
        );
    }
}
