//! Flutter's Material 3 (year2023) progress indicators: a 36 px circular
//! indicator with a 4 px stroke, and a 4 px linear bar with square corners.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_material::{CircularProgressIndicator, LinearProgressIndicator};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::{Directionality, Widget, internal::WidgetTree};

fn paint(widget: Widget) -> DisplayList {
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, widget).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(400., 400.)))
        .expect("layout");
    tree.paint()
}

#[test]
fn circular_indicator() {
    let painted = paint(CircularProgressIndicator::new().value(0.5).into());
    let (bounds, width) = painted
        .commands()
        .iter()
        .find_map(|command| match command {
            PaintCommand::StrokePath { path, stroke, .. } => Some((path.bounds()?, stroke.width)),
            _ => None,
        })
        .expect("track stroke");
    assert_eq!(width, 4.);
    // The stroke centers on a circle of radius (36 - 4) / 2 = 16.
    assert!((bounds.size.width - 32.).abs() < 0.01, "{bounds:?}");
}

#[test]
fn linear_indicator() {
    let painted = paint(LinearProgressIndicator::new().value(0.5).into());
    let heights: Vec<f32> = painted
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::RRect { rrect, .. } => {
                assert_eq!(rrect.radii.top_left, 0.);
                Some(rrect.rect.size.height)
            }
            PaintCommand::Rect { rect, .. } => Some(rect.size.height),
            _ => None,
        })
        .collect();
    assert!(!heights.is_empty());
    assert!(heights.iter().all(|height| *height == 4.), "{heights:?}");
}
