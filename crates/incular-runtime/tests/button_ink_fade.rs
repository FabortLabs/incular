//! A button's hover overlay fades in as Flutter's `InkWell` highlight does
//! (`getFadeDurationForType`: 50 ms for hover) instead of switching in one
//! frame, while its background color changes at once as `Material`'s does.

use incular_config::Constraints;
use incular_controls::{Button, ButtonStyle};
use incular_core::time::Instant;
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{Align, Alignment, Directionality, TextDirection, Widget};
use std::time::Duration;

const OVERLAY: Color = Color::rgba(255, 0, 0, 200);

fn bounds() -> Constraints {
    Constraints::tight(Size::new(200., 100.))
}

/// The alpha of the red overlay painted this frame, if any.
fn overlay_alpha(runtime: &mut Runtime, now: Instant) -> u8 {
    let (list, _) = runtime.run_frame_at(bounds(), now).expect("frame");
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::RRect {
                brush: Brush::Solid(color),
                ..
            } if color.red == 255 && color.green == 0 && color.blue == 0 => Some(color.alpha),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn the_hover_overlay_fades_in_over_fifty_milliseconds() {
    let style = ButtonStyle::default().overlay_color(OVERLAY).elevation(0.0);
    let button: Widget = Button::builder()
        .label("Go".to_string())
        .style(style)
        .build()
        .into();
    let root = Directionality::new(TextDirection::Ltr, Align::new(Alignment::CENTER, button));
    let mut runtime = Runtime::new(root.into()).expect("mount");
    let start = Instant::now();
    assert_eq!(overlay_alpha(&mut runtime, start), 0);
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Move,
        position: Offset::new(100., 50.),
    });
    let first = overlay_alpha(&mut runtime, start + Duration::from_millis(16));
    let middle = overlay_alpha(&mut runtime, start + Duration::from_millis(41));
    let settled = overlay_alpha(&mut runtime, start + Duration::from_millis(120));
    assert_eq!(settled, OVERLAY.alpha, "the overlay settles at its color");
    assert!(
        first < settled && middle > first && middle < settled,
        "the overlay fades: {first} then {middle} then {settled}"
    );
}
