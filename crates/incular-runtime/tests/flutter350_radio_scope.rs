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

fn radio_key(runtime: &mut Runtime, code: Code, shift: bool) {
    use incular_core::Modifiers;
    let key = match code {
        Code::Tab => KeyboardKey::Named(NamedKey::Tab),
        Code::ArrowDown => KeyboardKey::Named(NamedKey::ArrowDown),
        Code::ArrowUp => KeyboardKey::Named(NamedKey::ArrowUp),
        Code::ArrowLeft => KeyboardKey::Named(NamedKey::ArrowLeft),
        Code::ArrowRight => KeyboardKey::Named(NamedKey::ArrowRight),
        Code::Space => KeyboardKey::Character(" ".into()),
        _ => KeyboardKey::Named(NamedKey::Unidentified),
    };
    let mut event = KeyboardEvent::key_down(key, code);
    if shift {
        event.modifiers = Modifiers::SHIFT;
    }
    let _ = runtime.handle_input(InputEvent::Key(event));
    pump(runtime);
}

fn pick(runtime: &mut Runtime, key: u64) {
    let element = runtime.tree().element_with_key(&key.into()).unwrap();
    let bounds = runtime.tree().element_bounds(element).unwrap();
    let center = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: center,
        });
    }
    pump(runtime);
}

fn keyboard_scene(
    disabled: Option<u32>,
    surround: bool,
) -> (Runtime, Signal<Option<u32>>, [FocusNode; 3], [FocusNode; 2]) {
    use incular_widgets::{EditableText, TextDirection};
    let selected = Signal::new(None);
    let nodes = [FocusNode::new(), FocusNode::new(), FocusNode::new()];
    let fields = [FocusNode::new(), FocusNode::new()];
    let radio_items: Widget = Column::new(nodes.iter().enumerate().map(|(index, node)| {
        let value = index as u32;
        Widget::from(
            RawRadio::new(value, None)
                .enabled(disabled != Some(value))
                .focus_node(node.clone())
                .builder(|_| Widget::box_(Size::new(20., 16.), Color::WHITE)),
        )
        .with_key(1 + u64::from(value))
    }))
    .into();
    let field = |node: &FocusNode| -> Widget {
        EditableText::new(incular_text::TextEditingController::new())
            .text_direction(TextDirection::Ltr)
            .focus_node(node.clone())
            .into()
    };
    let field_widgets = surround.then(|| [field(&fields[0]), field(&fields[1])]);
    let mut runtime = Runtime::new(
        Builder::new({
            let selected = selected.clone();
            move |_| {
                let notify = selected.clone();
                let group: Widget = RadioGroup::new(radio_items.clone())
                    .value(selected.get())
                    .on_changed_optional(move |value| {
                        notify.set(value);
                    })
                    .into();
                if let Some([before, after]) = &field_widgets {
                    Column::new([before.clone(), group, after.clone()]).into()
                } else {
                    group
                }
            }
        })
        .into(),
    )
    .unwrap();
    pump(&mut runtime);
    (runtime, selected, nodes, fields)
}

#[test]
fn arrow_keys_wrap_in_reading_order_and_skip_disabled_radios() {
    for disabled in [None, Some(1)] {
        let (mut runtime, selected, focus, _) = keyboard_scene(disabled, false);
        pick(&mut runtime, 1);
        assert_eq!(selected.get(), Some(0));
        assert!(focus[0].has_focus());
        let expected = if disabled.is_none() {
            [1, 2, 0, 2, 1]
        } else {
            [2, 0, 2, 0, 2]
        };
        for (code, index) in [
            Code::ArrowDown,
            Code::ArrowRight,
            Code::ArrowRight,
            Code::ArrowLeft,
            Code::ArrowUp,
        ]
        .into_iter()
        .zip(expected)
        {
            radio_key(&mut runtime, code, false);
            assert_eq!(selected.get(), Some(index));
            assert!(focus[index as usize].has_focus());
        }
    }
}

