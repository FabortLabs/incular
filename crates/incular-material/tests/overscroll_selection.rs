//! Flutter's `MaterialScrollBehavior.buildOverscrollIndicator`: the theme
//! picks the Android overscroll indicator, and the Material 2 glow takes the
//! colour scheme's secondary colour.

use incular_config::TargetPlatform;
use incular_material::{MaterialScrollBehavior, ThemeData};
use incular_widgets::AndroidOverscrollIndicator;

fn behavior_on(platform: TargetPlatform) -> MaterialScrollBehavior {
    MaterialScrollBehavior::new().platform(platform)
}

#[test]
fn material3_android_stretches() {
    let theme = ThemeData::light_shared();
    assert!(theme.core().use_material3);
    let behavior = behavior_on(TargetPlatform::Android).scroll_behavior_for(&theme);
    assert_eq!(
        behavior.overscroll_indicator(),
        Some(AndroidOverscrollIndicator::Stretch)
    );
}

#[test]
fn material2_android_glows_in_the_secondary_colour() {
    let theme = ThemeData::light_shared()
        .as_ref()
        .clone()
        .with_use_material3(false);
    let behavior = behavior_on(TargetPlatform::Android).scroll_behavior_for(&theme);
    assert_eq!(
        behavior.overscroll_indicator(),
        Some(AndroidOverscrollIndicator::Glow)
    );
    assert_eq!(
        behavior.get_glow_color(),
        theme.core().color_scheme.secondary
    );
}

#[test]
fn material_fuchsia_glows_and_other_platforms_show_nothing() {
    let theme = ThemeData::light_shared();
    let fuchsia = behavior_on(TargetPlatform::Fuchsia).scroll_behavior_for(&theme);
    assert_eq!(
        fuchsia.overscroll_indicator(),
        Some(AndroidOverscrollIndicator::Glow)
    );
    for platform in [
        TargetPlatform::Ios,
        TargetPlatform::Linux,
        TargetPlatform::MacOs,
        TargetPlatform::Windows,
    ] {
        let behavior = behavior_on(platform).scroll_behavior_for(&theme);
        assert_eq!(behavior.overscroll_indicator(), None, "{platform:?}");
    }
}

/// The largest vertical scale among the layers' local transforms in a
/// compositor debug dump (`Affine([a, b, c, d, e, f])`, `d` the y scale).
fn max_vertical_scale(dump: &str) -> f64 {
    dump.split("local=Transform(Affine([")
        .skip(1)
        .filter_map(|rest| rest.split(',').nth(3)?.trim().parse::<f64>().ok())
        .fold(0.0, f64::max)
}

/// A Material 3 Android list stretches its content when dragged past the top:
/// the scroll view's content layer takes a non-uniform scale.
#[test]
fn material3_android_drag_past_the_top_stretches_the_list() {
    use incular_config::Constraints;
    use incular_core::{Color, Offset, PointerPhase, Size};
    use incular_widgets::internal::WidgetTree;
    use incular_widgets::{ListView, ScrollConfiguration, Widget};
    use std::time::{Duration, Instant};

    let theme = ThemeData::light_shared();
    let behavior = behavior_on(TargetPlatform::Android).scroll_behavior_for(&theme);
    let rows = (0..10).map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE));
    let mut tree = WidgetTree::new();
    tree.mount(
        ScrollConfiguration::new(ListView::new(rows.collect::<Vec<_>>()))
            .behavior(behavior)
            .into(),
    )
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    let resting = tree.compositor_debug_tree();
    let base = Instant::now();
    for (phase, y, millis) in [
        (PointerPhase::Down, 10., 0),
        (PointerPhase::Move, 40., 100),
        (PointerPhase::Move, 90., 200),
    ] {
        tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(50., y),
            phase,
            time: base + Duration::from_millis(millis),
        });
    }
    tree.update_compositor(base + Duration::from_millis(216))
        .expect("frame");
    let stretched = tree.compositor_debug_tree();
    assert!(max_vertical_scale(&resting) <= 1.0, "{resting}");
    assert!(max_vertical_scale(&stretched) > 1.01, "{stretched}");
}
