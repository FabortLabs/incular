//! Flutter's `StretchEffect` (transform fallback) and
//! `StretchingOverscrollIndicator` (`_StretchController.pull`).

use std::time::{Duration, Instant};

use incular_config::{Axis, AxisDirection, Constraints, TextDirection};
use incular_core::{Color, Offset, PointerPhase, Rect, Size};
use incular_widgets::internal::{ElementId, RenderKind, WidgetTree};
use incular_widgets::{
    Directionality, ListView, StretchEffect, StretchingOverscrollIndicator, Widget,
};

fn leaf_bounds(widget: Widget) -> Rect {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    let mut leaf = root;
    while let Some(&child) = tree.children(leaf).and_then(|kids| kids.first()) {
        leaf = child;
    }
    tree.element_bounds(leaf).expect("bounds")
}

fn stretched(strength: f32, axis: Axis, direction: TextDirection) -> Rect {
    leaf_bounds(
        Directionality::new(
            direction,
            StretchEffect::new(
                strength,
                axis,
                Widget::box_(Size::new(100., 100.), Color::WHITE),
            ),
        )
        .into(),
    )
}

#[test]
fn the_stretch_anchors_at_the_pulled_edge() {
    let ltr = TextDirection::Ltr;
    // Forward vertical: anchored at the top, 1 + 0.5 times as tall.
    assert_eq!(
        stretched(0.5, Axis::Vertical, ltr),
        Rect::from_origin_size(Offset::ZERO, Size::new(100., 150.))
    );
    // Backward vertical: anchored at the bottom.
    assert_eq!(
        stretched(-0.5, Axis::Vertical, ltr),
        Rect::from_origin_size(Offset::new(0., -50.), Size::new(100., 150.))
    );
    // Horizontal follows the reading direction: start is left in LTR...
    assert_eq!(
        stretched(0.5, Axis::Horizontal, ltr),
        Rect::from_origin_size(Offset::ZERO, Size::new(150., 100.))
    );
    // ...and right in RTL.
    assert_eq!(
        stretched(0.5, Axis::Horizontal, TextDirection::Rtl),
        Rect::from_origin_size(Offset::new(-50., 0.), Size::new(150., 100.))
    );
    // No strength, no transform.
    assert_eq!(
        stretched(0., Axis::Vertical, ltr),
        Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.))
    );
}

fn viewport(tree: &WidgetTree, from: ElementId) -> ElementId {
    let mut stack = vec![from];
    while let Some(id) = stack.pop() {
        if let Some(RenderKind::SliverViewport { .. }) = tree
            .render_id(id)
            .and_then(|render| tree.render_object_kind(render))
        {
            return id;
        }
        stack.extend(tree.children(id).unwrap_or_default().iter().copied());
    }
    panic!("no viewport");
}

#[test]
fn pulling_past_the_start_stretches_then_springs_back() {
    let mut tree = WidgetTree::new();
    let rows = (0..10).map(|_| Widget::box_(Size::new(100., 40.), Color::WHITE));
    let root = tree
        .mount(
            StretchingOverscrollIndicator::new(
                AxisDirection::Down,
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
    // Past the 18 px slop; `DragStartBehavior.start` applies later moves.
    send(&mut tree, PointerPhase::Move, 40., 100);
    // 50 px past the start of a 100 px viewport: pull(-0.5).
    send(&mut tree, PointerPhase::Move, 90., 200);
    layout(&mut tree);
    let distance = 0.5_f32;
    let strength =
        0.016 * distance + 0.016 * (1. - (-distance * (std::f32::consts::E / 0.33)).exp());
    let bounds = tree
        .element_bounds(viewport(&tree, root))
        .expect("viewport bounds");
    assert_eq!(bounds.origin, Offset::ZERO);
    assert!((bounds.size.height - 100. * (1. + strength)).abs() < 1e-3);
    // A slow release springs the content back.
    send(&mut tree, PointerPhase::Up, 90., 2_000);
    for millis in (2_000..4_000).step_by(16) {
        tree.update_compositor(base + Duration::from_millis(millis))
            .expect("frame");
        layout(&mut tree);
    }
    let bounds = tree
        .element_bounds(viewport(&tree, root))
        .expect("viewport bounds");
    assert_eq!(bounds.size, Size::new(100., 100.));
}
