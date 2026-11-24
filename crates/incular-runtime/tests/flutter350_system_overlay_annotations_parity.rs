//! Headless system-overlay style lookup against pinned Flutter 3.50.0
//! rendering/view_chrome_style_test.dart. Asserts the style for the host,
//! not native status/nav bar drawing or platform-channel updates.

use incular_config::{Alignment, Constraints, EdgeInsets, RuntimeEnvironment};
use incular_core::{Color, Size};
use incular_platform::SystemUiOverlayStyle;
use incular_runtime::Runtime;
use incular_widgets::{Align, AnnotatedRegion, Column, Expanded, SizedBox, Widget};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xf3, 0xff);
const GREEN: Color = Color::rgba(0x4c, 0xaf, 0x50, 0xff);

fn style_for_host(root: impl Into<Widget>) -> Option<SystemUiOverlayStyle> {
    let size = Size::new(480., 960.);
    let padding = EdgeInsets {
        left: 0.,
        top: 25.,
        right: 0.,
        bottom: 54.,
    };
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime.set_environment(RuntimeEnvironment {
        viewport: size,
        physical_width: 960,
        physical_height: 1920,
        scale_factor: 2.,
        safe_insets: padding,
        view_padding: padding,
        ..RuntimeEnvironment::default()
    });
    runtime.run_frame(Constraints::tight(size)).unwrap();
    runtime.take_system_ui_overlay_style()
}

fn top_style(height: f32) -> Option<Color> {
    let annotation = AnnotatedRegion::new(
        SystemUiOverlayStyle {
            status_bar_color: Some(BLUE),
            ..SystemUiOverlayStyle::default()
        },
        SizedBox::new().width(100.).height(height),
    );
    style_for_host(Align::new(Alignment::TOP_CENTER, annotation))
        .and_then(|style| style.status_bar_color)
}

// Upstream :68, Android and iOS variants. 11.5 is below the 12.5 center.
#[test]
fn status_bar_color_is_not_applied_when_annotation_stops_before_center() {
    assert_eq!(top_style(25. / 2. - 1.), None);
}

// Upstream :89, Android and iOS variants. 13.5 crosses the 12.5 center.
#[test]
fn status_bar_color_is_applied_when_annotation_reaches_past_center() {
    assert_eq!(top_style(25. / 2. + 1.), Some(BLUE));
}

// Upstream :194 Android variant, comparing the merged style payload prior
// to the host deciding which fields the native platform can actually apply.
#[test]
fn upper_and_lower_annotations_supply_their_own_system_bar_colors() {
    let annotation = |color| -> Widget {
        Expanded::new(AnnotatedRegion::new(
            SystemUiOverlayStyle {
                status_bar_color: Some(color),
                system_navigation_bar_color: Some(color),
                ..SystemUiOverlayStyle::default()
            },
            SizedBox::expand(),
        ))
        .into()
    };
    let style =
        style_for_host(Column::new([annotation(BLUE), annotation(GREEN)])).expect("style for host");
    assert_eq!(style.status_bar_color, Some(BLUE));
    assert_eq!(style.system_navigation_bar_color, Some(GREEN));
}
