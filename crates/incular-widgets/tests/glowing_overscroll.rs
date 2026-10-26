//! Flutter's `GlowingOverscrollIndicator` (`_GlowController.pull`).

use std::time::{Duration, Instant};

use incular_config::{AxisDirection, Constraints};
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{GlowingOverscrollIndicator, ListView, Widget};

fn glow_alphas(tree: &mut WidgetTree) -> Vec<u8> {
    tree.paint()
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::FillPath {
                brush: Brush::Solid(color),
                ..
            } if color.red == 0x21 && color.green == 0x96 && color.blue == 0xF3 => {
                Some(color.alpha)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn pulling_past_the_start_glows_at_the_leading_edge() {
    let mut tree = WidgetTree::new();
    let rows = (0..10).map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE));
    tree.mount(
        GlowingOverscrollIndicator::new(
            AxisDirection::Down,
            Color::rgba(0x21, 0x96, 0xF3, 0xFF),
            ListView::new(rows.collect::<Vec<_>>()),
        )
        .into(),
    )
    .expect("mount");
    let layout = |tree: &mut WidgetTree| {
        tree.layout(Constraints::tight(Size::new(100., 100.)))
            .expect("layout");
    };
    layout(&mut tree);
    assert!(glow_alphas(&mut tree).is_empty());
    let base = Instant::now();
    let send = |tree: &mut WidgetTree, phase, y: f32, millis: u64| {
        tree.dispatch_gesture(incular_gestures::PointerEvent {
            pointer: 1,
            position: Offset::new(50., y),
            phase,
            time: base + Duration::from_millis(millis),
        });
    };
    send(&mut tree, PointerPhase::Down, 10., 0);
    send(&mut tree, PointerPhase::Move, 40., 100);
    // 50 px of overscroll in a 100 px viewport: the opacity tween runs from
    // 0 to min(50 / 100 * 0.8, 0.5) = 0.4 over 167 ms.
    send(&mut tree, PointerPhase::Move, 90., 200);
    let frame = |tree: &mut WidgetTree, millis: u64| {
        tree.update_compositor(base + Duration::from_millis(millis))
            .expect("frame");
        layout(tree);
    };
    frame(&mut tree, 200);
    frame(&mut tree, 290);
    // 90 / 167 of the way, decelerated: 1 - (1 - 90/167)^2.
    let t = 1. - (1. - 90. / 167.0_f32).powi(2);
    let alpha = (255. * 0.4 * t).round() as u8;
    assert_eq!(alpha, 80);
    assert_eq!(glow_alphas(&mut tree), vec![alpha]);
    // Released, the pull recedes over 600 ms and the glow goes away.
    send(&mut tree, PointerPhase::Up, 90., 300);
    for millis in (300..1_500).step_by(16) {
        frame(&mut tree, millis);
    }
    assert!(glow_alphas(&mut tree).is_empty());
}
