//! Accessibility actions must notify outside TapRegions, independently of real pointer input.
use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_semantics::SemanticAction;
use incular_widgets::{
    Directionality, Row, Semantics, SizedBox, TapRegion, TapRegionSurface, Text, TextDirection,
    Widget,
};
use std::{cell::Cell, rc::Rc};

const OUTSIDE: &str = "Outside";
const INSIDE: &str = "Inside";
fn perform(action: SemanticAction, inside: bool, actionable: bool) -> (usize, usize) {
    let outside_calls = Rc::new(Cell::new(0));
    let action_calls = Rc::new(Cell::new(0));
    let label = if inside { INSIDE } else { OUTSIDE };
    let child_for = |name| -> Widget {
        let contents = SizedBox::new().width(100.).height(50.);
        if !actionable || name != label {
            return contents.child(Text::new(name)).into();
        }
        let calls = action_calls.clone();
        let callback = move || calls.set(calls.get() + 1);
        let target = Semantics::new(Widget::box_(
            Size::new(100., 50.),
            Color::rgba(0, 0, 255, 255),
        ))
        .label(name);
        match action {
            SemanticAction::Activate => target.on_tap(callback).into(),
            SemanticAction::LongPress => target.on_long_press(callback).into(),
            _ => unreachable!(),
        }
    };
    let observed = outside_calls.clone();
    let region =
        TapRegion::new(child_for(INSIDE)).on_tap_outside(move |_| observed.set(observed.get() + 1));
    let scene = Directionality::new(
        TextDirection::Ltr,
        TapRegionSurface::new(Row::new([child_for(OUTSIDE), region.into()])),
    );
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    let id = {
        let (id, node) = runtime
            .tree()
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some(label))
            .expect("semantic node by label");
        assert_eq!(node.actions.contains(&action.kind()), actionable);
        id
    };
    assert_eq!(outside_calls.get(), 0);
    let handled = runtime.dispatch_semantic_action(id, action);
    runtime.run_frame(bounds).unwrap();
    if actionable {
        assert!(handled, "the target action handler must run");
    }
    (outside_calls.get(), action_calls.get())
}

#[test]
fn semantic_tap_outside_notifies_a_tap_region() {
    assert_eq!(perform(SemanticAction::Activate, false, false), (1, 0));
}
#[test]
fn semantic_tap_inside_does_not_notify_the_outside_callback() {
    assert_eq!(perform(SemanticAction::Activate, true, false), (0, 0));
}
#[test]
fn semantic_long_press_outside_notifies_a_tap_region() {
    assert_eq!(perform(SemanticAction::LongPress, false, false), (1, 0));
}
#[test]
fn actionable_control_still_invokes_target_and_outside_region() {
    let actual = [SemanticAction::Activate, SemanticAction::LongPress]
        .map(|action| perform(action, false, true));
    assert_eq!(actual, [(1, 1), (1, 1)]);
}
