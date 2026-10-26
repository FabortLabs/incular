//! Flutter's Material 3 button defaults: stadium-shaped buttons at least
//! 64x40, and a 40x40 icon button.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_material::{ElevatedButton, IconButton, TextButton};
use incular_rendering::{PaintCommand, RRect};
use incular_widgets::{Directionality, SizedBox, Widget, internal::WidgetTree};

fn surfaces(widget: Widget) -> Vec<RRect> {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, widget).into())
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

#[test]
fn text_buttons_are_40_tall_stadiums() {
    for button in [
        Widget::from(ElevatedButton::new("OK").on_click(|| {})),
        Widget::from(TextButton::new("OK").on_click(|| {})),
    ] {
        let rects = surfaces(button);
        assert!(
            rects.iter().any(|rrect| rrect.rect.size.height == 40.
                && rrect.rect.size.width >= 64.
                && rrect.radii.top_left == 20.),
            "{rects:?}"
        );
    }
}

#[test]
fn icon_button_is_40_square() {
    let rects = surfaces(
        IconButton::icon(SizedBox::square(24.))
            .on_click(|| {})
            .into(),
    );
    assert!(
        rects
            .iter()
            .any(|rrect| rrect.rect.size == Size::new(40., 40.) && rrect.radii.top_left == 20.),
        "{rects:?}"
    );
}
