//! Raw radio control on a mounted, typed, controlled group.
use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Code, Color, InputEvent, KeyboardEvent, KeyboardKey, NamedKey, Offset, PointerPhase, Size,
};
use incular_runtime::{Runtime, Signal};
use incular_semantics::{CheckedState, Role};
use incular_widgets::{Builder, Column, FocusNode, RadioGroup, RawRadio, Widget};

fn pump(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(200., 120.)))
        .unwrap();
}

fn checked(runtime: &Runtime) -> Vec<CheckedState> {
    let tree = runtime.tree().semantics();
    let root = tree.node(tree.root().expect("group semantics")).unwrap();
    assert_eq!(root.role, Role::RadioGroup);
    root.children
        .iter()
        .map(|id| {
            let child = tree.node(*id).unwrap();
            assert_eq!(child.role, Role::Radio);
            child
                .state
                .checked
                .expect("radio reports an explicit state")
        })
        .collect()
}

#[test]
fn controlled_radios_select_toggle_and_retain_focus_without_duplicate_callbacks() {
    let selected = Signal::new(Some(1_u32));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let focus = [FocusNode::new(), FocusNode::new(), FocusNode::new()];
    let radios: Vec<_> = focus
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let number = (index + 1) as u32;
            Widget::from(
                RawRadio::new(number, None)
                    .focus_node(node.clone())
                    .toggleable(number == 2)
                    .enabled(number != 3)
                    .builder(|state| {
                        Widget::box_(
                            Size::new(30., 24.),
                            if state.selected {
                                Color::BLACK
                            } else {
                                Color::WHITE
                            },
                        )
                    }),
            )
            .with_key(u64::from(number))
        })
        .collect();
    let group_content: Widget = Column::new(radios).into();
    let mut runtime = Runtime::new(
        Builder::new({
            let selected = selected.clone();
            let calls = calls.clone();
            move |_| {
                let change_value = selected.clone();
                let calls = calls.clone();
                RadioGroup::new(group_content.clone())
                    .value(selected.get())
                    .on_changed_optional(move |value| {
                        calls.borrow_mut().push(value);
                        change_value.set(value);
                    })
                    .into()
            }
        })
        .into(),
    )
    .unwrap();
    pump(&mut runtime);
    assert_eq!(
        checked(&runtime),
        [
            CheckedState::Checked,
            CheckedState::Unchecked,
            CheckedState::Unchecked
        ]
    );

    let tap = |runtime: &mut Runtime, number: u64| {
        let id = runtime.tree().element_with_key(&number.into()).unwrap();
        let bounds = runtime.tree().element_bounds(id).unwrap();
        let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = runtime.handle_input(InputEvent::Pointer { phase, position });
        }
        pump(runtime);
    };

    tap(&mut runtime, 1);
    assert!(calls.borrow().is_empty()); // Already checked, not toggleable.
    tap(&mut runtime, 2);
    assert_eq!(*calls.borrow(), [Some(2)]);
    assert!(focus[1].has_focus());
    assert_eq!(
        checked(&runtime),
        [
            CheckedState::Unchecked,
            CheckedState::Checked,
            CheckedState::Unchecked
        ]
    );

    let space = KeyboardEvent::key_down(KeyboardKey::Named(NamedKey::Unidentified), Code::Space);
    let _ = runtime.handle_input(InputEvent::Key(space));
    pump(&mut runtime);
    assert_eq!(*calls.borrow(), [Some(2), None]);
    assert!(focus[1].has_focus());
    assert_eq!(checked(&runtime), [CheckedState::Unchecked; 3]);

    tap(&mut runtime, 3);
    assert_eq!(*calls.borrow(), [Some(2), None]);
    assert!(!focus[2].has_focus());
}
