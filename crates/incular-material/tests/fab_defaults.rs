//! Flutter's `_FABDefaultsM3`: square 56/40/96 buttons with corner radii
//! 16/12/28, and a 56 px tall extended button with radius 16.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_material::FloatingActionButton;
use incular_rendering::{PaintCommand, RRect};
use incular_widgets::{Directionality, SizedBox, Widget, internal::WidgetTree};

fn surfaces(button: FloatingActionButton) -> Vec<RRect> {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, Widget::from(button)).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::RRect { rrect, .. } => Some(*rrect),
            _ => None,
        })
        .collect()
}

fn has(rrects: &[RRect], width: f32, height: f32, radius: f32) -> bool {
    rrects
        .iter()
        .any(|rrect| rrect.rect.size == Size::new(width, height) && rrect.radii.top_left == radius)
}

#[test]
fn square_sizes_and_radii() {
    let regular = surfaces(FloatingActionButton::new(SizedBox::square(24.)));
    assert!(has(&regular, 56., 56., 16.), "{regular:?}");
    let small = surfaces(FloatingActionButton::small(SizedBox::square(24.)));
    assert!(has(&small, 40., 40., 12.), "{small:?}");
    let large = surfaces(FloatingActionButton::large(SizedBox::square(36.)));
    assert!(has(&large, 96., 96., 28.), "{large:?}");
}

#[test]
fn extended_is_56_tall_with_radius_16() {
    let extended = surfaces(FloatingActionButton::extended("Create"));
    assert!(
        extended
            .iter()
            .any(|rrect| rrect.rect.size.height == 56. && rrect.radii.top_left == 16.),
        "{extended:?}"
    );
}
