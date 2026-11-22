//! AppBar's system overlay style: Flutter annotates the whole bar with
//! `AnnotatedRegion<SystemUiOverlayStyle>`, so the status-bar icons follow the
//! bar's background unless the bar names a style itself.

use incular_config::{Brightness, Constraints, EdgeInsets, RuntimeEnvironment};
use incular_core::{Color, Size};
use incular_material::{AppBar, ThemeData};
use incular_runtime::Runtime;
use incular_widgets::{Directionality, Semantics, SystemUiOverlayStyle, TextDirection, Widget};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(220., 56.))
}

/// A 220x56 view whose status bar is 24 logical pixels tall, so the bar at the
/// top of the view covers the status-bar probe point.
fn mount(bar: AppBar) -> Runtime {
    let mut runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, bar).into()).expect("runtime");
    runtime.set_environment(RuntimeEnvironment {
        viewport: Size::new(220., 56.),
        view_padding: EdgeInsets {
            left: 0.,
            top: 24.,
            right: 0.,
            bottom: 0.,
        },
        ..RuntimeEnvironment::default()
    });
    runtime.run_frame(constraints()).expect("frame");
    runtime
}

fn title() -> Widget {
    Semantics::new(Widget::box_(Size::new(40., 20.), Color::WHITE))
        .label("Title")
        .into()
}

fn grey(channel: u8) -> Color {
    Color::rgba(channel, channel, channel, 255)
}

fn status_style(bar: AppBar) -> SystemUiOverlayStyle {
    let mut runtime = mount(bar);
    runtime
        .take_system_ui_overlay_style()
        .expect("the app bar annotates the status bar")
}

#[test]
fn a_dark_background_gets_light_status_icons() {
    let style = status_style(AppBar::new(title()).background_color(Color::BLACK));
    assert_eq!(style.status_bar_icon_brightness, Some(Brightness::Light));
    assert_eq!(style.status_bar_brightness, Some(Brightness::Dark));
}

#[test]
fn a_light_background_gets_dark_status_icons() {
    let style = status_style(AppBar::new(title()).background_color(Color::WHITE));
    assert_eq!(style.status_bar_icon_brightness, Some(Brightness::Dark));
    assert_eq!(style.status_bar_brightness, Some(Brightness::Light));
}

#[test]
fn material_three_makes_the_status_bar_transparent() {
    let style = status_style(AppBar::new(title()).background_color(Color::BLACK));
    assert_eq!(style.status_bar_color, Some(Color::TRANSPARENT));
    assert_eq!(style.system_navigation_bar_color, None);
}

#[test]
fn an_explicit_style_wins_over_the_background() {
    let style = status_style(
        AppBar::new(title())
            .background_color(Color::BLACK)
            .system_overlay_style(SystemUiOverlayStyle::DARK),
    );
    assert_eq!(style, SystemUiOverlayStyle::DARK);
}

#[test]
fn brightness_estimate_follows_flutter_luminance_threshold() {
    // Flutter: light when (relativeLuminance + 0.05)^2 > 0.15. Black has
    // luminance 0, white 1, and mid grey 0x80 about 0.216, which is dark.
    // Grey 0x9C (luminance about 0.3327) is still dark; 0xA0 (about 0.3512)
    // crosses the threshold.
    assert_eq!(
        ThemeData::estimate_brightness_for_color(Color::BLACK),
        Brightness::Dark
    );
    assert_eq!(
        ThemeData::estimate_brightness_for_color(Color::WHITE),
        Brightness::Light
    );
    assert_eq!(
        ThemeData::estimate_brightness_for_color(grey(0x80)),
        Brightness::Dark
    );
    assert_eq!(
        ThemeData::estimate_brightness_for_color(grey(0x9C)),
        Brightness::Dark
    );
    assert_eq!(
        ThemeData::estimate_brightness_for_color(grey(0xA0)),
        Brightness::Light
    );
}
