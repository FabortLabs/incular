use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, CrossAxisAlignment, EdgeInsets, MainAxisAlignment};
use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, Focus, FocusNode, FocusScope, Padding, Row, SizedBox, TextDirection,
    Widget,
};

struct Scene {
    runtime: Runtime,
    nodes: Vec<FocusNode>,
    changes: Rc<RefCell<Vec<Option<bool>>>>,
    current: Option<usize>,
}

impl Scene {
    fn new(
        count: usize,
        make_body: impl FnOnce(&dyn Fn(usize, f32, f32) -> Widget) -> Widget,
    ) -> Self {
        let nodes: Vec<_> = (0..count).map(|_| FocusNode::new()).collect();
        let changes = Rc::new(RefCell::new(vec![None; count]));
        let focus = |index: usize, width, height| {
            let changes = changes.clone();
            Focus::new(SizedBox::new().width(width).height(height))
                .node(nodes[index].clone())
                .on_focus_change(move |focused| changes.borrow_mut()[index] = Some(focused))
                .into()
        };
        let body = make_body(&focus);
        let mut scene = Self {
            runtime: Runtime::new(
                Directionality::new(TextDirection::Ltr, FocusScope::new(body).widget_order())
                    .into(),
            )
            .unwrap(),
            nodes,
            changes,
            current: None,
        };
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    }

    fn assert_focus(&mut self, new: usize) {
        let changes = self.changes.borrow();
        for (i, node) in self.nodes.iter().enumerate() {
            assert_eq!(
                node.has_focus(),
                i == new,
                "focus at node {i}, expected {new}"
            );
            let expected = if self.current == Some(new) {
                None
            } else if i == new {
                Some(true)
            } else if self.current == Some(i) {
                Some(false)
            } else {
                None
            };
            assert_eq!(changes[i], expected, "callback from node {i}");
        }
        self.current = Some(new);
    }

    fn request(&mut self, target: usize) {
        self.changes.borrow_mut().fill(None);
        self.nodes[target].request_focus();
        self.frame();
        self.assert_focus(target);
    }

    fn move_focus(&mut self, code: Code, target: usize) {
        self.changes.borrow_mut().fill(None);
        let key_name = match code {
            Code::ArrowDown => NamedKey::ArrowDown,
            Code::ArrowUp => NamedKey::ArrowUp,
            Code::ArrowLeft => NamedKey::ArrowLeft,
            Code::ArrowRight => NamedKey::ArrowRight,
            _ => unreachable!(),
        };
        if self.current == Some(target) {
            assert!(matches!(
                self.runtime
                    .tree()
                    .resolve_directional_focus(self.runtime.focused_element(), code),
                incular_widgets::internal::DirectionalFocusResolution::Unchanged,
            ));
        }
        let mut key = KeyboardEvent::key_down(KeyboardKey::Named(key_name), code);
        self.runtime.dispatch_keyboard_event(key.clone());
        key.state = KeyState::Up;
        self.runtime.dispatch_keyboard_event(key);
        self.frame();
        self.assert_focus(target);
    }
}

#[test]
fn irregular_grid_follows_the_right_band_at_every_turn() {
    let mut scene = Scene::new(9, |focus| {
        Column::new((0..3).map(|row| {
            Row::new((0..3).map(|column| {
                let id = row * 3 + column;
                focus(
                    id,
                    if id == 3 { 150. } else { 100. },
                    if id == 1 { 150. } else { 100. },
                )
            }))
            .main_axis_alignment(MainAxisAlignment::Center)
            .cross_axis_alignment(if row == 0 {
                CrossAxisAlignment::End
            } else {
                CrossAxisAlignment::Center
            })
        }))
        .into()
    });
    scene.request(1);
    for (direction, target) in [
        (Code::ArrowDown, 4),
        (Code::ArrowDown, 7),
        (Code::ArrowDown, 7),
        (Code::ArrowRight, 8),
        (Code::ArrowUp, 5),
        (Code::ArrowUp, 2),
        (Code::ArrowUp, 2),
        (Code::ArrowLeft, 1),
        (Code::ArrowLeft, 0),
        (Code::ArrowLeft, 0),
    ] {
        scene.move_focus(direction, target);
    }
}

#[test]
fn downward_traversal_takes_the_next_row_when_no_candidate_overlaps_the_band() {
    let mut scene = Scene::new(4, |focus| {
        Column::new((0..4).map(|row| {
            Padding::new(
                EdgeInsets::from_ltrb(0., 0., if row == 0 { 0. } else { 110. }, 0.),
                focus(row, if row == 1 { 150. } else { 100. }, 100.),
            )
        }))
        .cross_axis_alignment(CrossAxisAlignment::End)
        .into()
    });
    scene.request(0);
    for target in [1, 2, 3, 3] {
        scene.move_focus(Code::ArrowDown, target);
    }
}

#[test]
fn rightward_traversal_takes_the_next_column_when_no_candidate_overlaps_the_band() {
    let mut scene = Scene::new(4, |focus| {
        Row::new((0..4).map(|column| {
            Padding::new(
                EdgeInsets::from_ltrb(0., if column == 0 { 0. } else { 110. }, 0., 0.),
                focus(column, 100., if column == 1 { 150. } else { 100. }),
            )
        }))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .into()
    });
    scene.request(0);
    for target in [1, 2, 3, 3] {
        scene.move_focus(Code::ArrowRight, target);
    }
}
