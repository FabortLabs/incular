//! Flutter's page transitions by platform (`PageTransitionsTheme`): the
//! Zoom transition's scale sequence and surface scrim, FadeForwards' slide,
//! and the durations each builder animates over. Expected values come from
//! Flutter 3.41.7's tween sequence and curves.

use std::time::Duration;

use incular_config::{Constraints, TargetPlatform};
use incular_core::{Color, Offset, Size, time::Instant};
use incular_material::{PageTransitionsBuilder, ThemeData};
use incular_rendering::{DisplayList, PaintCommand};
use incular_widgets::Widget;
use incular_widgets::internal::WidgetTree;

const PAGE: Color = Color::rgba(255, 0, 0, 255);

fn page() -> Widget {
    Widget::box_(Size::new(100., 100.), PAGE)
}

fn paint(widget: Widget) -> DisplayList {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree.paint()
}

/// The x scale of every transform pushed in the frame.
fn scales(list: &DisplayList) -> Vec<f32> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushTransform { transform } => Some(
                transform.transform_point(Offset::new(1., 0.)).x
                    - transform.transform_point(Offset::ZERO).x,
            ),
            _ => None,
        })
        .collect()
}

#[test]
fn platforms_default_to_flutters_builders() {
    let theme = ThemeData::light();
    let builders = theme.core().page_transitions_theme.builders;
    assert!(matches!(
        builders.for_platform(TargetPlatform::Windows),
        PageTransitionsBuilder::Zoom { .. }
    ));
    assert!(matches!(
        builders.for_platform(TargetPlatform::Android),
        PageTransitionsBuilder::FadeForwards { .. }
    ));
    assert_eq!(
        builders.for_platform(TargetPlatform::Ios),
        PageTransitionsBuilder::Cupertino
    );
    assert_eq!(
        PageTransitionsBuilder::FadeForwards {
            background_color: None
        }
        .transition_duration(),
        Duration::from_millis(450)
    );
}

#[test]
fn the_zoom_transition_scales_in_over_a_surface_scrim() {
    let theme = ThemeData::light().with_platform(TargetPlatform::Windows);
    let surface = theme.core().color_scheme.surface;
    let transition = theme.page_route_transition();
    let animation = transition
        .route_animation()
        .expect("a page route animates")
        .animation();
    assert_eq!(animation.duration(), Duration::from_millis(300));
    let start = Instant::now();
    animation.forward(start);
    animation.tick(start + Duration::from_millis(150));
    let list = paint(transition.apply(page()));
    // Scale 0.85 -> 1.0 along Flutter's sequence, which is 0.937036 at 0.5.
    let expected = 0.85 + 0.15 * 0.937_036;
    assert!(
        scales(&list)
            .iter()
            .any(|scale| (scale - expected).abs() < 1e-4),
        "{:?}",
        scales(&list)
    );
    // The scrim reaches 60% of the surface by a third of the way through.
    let scrim = (f32::from(surface.alpha) * 0.6).round() as u8;
    assert!(list.commands().iter().any(|command| matches!(
        command,
        PaintCommand::RRect { brush: incular_rendering::Brush::Solid(color), .. }
            if *color == Color::rgba(surface.red, surface.green, surface.blue, scrim)
    )));
    // At rest the page is drawn plainly.
    animation.tick(start + Duration::from_millis(400));
    let list = paint(transition.apply(page()));
    assert!(scales(&list).iter().all(|scale| (scale - 1.0).abs() < 1e-6));
}

#[test]
fn the_page_below_zooms_out_as_a_page_is_pushed_over_it() {
    let theme = ThemeData::light().with_platform(TargetPlatform::Windows);
    let below = theme.page_route_transition();
    let above = theme.page_route_transition();
    let route = below.route_animation().expect("animates");
    route.animation().set_value(1.0);
    route.secondary_animation().set_parent(Some(
        above
            .route_animation()
            .expect("animates")
            .animation()
            .into(),
    ));
    let start = Instant::now();
    let pushed = above.route_animation().expect("animates").animation();
    pushed.forward(start);
    pushed.tick(start + Duration::from_millis(75));
    let list = paint(below.apply(page()));
    // `_ZoomExitTransition` forward: 1.0 -> 1.05, at 0.25 (sequence 0.625357).
    let expected = 1.0 + 0.05 * 0.625_357;
    assert!(
        scales(&list)
            .iter()
            .any(|scale| (scale - expected).abs() < 1e-4),
        "{:?}",
        scales(&list)
    );
}
