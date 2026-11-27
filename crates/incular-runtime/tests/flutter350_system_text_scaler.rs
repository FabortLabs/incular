use incular_config::RuntimeEnvironment;
use incular_text::{TextScaleBoundsError, TextScaler};
use incular_widgets::{MediaQuery, ViewController, ViewMetrics};

fn scaler(view: &ViewController) -> TextScaler {
    MediaQuery::system_text_scaler_from_view(view)
}

fn view() -> ViewController {
    ViewController::primary(ViewMetrics::new(800, 600, 1.0))
}

#[test]
fn view_scaler_equality_uses_the_captured_platform_setting() {
    let view = view();
    let next = |text_scale| {
        assert!(view.set_environment(RuntimeEnvironment {
            text_scale,
            ..RuntimeEnvironment::default()
        }));
        scaler(&view)
    };
    let first = next(123.0);
    let changed = next(345.0);
    let restored = next(123.0);
    assert_eq!(first, restored);
    assert_ne!(first, changed);
    assert_eq!(first.factor(), 123.0);
    assert_eq!(changed.factor(), 345.0);
}

#[test]
fn reclamping_a_system_scaler_combines_overlap_and_collapses_disjoint_ranges() {
    let system = scaler(&view());
    let reclamp = |first: (f32, f32), second: (f32, f32)| {
        system
            .try_clamp(first.0, first.1)
            .unwrap()
            .try_clamp(second.0, second.1)
            .unwrap()
    };
    assert_eq!(reclamp((1., 1.5), (0.5, 1.)), TextScaler::no_scaling());
    assert_eq!(reclamp((1., 2.), (3., 4.)), TextScaler::linear(3.));
    assert_eq!(reclamp((5., 6.), (3., 4.)), TextScaler::linear(4.));

    let overlap = reclamp((1., 3.), (2., 4.));
    assert_eq!(overlap, system.try_clamp(2., 3.).unwrap());
    assert_eq!(overlap.scale(17.), 34.);
    // Equality must retain the bounds, not just the effective host factor:
    // [2, 4] and [2, 3] both give 2× at this platform setting.
    assert_ne!(overlap, system.try_clamp(2., 4.).unwrap());
    assert_ne!(overlap, TextScaler::linear(2.));
}

#[test]
fn clamped_system_policy_rejects_the_same_four_invalid_bounds() {
    let clamped = scaler(&view()).try_clamp(1., 5.).unwrap();
    for (min, max, error) in [
        (3., 2., TextScaleBoundsError::InvalidMaximum),
        (3., f32::NAN, TextScaleBoundsError::InvalidMaximum),
        (-1., 2., TextScaleBoundsError::NegativeMinimum),
        (
            f32::INFINITY,
            f32::INFINITY,
            TextScaleBoundsError::NonFiniteMinimum,
        ),
    ] {
        assert_eq!(clamped.try_clamp(min, max), Err(error));
    }
}
