//! Flutter's Material 3 `Drawer`: 304 wide (`_kWidth`), its child
//! unpadded, its end corners rounded by 16 (`_DrawerDefaultsM3.shape`).

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_material::Drawer;
use incular_rendering::{CornerRadii, PaintCommand};
use incular_widgets::{Directionality, Widget, internal::WidgetTree};

#[test]
fn open_drawer_panel_geometry() {
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            Drawer::new()
                .open(true)
                .child(Widget::box_(Size::new(600., 400.), Color::WHITE))
                .panel(Widget::box_(Size::new(50., 50.), Color::BLACK)),
        )
        .into(),
    )
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(600., 400.)))
        .expect("layout");
    let panel = tree
        .paint()
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::RRect { rrect, .. } if rrect.radii != CornerRadii::ZERO => Some(*rrect),
            _ => None,
        })
        .expect("panel surface");
    assert_eq!(panel.rect.size.width, 304.);
    assert_eq!(
        panel.radii,
        CornerRadii {
            top_left: 0.,
            top_right: 16.,
            bottom_right: 16.,
            bottom_left: 0.,
        }
    );
}