#[test]
fn tab_visits_only_the_first_or_selected_radio_between_editable_neighbors() {
    let (mut runtime, selected, focus, fields) = keyboard_scene(None, true);
    fields[0].request_focus();
    pump(&mut runtime);
    assert!(fields[0].has_focus());
    radio_key(&mut runtime, Code::Tab, false);
    assert!(focus[0].has_focus());
    radio_key(&mut runtime, Code::Tab, false);
    assert!(!focus[0].has_focus() && !focus[1].has_focus());
    assert!(fields[1].has_focus());

    pick(&mut runtime, 2);
    assert_eq!(selected.get(), Some(1));
    fields[1].request_focus();
    pump(&mut runtime);
    assert!(fields[1].has_focus());
    radio_key(&mut runtime, Code::Tab, true);
    assert!(focus[1].has_focus());
    assert!(!focus[0].has_focus() && !fields[1].has_focus());
}

#[test]
fn radio_shortcuts_leave_the_parent_shortcuts_alone_when_an_editor_has_focus() {
    use incular_core::Modifiers;
    use incular_widgets::{CallbackShortcuts, EditableText, SingleActivator, TextDirection};

    let log = Rc::new(RefCell::new(Vec::new()));
    let selected = Signal::new(None::<u32>);
    let radio = FocusNode::new();
    let field = FocusNode::new();
    let children: [Widget; 4] = [
        RawRadio::new(0_u32, None).focus_node(radio.clone()).into(),
        RawRadio::new(1_u32, None).into(),
        RawRadio::new(2_u32, None).into(),
        EditableText::new(incular_text::TextEditingController::new())
            .text_direction(TextDirection::Ltr)
            .focus_node(field.clone())
            .into(),
    ];
    let child: Widget = Column::new(children).into();
    let group = Builder::new({
        let selected = selected.clone();
        move |_| {
            let notify = selected.clone();
            RadioGroup::new(child.clone())
                .value(selected.get())
                .on_changed_optional(move |value| {
                    notify.set(value);
                })
                .into()
        }
    });
    let mut command = CallbackShortcuts::new(group);
    for (binding, code) in [
        (KeyboardKey::Named(NamedKey::ArrowLeft), Code::ArrowLeft),
        (KeyboardKey::Named(NamedKey::ArrowRight), Code::ArrowRight),
        (KeyboardKey::Named(NamedKey::ArrowDown), Code::ArrowDown),
        (KeyboardKey::Named(NamedKey::ArrowUp), Code::ArrowUp),
        (KeyboardKey::Character(" ".into()), Code::Space),
    ] {
        let log = log.clone();
        command = command.shortcut(
            SingleActivator::new(binding, Modifiers::empty()),
            move || log.borrow_mut().push(code),
        );
    }
    let mut runtime =
        Runtime::new(incular_widgets::Directionality::new(TextDirection::Ltr, command).into())
            .unwrap();
    pump(&mut runtime);

    radio.request_focus();
    pump(&mut runtime);
    assert_eq!(
        runtime
            .focused_element()
            .and_then(|id| runtime.tree().focus_node_at(id)),
        Some(radio.clone())
    );
    assert!(radio.has_focus());
    for (code, value) in [
        (Code::Space, 0),
        (Code::ArrowRight, 1),
        (Code::ArrowLeft, 0),
    ] {
        radio_key(&mut runtime, code, false);
        assert_eq!(
            selected.get(),
            Some(value),
            "event {code:?}, parent recorded {:?}",
            log.borrow()
        );
        assert!(log.borrow().is_empty());
    }

    field.request_focus();
    pump(&mut runtime);
    assert!(field.has_focus());
    let expected = [
        Code::ArrowLeft,
        Code::ArrowRight,
        Code::ArrowDown,
        Code::ArrowUp,
        Code::Space,
    ];
    for (index, key) in expected.into_iter().enumerate() {
        radio_key(&mut runtime, key, false);
        assert_eq!(selected.get(), Some(0));
        assert_eq!(&*log.borrow(), &expected[..=index]);
    }

    log.borrow_mut().clear();
    radio.request_focus();
    pump(&mut runtime);
    assert!(radio.has_focus());
    for (code, value) in [(Code::ArrowDown, 1), (Code::ArrowUp, 0)] {
        radio_key(&mut runtime, code, false);
        assert_eq!(selected.get(), Some(value));
        assert!(log.borrow().is_empty());
    }
}

