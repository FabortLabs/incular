use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, MainAxisAlignment};
use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, FocusScope, FocusScopeNode, Row, SizedBox,
    TextDirection, Widget,
};

struct Scene {
    runtime: Runtime,
    nodes: Vec<FocusNode>,
    scope: FocusScopeNode,
    changed: Rc<RefCell<Vec<Option<bool>>>>,
}

impl Scene {
    fn new(row_sizes: &[usize], centered: bool) -> Self {
        let count = row_sizes.iter().sum();
        let nodes: Vec<_> = (0..count).map(|_| FocusNode::new()).collect();
        let changed = Rc::new(RefCell::new(vec![None; count]));
        let scope = FocusScopeNode::new();
        let mut next = 0;
        let rows: Vec<Widget> = row_sizes
            .iter()
            .map(|&n| {
                let row = Row::new((next..next + n).map(|index| {
                    let changed = changed.clone();
                    Focus::new(SizedBox::new().width(100.).height(100.))
                        .node(nodes[index].clone())
                        .on_focus_change(move |has_focus| {
                            changed.borrow_mut()[index] = Some(has_focus)
                        })
                }))
                .main_axis_alignment(if centered {
                    MainAxisAlignment::Center
                } else {
                    MainAxisAlignment::Start
                });
                next += n;
                row.into()
            })
            .collect();
        let runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                FocusScope::new(Column::new(rows))
                    .node(scope.clone())
                    .widget_order(),
            )
            .into(),
        )
        .unwrap();
        let mut scene = Self {
            runtime,
            nodes,
            scope,
            changed,
        };
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    }

    fn assert_transition(&self, previous: Option<usize>, current: usize) {
        assert!(self.scope.has_focus());
        let changes = self.changed.borrow();
        for (index, node) in self.nodes.iter().enumerate() {
            assert_eq!(node.has_focus(), index == current);
            assert_eq!(
                changes[index],
                if index == current {
                    Some(true)
                } else if previous == Some(index) {
                    Some(false)
                } else {
                    None
                },
                "focus callback for node {index}, previous {previous:?}, current {current}"
            );
        }
    }

    fn request(&mut self, previous: Option<usize>, target: usize) {
        self.changed.borrow_mut().fill(None);
        self.nodes[target].request_focus();
        self.frame();
        self.assert_transition(previous, target);
    }

    fn step(&mut self, code: Code, previous: usize, target: usize) {
        self.changed.borrow_mut().fill(None);
        let name = match code {
            Code::ArrowUp => NamedKey::ArrowUp,
            Code::ArrowDown => NamedKey::ArrowDown,
            Code::ArrowLeft => NamedKey::ArrowLeft,
            Code::ArrowRight => NamedKey::ArrowRight,
            _ => unreachable!(),
        };
        let mut key = KeyboardEvent::key_down(KeyboardKey::Named(name), code);
        assert!(self.runtime.dispatch_keyboard_event(key.clone()));
        key.state = KeyState::Up;
        self.runtime.dispatch_keyboard_event(key);
        self.frame();
        self.assert_transition(Some(previous), target);
    }
}

#[test]
fn focus_and_callbacks_move_around_all_four_corners() {
    let mut scene = Scene::new(&[2, 2], false);
    scene.request(None, 0);
    for (code, before, after) in [
        (Code::ArrowRight, 0, 1),
        (Code::ArrowDown, 1, 3),
        (Code::ArrowLeft, 3, 2),
        (Code::ArrowUp, 2, 0),
    ] {
        scene.step(code, before, after);
    }
}

#[test]
fn reversing_direction_returns_to_actual_predecessors_in_the_pyramid() {
    let mut scene = Scene::new(&[1, 2, 3], true);
    scene.request(None, 4);
    for (code, before, after) in [
        (Code::ArrowUp, 4, 2),
        (Code::ArrowUp, 2, 0),
        (Code::ArrowDown, 0, 2),
        (Code::ArrowDown, 2, 4),
        (Code::ArrowLeft, 4, 3),
        (Code::ArrowUp, 3, 1),
        (Code::ArrowDown, 1, 3),
        (Code::ArrowUp, 3, 1),
        (Code::ArrowUp, 1, 0),
        (Code::ArrowDown, 0, 1),
        (Code::ArrowDown, 1, 3),
    ] {
        scene.step(code, before, after);
    }
}

#[test]
fn an_explicit_focus_request_restarts_directional_history() {
    let mut scene = Scene::new(&[1, 1, 1, 1, 1], false);
    scene.request(None, 0);
    scene.step(Code::ArrowDown, 0, 1);
    scene.step(Code::ArrowDown, 1, 2);
    scene.step(Code::ArrowDown, 2, 3);
    scene.request(Some(3), 1);
    scene.step(Code::ArrowDown, 1, 2);
    scene.step(Code::ArrowUp, 2, 1);
    scene.step(Code::ArrowUp, 1, 0);
}

#[test]
fn initial_direction_chooses_the_entry_edge() {
    let scene = Scene::new(&[2, 2], false);
    assert!(scene.runtime.focused_element().is_none());
    for (code, expected) in [
        (Code::ArrowUp, 2),
        (Code::ArrowDown, 0),
        (Code::ArrowLeft, 1),
        (Code::ArrowRight, 0),
    ] {
        let tree = scene.runtime.tree();
        let target = tree.directional_focus_target(None, code).unwrap();
        assert_eq!(
            tree.focus_node_at(target).as_ref(),
            Some(&scene.nodes[expected])
        );
    }
}

#[test]
fn reversing_ignores_a_detached_or_replaced_predecessor() {
    use incular_widgets::{GlobalKey, Positioned, Stack};

    for replacement in [false, true] {
        let from = FocusNode::new();
        let aligned = FocusNode::new();
        let current = FocusNode::new();
        let new_node = FocusNode::new();
        let from_key = GlobalKey::new();
        let view = |updated: bool| {
            let focus = |node: &FocusNode| {
                Focus::new(SizedBox::new().width(100.).height(100.)).node(node.clone())
            };
            let from_widget = if !updated {
                Widget::from(focus(&from)).with_key(from_key)
            } else if replacement {
                Widget::from(focus(&new_node)).with_key(from_key)
            } else {
                SizedBox::new().width(100.).height(100.).into()
            };
            let scene = Stack::new([
                Widget::from(Positioned::new(from_widget).top(0.).left(0.)),
                Positioned::new(focus(&aligned)).top(0.).left(300.).into(),
                Positioned::new(focus(&current)).top(100.).left(300.).into(),
            ]);
            Widget::from(Directionality::new(
                TextDirection::Ltr,
                FocusScope::new(scene),
            ))
        };
        let mut runtime = Runtime::new(view(false)).unwrap();
        let window = Constraints::tight(Size::new(800., 600.));
        runtime.run_frame(window).unwrap();
        from.request_focus();
        runtime.run_frame(window).unwrap();
        assert!(from.has_primary_focus());
        assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
            KeyboardKey::Named(NamedKey::ArrowDown),
            Code::ArrowDown,
        )));
        assert!(current.has_primary_focus());
        let root = runtime.tree().root().unwrap();
        runtime.schedule_update(root, view(true)).unwrap();
        runtime.run_frame(window).unwrap();
        assert!(current.has_primary_focus());
        assert!(runtime.dispatch_keyboard_event(KeyboardEvent::key_down(
            KeyboardKey::Named(NamedKey::ArrowUp),
            Code::ArrowUp,
        )));
        assert!(aligned.has_primary_focus());
        assert!(!from.has_focus() && !new_node.has_focus());
    }
}
