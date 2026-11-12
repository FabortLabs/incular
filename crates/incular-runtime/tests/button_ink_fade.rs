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

/// The ink circles painted this frame: center, radius and alpha of each
/// round red rrect the size of its corner radius.
fn ripples(runtime: &mut Runtime, now: Instant) -> Vec<(Offset, f32, u8)> {
    let (list, _) = runtime.run_frame_at(bounds(), now).expect("frame");
    let mut translation = Offset::ZERO;
    let mut stack = Vec::new();
    let mut circles = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                stack.push(translation);
                translation = translation + transform.transform_point(Offset::ZERO);
            }
            PaintCommand::PopTransform => translation = stack.pop().unwrap_or(Offset::ZERO),
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } if color.red == 255
                && color.green == 0
                && (rrect.rect.size.width - rrect.rect.size.height).abs() < 1e-3
                && (rrect.radii.top_left * 2. - rrect.rect.size.width).abs() < 1e-3 =>
            {
                let rect = rrect.rect;
                let r = rect.size.width / 2.;
                circles.push((
                    rect.origin + translation + Offset::new(r, r),
                    r,
                    color.alpha,
                ));
            }
            _ => {}
        }
    }
    circles
}

#[test]
fn a_press_starts_an_ink_ripple_that_grows_then_fades_out_after_release() {
    let style = ButtonStyle::default()
        .overlay_color(OVERLAY)
        .elevation(0.0)
        .fixed_size(Size::new(80., 40.));
    let button: Widget = Button::builder()
        .label("Go".to_string())
        .style(style)
        .build()
        .into();
    let root = Directionality::new(TextDirection::Ltr, Align::new(Alignment::TOP_LEFT, button));
    let mut runtime = Runtime::new(root.into()).expect("mount");
    let start = Instant::now();
    assert!(ripples(&mut runtime, start).is_empty());
    let press = Offset::new(20., 10.);
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Down,
        position: press,
    });
    // `InkRipple`: the radius starts at 30% of half the diagonal, at the
    // press, with the fade-in at zero alpha.
    let target = 80_f32.hypot(40.) / 2.;
    let first = ripples(&mut runtime, start + Duration::from_millis(16));
    assert_eq!(first.len(), 1, "{first:?}");
    let (center, radius, alpha) = first[0];
    assert!(
        (center.x - press.x).hypot(center.y - press.y) < 0.01,
        "{center:?}"
    );
    assert!((radius - target * 0.3).abs() < 0.01, "{radius}");
    assert_eq!(alpha, 0);
    // 40 ms in: fading in (75 ms) and slowly growing (1 s unconfirmed).
    let (_, grown, fading) = ripples(&mut runtime, start + Duration::from_millis(56))[0];
    assert_eq!(fading, (f32::from(OVERLAY.alpha) * 40. / 75.).round() as u8);
    assert!(grown > radius);
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Up,
        position: press,
    });
    // Confirmed: the radius finishes over 225 ms, the alpha fades over the
    // last 40% of 375 ms.
    let _ = ripples(&mut runtime, start + Duration::from_millis(72));
    let (_, _, held) = ripples(&mut runtime, start + Duration::from_millis(250))[0];
    assert_eq!(
        held, OVERLAY.alpha,
        "full alpha until the fade-out interval"
    );
    let (_, late_radius, late) = ripples(&mut runtime, start + Duration::from_millis(400))[0];
    assert!(late < OVERLAY.alpha && late > 0, "{late}");
    assert!((late_radius - (target + 5.)).abs() < 0.01, "{late_radius}");
    // Removed once the fade-out completes.
    let _ = ripples(&mut runtime, start + Duration::from_millis(460));
    assert!(ripples(&mut runtime, start + Duration::from_millis(480)).is_empty());
}

fn mounted_elements(
    runtime: &Runtime,
) -> std::collections::HashSet<incular_widgets::internal::ElementId> {
    let tree = runtime.tree();
    let mut seen = std::collections::HashSet::new();
    let mut work: Vec<_> = tree.root().into_iter().collect();
    while let Some(id) = work.pop() {
        seen.insert(id);
        work.extend(tree.children(id).unwrap_or_default().iter().copied());
    }
    seen
}

/// The ink layer is added the first time a button shows ink; the content
/// beside it keeps its elements, so nothing below the button remounts.
#[test]
fn the_first_hover_adds_the_ink_layer_without_remounting_the_content() {
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
    let before = mounted_elements(&runtime);
    let _ = runtime.handle_input(InputEvent::Pointer {
        phase: PointerPhase::Move,
        position: Offset::new(100., 50.),
    });
    overlay_alpha(&mut runtime, start + Duration::from_millis(16));
    let settled = overlay_alpha(&mut runtime, start + Duration::from_millis(120));
    assert_eq!(settled, OVERLAY.alpha);
    let after = mounted_elements(&runtime);
    assert!(after.len() > before.len(), "the ink layer is mounted");
    let lost: Vec<_> = before.difference(&after).collect();
    assert!(lost.is_empty(), "elements remounted: {lost:?}");
}