#[test]
fn a_zero_area_group_mounts_raw_radios_with_independent_registries() {
    use incular_widgets::{
        MouseCursor, RadioGroupRegistry, Text, TextDirection, internal::WidgetTree,
    };
    let radio = |value, label| {
        RawRadio::new(value, None)
            .group_registry(RadioGroupRegistry::new(None, |_| {}))
            .focus_node(FocusNode::new())
            .mouse_cursor(MouseCursor::Click)
            .builder(move |_| Text::new(label).text_direction(TextDirection::Ltr).into())
    };
    let group = RadioGroup::<u32>::new(Column::new([radio(1, "X"), radio(2, "Y")]));
    let mut tree = WidgetTree::new();
    tree.mount(group.into()).unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(tree.root_layout_size(), Some(Size::ZERO));
}

fn radio_fixture(values: &[(u32, bool)]) -> (Runtime, Signal<Option<u32>>) {
    let selected = Signal::new(None::<u32>);
    let controls: Widget =
        Column::new(values.iter().enumerate().map(|(index, &(value, enabled))| {
            Widget::from(
                RawRadio::new(value, None)
                    .enabled(enabled)
                    .builder(|_| Widget::box_(Size::new(25., 16.), Color::WHITE)),
            )
            .with_key(1 + index as u64)
        }))
        .into();
    let mut runtime = Runtime::new(
        Builder::new({
            let selected = selected.clone();
            move |_| {
                let notify = selected.clone();
                RadioGroup::<u32>::new(controls.clone())
                    .value(selected.get())
                    .on_changed_optional(move |value| {
                        notify.set(value);
                    })
                    .into()
            }
        })
        .into(),
    )
    .unwrap();
    pump(&mut runtime);
    (runtime, selected)
}

#[test]
fn radio_group_control_updates_exclusive_semantics_and_a_disabled_tap_does_nothing() {
    use incular_semantics::SemanticAnnotationFlag;
    let inspect = |runtime: &Runtime| {
        let semantics = runtime.tree().semantics();
        let group = semantics.node(semantics.root().unwrap()).unwrap();
        group
            .children
            .iter()
            .map(|&id| {
                let child = semantics.node(id).unwrap();
                assert_eq!(
                    child
                        .state
                        .annotation_flags
                        .get(SemanticAnnotationFlag::MutuallyExclusiveGroup),
                    Some(true)
                );
                (child.state.checked, child.state.enabled_state())
            })
            .collect::<Vec<_>>()
    };
    use CheckedState::{Checked, Unchecked};
    let (mut runtime, selected) = radio_fixture(&[(0, true), (1, true)]);
    assert_eq!(inspect(&runtime), [(Some(Unchecked), Some(true)); 2]);
    pick(&mut runtime, 1);
    assert_eq!(selected.get(), Some(0));
    assert_eq!(
        inspect(&runtime),
        [(Some(Checked), Some(true)), (Some(Unchecked), Some(true))]
    );
    pick(&mut runtime, 2);
    assert_eq!(selected.get(), Some(1));
    assert_eq!(
        inspect(&runtime),
        [(Some(Unchecked), Some(true)), (Some(Checked), Some(true))]
    );

    let (mut runtime, selected) = radio_fixture(&[(0, false), (1, true)]);
    let expected = [
        (Some(Unchecked), Some(false)),
        (Some(Unchecked), Some(true)),
    ];
    assert_eq!(inspect(&runtime), expected);
    pick(&mut runtime, 1);
    assert_eq!(selected.get(), None);
    assert_eq!(inspect(&runtime), expected);
}

