//! Flutter's `BoxDecoration.borderRadius` rounds each corner by its own
//! radius; a Container keeps all four.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_rendering::{CornerRadii, PaintCommand};
use incular_widgets::{
    BorderRadius, BoxDecoration, Container, Radius, Widget, internal::WidgetTree,
};

fn painted_radii(widget: Widget) -> Vec<CornerRadii> {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::RRect { rrect, .. } => Some(rrect.radii),
            _ => None,
        })
        .collect()
}

const RADII: CornerRadii = CornerRadii {
    top_left: 0.,
    top_right: 16.,
    bottom_right: 8.,
    bottom_left: 4.,
};

fn radius() -> BorderRadius {
    BorderRadius::only(
        Radius::circular(0.),
        Radius::circular(16.),
        Radius::circular(8.),
        Radius::circular(4.),
    )
}

#[test]
fn container_keeps_each_corner() {
    let container = Container::new().width(100.).height(100.).decoration(
        BoxDecoration::new()
            .color(Color::WHITE)
            .border_radius(radius()),
    );
    assert!(painted_radii(container.into()).contains(&RADII));
}
