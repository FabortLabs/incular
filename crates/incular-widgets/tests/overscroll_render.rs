//! Flutter's Android overscroll glow on a scroll view's own render object:
//! a drag past the edge paints the glow, and it recedes once released.
use std::time::{Duration, Instant};

use incular_config::{Constraints, TargetPlatform};
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{ListView, ScrollBehavior, ScrollConfiguration, Widget};

const GLOW: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);

fn glow_fills(tree: &mut WidgetTree) -> usize {
    tree.paint()
        .commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                PaintCommand::FillPath {
                    brush: Brush::Solid(color),
                    ..
                } if color.red == GLOW.red && color.green == GLOW.green && color.blue == GLOW.blue
            )
        })
        .count()
}

/// Mounts a list under a scroll configuration for `platform`, drags it past
/// its top edge, and returns the tree with the glow fill count after the drag.
fn drag_past_top(platform: TargetPlatform) -> (WidgetTree, Instant) {
    let mut tree = WidgetTree::new();
    let rows = (0..10).map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE));
    let behavior = ScrollBehavior::new().platform(platform).glow_color(GLOW);
    tree.mount(
        ScrollConfiguration::new(ListView::new(rows.collect::<Vec<_>>()))
            .behavior(behavior)
            .into(),
    )
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
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
    send(&mut tree, PointerPhase::Move, 90., 200);
    // The pull fades the glow in over a few frames, as Flutter's glow
    // controller does from its first tick.
    for millis in (200..=264).step_by(16) {
        tree.update_compositor(base + Duration::from_millis(millis))
            .expect("frame");
    }
    (tree, base)
}

#[test]
fn android_drag_past_the_top_paints_the_glow() {
    let (mut tree, _) = drag_past_top(TargetPlatform::Android);
    assert!(glow_fills(&mut tree) > 0);
}

#[test]
fn android_glow_recedes_after_release() {
    let (mut tree, base) = drag_past_top(TargetPlatform::Android);
    assert!(glow_fills(&mut tree) > 0);
    // Releasing ends the scroll, and the glow recedes over Flutter's 600 ms
    // `_recedeTime` (a held pull would take the 2 s `_pullDecayTime`).
    tree.dispatch_gesture(incular_gestures::PointerEvent {
        pointer: 1,
        position: Offset::new(50., 90.),
        phase: PointerPhase::Up,
        time: base + Duration::from_millis(280),
    });
    for millis in (280..1_500).step_by(16) {
        tree.update_compositor(base + Duration::from_millis(millis))
            .expect("frame");
    }
    assert_eq!(glow_fills(&mut tree), 0);
}

#[test]
fn desktop_drag_past_the_top_paints_no_glow() {
    let (mut tree, _) = drag_past_top(TargetPlatform::Windows);
    assert_eq!(glow_fills(&mut tree), 0);
}
