//! Focus traversal counterparts from Flutter 3.50.0.
//! The originals invoke FocusNode.nextFocus/previousFocus. Incular drives
//! the equivalent traversal intent through normalized Tab/Shift+Tab.

use incular_config::Constraints;
use incular_core::{Code, InputEvent, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, ExcludeFocusTraversal, Focus, FocusNode, FocusScope,
    FocusTraversalGroup, FocusTraversalOrder, Row, SizedBox, TextDirection, Widget,
};
use std::time::{Duration, Instant};

struct Host {
    runtime: Runtime,
    now: Instant,
}

impl Host {
    fn mount(widget: Widget) -> Self {
        let mut host = Self {
            runtime: Runtime::new(Directionality::new(TextDirection::Ltr, widget).into())
                .expect("mount focus traversal scene"),
            now: Instant::now(),
        };
        host.frame();
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.now)
            .expect("run focus traversal frame");
        self.now += Duration::from_millis(16);
    }

    fn tab(&mut self, reverse: bool) {
        let tab = KeyboardKey::Named(NamedKey::Tab);
        for mut event in [
            KeyboardEvent::key_down(tab.clone(), Code::Tab),
            KeyboardEvent::key_up(tab, Code::Tab),
        ] {
            if reverse {
                event.modifiers = Modifiers::SHIFT;
            }
            let _ = self
                .runtime
                .handle_input_at(InputEvent::Key(event), self.now);
        }
        self.frame();
    }
}

fn primary(nodes: &[FocusNode]) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.has_primary_focus().then_some(index))
        .collect()
}

fn ten_nodes() -> [FocusNode; 10] {
    std::array::from_fn(|_| FocusNode::new())
}

fn focus_target(node: &FocusNode) -> Focus {
    Focus::new(SizedBox::square(10.)).node(node.clone())
}

fn check_order(nodes: &[FocusNode; 10], children: Vec<Widget>) {
    let row = FocusScope::new(Row::new(children));
    let mut host = Host::mount(FocusTraversalGroup::new(row).ordered().into());
    assert!(host.runtime.focused_element().is_none());
    assert!(primary(nodes).is_empty());

    let mut forward = Vec::new();
    for _ in 0..10 {
        host.tab(false);
        forward.push(primary(nodes));
    }
    let mut backward = Vec::new();
    for _ in 1..10 {
        host.tab(true);
        backward.push(primary(nodes));
    }

    let expected_forward: Vec<_> = (0..10).rev().map(|i| vec![i]).collect();
    let expected_backward: Vec<_> = (1..10).map(|i| vec![i]).collect();
    assert_eq!((forward, backward), (expected_forward, expected_backward));
}

/// focus_traversal_test.dart:1279, ordered numeric priorities 10 through 1.
#[test]
fn tab_and_shift_tab_follow_numeric_focus_order() {
    let nodes = ten_nodes();
    let children = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            FocusTraversalOrder::new(focus_target(node))
                .order(10. - i as f64)
                .into()
        })
        .collect();
    check_order(&nodes, children);
}

/// focus_traversal_test.dart:1331, ordered lexical priorities J through A.
#[test]
fn tab_and_shift_tab_follow_lexical_focus_order() {
    let nodes = ten_nodes();
    let children = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            FocusTraversalOrder::new(focus_target(node))
                .lexical(char::from(b'J' - i as u8).to_string())
                .into()
        })
        .collect();
    check_order(&nodes, children);
}

fn mount_with_excluded_focus() -> (Host, [FocusNode; 4]) {
    let nodes: [FocusNode; 4] = std::array::from_fn(|_| FocusNode::new());
    let excluded = Widget::from(focus_target(&nodes[2])).with_key("excluded focus");
    let nested = Focus::new(excluded).node(nodes[1].clone());
    let column = Column::new(vec![
        Widget::from(focus_target(&nodes[0]).autofocus(true)),
        Widget::from(ExcludeFocusTraversal::new(nested)),
        Widget::from(focus_target(&nodes[3])).with_key("end focus"),
    ]);
    let mut host = Host::mount(FocusTraversalGroup::new(column).into());
    host.frame();
    (host, nodes)
}

/// focus_traversal_test.dart:3691. Both the excluded Focus and its nested
/// Focus are skipped by traversal.
#[test]
fn tab_skips_excluded_focus_and_its_descendant() {
    let (mut host, nodes) = mount_with_excluded_focus();
    assert_eq!(primary(&nodes), [0]);

    host.tab(false);
    assert_eq!(primary(&nodes), [3]);
}

/// Additional Flutter reference: traversal exclusion is not focus exclusion.
/// Verify the actual retained keyboard target as well as public node flags.
#[test]
fn explicit_request_focus_enters_traversal_excluded_subtree() {
    let (mut host, nodes) = mount_with_excluded_focus();
    let tree = host.runtime.tree();
    let end = tree
        .element_with_key(&"end focus".into())
        .expect("end target");
    let excluded = tree
        .element_with_key(&"excluded focus".into())
        .expect("excluded target");
    assert_ne!(end, excluded);
    assert_eq!(primary(&nodes), [0]);

    host.tab(false);
    assert_eq!(
        (host.runtime.focused_element(), primary(&nodes)),
        (Some(end), vec![3])
    );

    nodes[2].request_focus();
    host.frame();
    assert_eq!(
        (host.runtime.focused_element(), primary(&nodes)),
        (Some(excluded), vec![2]),
        "an explicit request must bypass traversal exclusion"
    );
    assert!(nodes[1].has_focus());
    assert!(!nodes[1].has_primary_focus());

    host.tab(false);
    assert_eq!(
        (host.runtime.focused_element(), primary(&nodes)),
        (Some(end), vec![3]),
        "forward traversal starts at the excluded node's position"
    );
    nodes[2].request_focus();
    host.frame();
    host.tab(true);
    assert_eq!(
        primary(&nodes),
        [0],
        "reverse traversal starts at the excluded node's position"
    );
}

#[test]
fn skip_traversal_still_allows_a_direct_node_request() {
    let nodes: [FocusNode; 3] = std::array::from_fn(|_| FocusNode::new());
    nodes[1].set_skip_traversal(true);
    let column = Column::new(vec![
        Widget::from(focus_target(&nodes[0]).autofocus(true)),
        Widget::from(focus_target(&nodes[1])),
        Widget::from(focus_target(&nodes[2])),
    ]);
    let mut host = Host::mount(FocusTraversalGroup::new(column).into());
    assert_eq!(primary(&nodes), [0]);
    host.tab(false);
    assert_eq!(primary(&nodes), [2]);
    nodes[1].request_focus();
    host.frame();
    assert_eq!(primary(&nodes), [1]);
    host.tab(true);
    assert_eq!(primary(&nodes), [0]);
}
