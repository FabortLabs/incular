//! Flutter's `ElevationOverlay.applySurfaceTint`: a transparent tint leaves
//! the surface colour alone, and an opaque one blends in by the Material 3
//! elevation opacity table (none at elevation 0, 5 % at elevation 1).

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_material::Material;
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::{Directionality, SizedBox, internal::WidgetTree};

fn surface_color(material: Material) -> Color {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, material).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(200., 200.)))
        .expect("layout");
    tree.paint()
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::Rect { color, .. }
            | PaintCommand::RRect {
                brush: Brush::Solid(color),
                ..
            } => Some(*color),
            _ => None,
        })
        .expect("material surface")
}

fn surface(tint: Color, elevation: f32) -> Material {
    Material::new(SizedBox::new().width(50.).height(50.))
        .color(Color::WHITE)
        .surface_tint_color(tint)
        .elevation(elevation)
}

#[test]
fn a_transparent_tint_leaves_the_surface_unchanged() {
    assert_eq!(surface_color(surface(Color::TRANSPARENT, 0.)), Color::WHITE);
    assert_eq!(surface_color(surface(Color::TRANSPARENT, 6.)), Color::WHITE);
}

#[test]
fn elevation_zero_applies_no_tint() {
    assert_eq!(
        surface_color(surface(Color::rgba(0, 0, 255, 255), 0.)),
        Color::WHITE
    );
}

#[test]
fn elevation_one_blends_five_percent_of_the_tint() {
    let color = surface_color(surface(Color::rgba(0, 0, 0, 255), 1.));
    // 255 * 0.95 = 242.25.
    assert!((241..=243).contains(&color.red), "{color:?}");
    assert_eq!(color.alpha, 255);
}
