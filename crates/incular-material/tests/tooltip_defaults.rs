//! Flutter's Material `Tooltip` defaults: no wait, 1.5 s shown after a
//! long press, 4 px corners and, on desktop, a 24 px minimum height.

use std::time::Duration;

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_material::{Tooltip, TooltipController};
use incular_rendering::PaintCommand;
use incular_widgets::{Directionality, Widget, internal::WidgetTree};

#[test]
fn durations() {
    let tooltip = Tooltip::new("Hint", Widget::box_(Size::new(10., 10.), Color::WHITE));
    assert_eq!(tooltip.wait_duration_value(), Duration::ZERO);
    assert_eq!(tooltip.show_duration_value(), Duration::from_millis(1_500));
    assert_eq!(tooltip.exit_duration_value(), Duration::from_millis(100));
}

#[test]
fn desktop_tooltip_box() {
    let controller = TooltipController::new();
    controller.show();
    let tooltip = Tooltip::new("Hint", Widget::box_(Size::new(10., 10.), Color::WHITE))
        .controller(controller);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, tooltip).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    let rrects: Vec<_> = tree
        .paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::RRect { rrect, .. } => Some(*rrect),
            _ => None,
        })
        .collect();
    let popup = rrects
        .iter()
        .find(|rrect| rrect.radii.top_left == 4.)
        .unwrap_or_else(|| panic!("tooltip box in {rrects:?}"));
    assert!(popup.rect.size.height >= 24., "{popup:?}");
}
