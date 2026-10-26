//! Flutter's `_CardDefaultsM3`: radius 12, a 4 px margin and no padding
//! around the child.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_material::Card;
use incular_rendering::PaintCommand;
use incular_widgets::{Directionality, Widget, internal::WidgetTree};

#[test]
fn card_surface_geometry() {
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            Card::new(Widget::box_(Size::new(80., 40.), Color::WHITE)).color(Color::BLACK),
        )
        .into(),
    )
    .expect("mount");
    tree.layout(Constraints::loose(Size::new(300., 200.)))
        .expect("layout");
    let mut origin = Offset::ZERO;
    let surface = tree
        .paint()
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::PushTransform { transform } => {
                origin = transform.transform_point(origin);
                None
            }
            PaintCommand::RRect { rrect, .. } if rrect.radii.top_left > 0. => Some(*rrect),
            _ => None,
        })
        .expect("card surface");
    assert_eq!(surface.radii.top_left, 12.);
    // No padding: the surface is the child's size.
    assert_eq!(surface.rect.size, Size::new(80., 40.));
    // The 4 px margin moves it in from the corner.
    assert_eq!(
        Offset::new(
            origin.x + surface.rect.origin.x,
            origin.y + surface.rect.origin.y
        ),
        Offset::new(4., 4.)
    );
}