#[test]
fn a_radio_group_is_a_semantic_container_between_independent_checkboxes() {
    use incular_widgets::{Semantics, SizedBox, internal::WidgetTree};
    let checkbox = || {
        Widget::from(
            Semantics::new(SizedBox::square(12.))
                .role(Role::Checkbox)
                .checked(true),
        )
    };
    let raw = |value| {
        RawRadio::new(value, None).builder(|_| Widget::box_(Size::new(12., 12.), Color::WHITE))
    };
    let scene = Semantics::new(Column::new([
        checkbox(),
        RadioGroup::<u32>::new(Column::new([raw(0), raw(1)])).into(),
        checkbox(),
    ]))
    .container(true);
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(200., 120.)))
        .unwrap();
    tree.update_semantics();
    let semantics = tree.semantics();
    let parent = semantics.node(semantics.root().unwrap()).unwrap();
    let group = parent
        .children
        .iter()
        .find_map(|id| {
            semantics
                .node(*id)
                .filter(|node| node.role == Role::RadioGroup)
        })
        .expect("radio group remains a child container of the outer semantics");
    assert_eq!(group.children.len(), 2);
    assert!(
        group
            .children
            .iter()
            .all(|id| semantics.node(*id).unwrap().role == Role::Radio)
    );
}

#[test]
fn repeated_values_only_raise_a_group_diagnostic_when_the_repeated_value_is_selected() {
    use incular_semantics::SemanticRoleIssueKind;
    let (mut runtime, selected) = radio_fixture(&[(0, true), (1, true), (1, true), (2, true)]);
    assert!(runtime.tree().debug_semantic_role_issues().is_empty());
    pick(&mut runtime, 2);
    assert_eq!(selected.get(), Some(1));
    let [issue] = runtime.tree().debug_semantic_role_issues() else {
        panic!("expected one duplicate selection diagnostic");
    };
    assert_eq!(issue.role, Role::RadioGroup);
    assert_eq!(
        issue.kind,
        SemanticRoleIssueKind::RadioGroupWithMultipleCheckedChildren
    );
}

#[test]
fn removing_a_radio_preserves_other_checked_nodes_and_drops_its_arrow_registration() {
    let skip = Signal::new(0_usize);
    let selected = Signal::new(Some(4_u32));
    let nodes: Vec<_> = (0..5).map(|_| FocusNode::new()).collect();
    let controls: Vec<_> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            Widget::from(
                RawRadio::new(i as u32, None)
                    .focus_node(node.clone())
                    .builder(|_| Widget::box_(Size::new(25., 16.), Color::WHITE)),
            )
            .with_key(50 + i as u64)
        })
        .collect();
    let mut runtime = Runtime::new(
        Builder::new({
            let skip = skip.clone();
            let selected = selected.clone();
            move |_| {
                let notify = selected.clone();
                RadioGroup::<u32>::new(Column::new(controls.iter().skip(skip.get()).cloned()))
                    .value(selected.get())
                    .on_changed_optional(move |value| {
                        notify.set(value);
                    })
                    .into()
            }
        })
        .into(),
    )
    .unwrap();
    pump(&mut runtime);
    assert_eq!(checked(&runtime).len(), 5);
    assert!(runtime.tree().debug_semantic_role_issues().is_empty());

    skip.set(1);
    pump(&mut runtime);
    assert_eq!(
        checked(&runtime),
        [
            CheckedState::Unchecked,
            CheckedState::Unchecked,
            CheckedState::Unchecked,
            CheckedState::Checked
        ]
    );
    assert!(runtime.tree().debug_semantic_role_issues().is_empty());
    nodes[4].request_focus();
    pump(&mut runtime);
    radio_key(&mut runtime, Code::ArrowDown, false);
    assert_eq!(selected.get(), Some(1));
    assert!(nodes[1].has_focus());
}
