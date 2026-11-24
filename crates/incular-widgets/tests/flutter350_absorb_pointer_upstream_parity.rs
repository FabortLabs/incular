//! Observables from Flutter 3.50.0 widgets/absorb_pointer_test.dart.

use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_gestures::PointerEvent;
use incular_semantics::{SemanticActionKind, SemanticRole};
use incular_widgets::{
    AbsorbPointer, Column, Expanded, GestureDetector, HitTestBehavior, SizedBox, Widget,
    internal::{ActionSurface, WidgetTree},
};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

fn mounted(widget: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
    tree
}

#[test]
fn absorber_does_not_block_a_disjoint_sibling() {
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let tap = GestureDetector::new(SizedBox::shrink())
        .behavior(HitTestBehavior::Translucent)
        .on_tap(move || observed.set(observed.get() + 1));
    let absorber = AbsorbPointer::new(SizedBox::shrink());
    let mut tree = mounted(
        Column::new([
            Widget::from(Expanded::new(tap)),
            Widget::from(Expanded::new(absorber)),
        ])
        .into(),
    );
    let start = Instant::now();
    for (contact, y) in [(1, 150.), (2, 450.)] {
        for (phase, delay) in [(PointerPhase::Down, 0), (PointerPhase::Up, 10)] {
            tree.dispatch_gesture(PointerEvent {
                pointer: contact,
                position: Offset::new(400., y),
                phase,
                time: start + Duration::from_millis(contact * 100 + delay),
            });
        }
    }
    assert_eq!(calls.get(), 1);
}

fn verify_absorber_semantics(absorbing: bool) {
    // Flutter's test-local TestButton has explicit enabled/focusable
    // semantics and Tap/Focus actions. This is the neutral retained
    // action surface, not a Material or Cupertino control.
    let control = ActionSurface::with_child(Widget::box_(Size::new(60., 30.), Color::WHITE))
        .label("button")
        .on_press(|| {});
    let mut tree = mounted(AbsorbPointer::new(control).absorbing(absorbing).into());
    tree.update_semantics();
    let button = tree
        .semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some("button"))
        .expect("absorber must preserve the labeled button node");
    assert_eq!(button.role, SemanticRole::Button);
    assert!(button.state.enabled, "absorber must preserve enabled state");
    assert!(
        button.state.focusable,
        "absorber must preserve focusable state"
    );
    let snapshot = (
        button.actions.contains(&SemanticActionKind::Activate),
        button.actions.contains(&SemanticActionKind::Focus),
    );
    assert_eq!(
        snapshot,
        (!absorbing, !absorbing),
        "Flutter keeps flags, but removes both the tap and focus action when absorbing"
    );
}

#[test]
fn disabled_absorber_keeps_both_accessibility_actions() {
    verify_absorber_semantics(false);
}

#[test]
fn active_absorber_removes_tap_and_focus_actions_without_hiding_control() {
    verify_absorber_semantics(true);
}
