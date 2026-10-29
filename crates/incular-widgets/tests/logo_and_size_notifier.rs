//! Flutter's `FlutterLogo` (mark only) and `SizeChangedLayoutNotifier`.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size, Transform as CoreTransform};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{FlutterLogo, IconTheme, SizeChangedLayoutNotifier, Widget};

fn laid_out_size(widget: Widget) -> Size {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(500., 500.)))
        .expect("layout");
    tree.element_bounds(root).expect("bounds").size
}

#[test]
fn the_logo_takes_its_size_or_the_icon_theme_size() {
    assert_eq!(
        laid_out_size(FlutterLogo::new().into()),
        Size::new(24., 24.)
    );
    assert_eq!(
        laid_out_size(FlutterLogo::new().size(100.).into()),
        Size::new(100., 100.)
    );
    assert_eq!(
        laid_out_size(IconTheme::new(FlutterLogo::new()).size(40.).into()),
        Size::new(40., 40.)
    );
}

#[test]
fn the_mark_is_centered_in_the_largest_square() {
    let list = FlutterLogo::paint(Size::new(300., 202.));
    let commands = list.commands();
    // A 202 square at x = (300 - 202) / 2 = 49, at scale 1, with the 166
    // wide mark centered in it: 49 + (202 - 166) / 2 = 67.
    assert_eq!(
        commands[0],
        PaintCommand::PushTransform {
            transform: CoreTransform::translation(incular_core::Offset::new(67., 0.)),
        }
    );
    let fills: Vec<Brush> = commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::FillPath { brush, .. } => Some(brush.clone()),
            _ => None,
        })
        .collect();
    let light = Brush::Solid(Color::rgba(0x54, 0xC5, 0xF8, 0xFF));
    let dark = Brush::Solid(Color::rgba(0x01, 0x57, 0x9B, 0xFF));
    assert_eq!(fills[..3], [light.clone(), light, dark]);
    assert!(matches!(fills[3], Brush::LinearGradient(_)));
    assert!(commands.iter().any(|command| matches!(
        command,
        PaintCommand::Rect { color, .. } if *color == Color::rgba(0x29, 0xB6, 0xF6, 0xFF)
    )));
}

#[test]
fn size_changes_are_reported_after_the_first_layout() {
    let reports = Rc::new(RefCell::new(Vec::new()));
    let notifier = |side: f32| -> Widget {
        let reports = reports.clone();
        SizeChangedLayoutNotifier::new(
            move |size| reports.borrow_mut().push(size),
            Widget::box_(Size::new(side, side), Color::WHITE),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(notifier(20.)).expect("mount");
    let layout = |tree: &mut WidgetTree| {
        tree.layout(Constraints::loose(Size::new(500., 500.)))
            .expect("layout");
    };
    layout(&mut tree);
    assert!(reports.borrow().is_empty());
    tree.update(root, notifier(20.)).expect("update");
    layout(&mut tree);
    assert!(reports.borrow().is_empty());
    tree.update(root, notifier(30.)).expect("update");
    layout(&mut tree);
    assert_eq!(*reports.borrow(), [Size::new(30., 30.)]);
}
