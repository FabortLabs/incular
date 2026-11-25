//! Accessibility actions must notify outside TapRegions, independently of real pointer input.
use incular_config::Constraints;
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_semantics::SemanticAction;
use incular_widgets::{
    Directionality, HitTestBehavior, Listener, Row, Semantics, SizedBox, TapRegion,
    TapRegionSurface, Text, TextDirection, Widget,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

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

#[test]
fn semantic_notifications_are_down_only_ordered_and_cannot_consume_the_target_action() {
    for action in [SemanticAction::Activate, SemanticAction::LongPress] {
        let trace = Rc::new(RefCell::new(Vec::new()));
        let point = Rc::new(Cell::new(None));
        let entry = |name| {
            let trace = Rc::clone(&trace);
            move || trace.borrow_mut().push(name)
        };
        let box_widget = || Widget::box_(Size::new(100., 50.), Color::WHITE);
        let target = Semantics::new(box_widget()).label("action target");
        let target = match action {
            SemanticAction::Activate => target.on_tap(entry("target")),
            SemanticAction::LongPress => target.on_long_press(entry("target")),
            _ => unreachable!(),
        };
        let a = TapRegion::new(box_widget())
            .consume_outside_taps(true)
            .on_tap_outside({
                let point = Rc::clone(&point);
                let mark = entry("A.outside");
                move |event| {
                    assert_eq!(event.phase, PointerPhase::Down);
                    point.set(Some(event.position));
                    mark();
                }
            })
            .on_tap_up_outside({
                let mark = entry("unexpected up");
                move |_| mark()
            });
        let b = TapRegion::new(target)
            .behavior(HitTestBehavior::Opaque)
            .on_tap_inside({
                let mark = entry("B.inside");
                move |_| mark()
            });
        let listener = Listener::new(TapRegionSurface::new(Row::new([
            Widget::from(a),
            Widget::from(b),
        ])))
        .on_pointer_down({
            let mark = entry("unexpected raw down");
            move |_| mark()
        });
        let scene = Directionality::new(TextDirection::Ltr, listener);
        let mut runtime = Runtime::new(scene.into()).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let (id, bounds) = runtime
            .tree()
            .semantics()
            .iter()
            .find_map(|(id, node)| {
                (node.label.as_deref() == Some("action target")).then_some((id, node.bounds))
            })
            .expect("real mounted target");
        assert!(runtime.dispatch_semantic_action(id, action));
        assert_eq!(&*trace.borrow(), &["A.outside", "B.inside", "target"]);
        assert_eq!(
            point.get(),
            Some(Offset::new(
                bounds.origin.x + bounds.size.width * 0.5,
                bounds.origin.y + bounds.size.height * 0.5,
            ))
        );
    }
}
