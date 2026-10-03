use incular_core::Color;
use incular_semantics::SemanticState;

pub(super) use incular_core::finite_non_negative;

pub(super) fn blend_color(base: Color, overlay: Color, amount: f32) -> Color {
    let t = amount.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| {
        (f32::from(a) + (f32::from(b) - f32::from(a)) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(
        mix(base.red, overlay.red),
        mix(base.green, overlay.green),
        mix(base.blue, overlay.blue),
        mix(base.alpha, overlay.alpha),
    )
}

pub(super) fn semantic_state(enabled: bool) -> SemanticState {
    SemanticState {
        enabled,
        focusable: enabled,
        ..SemanticState::default()
    }
}
