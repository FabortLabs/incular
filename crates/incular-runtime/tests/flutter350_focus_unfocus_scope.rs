use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Focus, FocusNode, FocusScope, FocusScopeNode, SizedBox, UnfocusDisposition,
};

struct Fixture {
    root: FocusScopeNode,
    scopes: [FocusScopeNode; 2],
    nodes: [FocusNode; 4],
    runtime: Runtime,
}

impl Fixture {
    fn new() -> Self {
        let root = FocusScopeNode::new();
        let scopes = [FocusScopeNode::new(), FocusScopeNode::new()];
        let nodes = std::array::from_fn(|_| FocusNode::new());
        let branch = |scope: &FocusScopeNode, first: usize, second: usize| {
            FocusScope::new(Focus::new(Column::new([
                Focus::new(SizedBox::shrink()).node(nodes[first].clone()),
                Focus::new(SizedBox::shrink()).node(nodes[second].clone()),
            ])))
            .node(scope.clone())
        };
        let scene = FocusScope::new(Column::new([
            branch(&scopes[0], 0, 1),
            branch(&scopes[1], 2, 3),
        ]))
        .node(root.clone());
        let mut fixture = Self {
            root,
            scopes,
            nodes,
            runtime: Runtime::new(scene.into()).unwrap(),
        };
        for child in [3, 1, 2, 0] {
            fixture.focus(child);
        }
        fixture.history(Some(0), Some(2));
        fixture.primary(0);
        fixture
    }

    fn pump(&mut self) {
        self.runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
    }

    fn focus(&mut self, child: usize) {
        self.nodes[child].request_focus();
        self.pump();
    }

    fn history(&self, left: Option<usize>, right: Option<usize>) {
        assert_eq!(
            self.scopes[0].focused_child(),
            left.map(|i| self.nodes[i].clone())
        );
        assert_eq!(
            self.scopes[1].focused_child(),
            right.map(|i| self.nodes[i].clone())
        );
    }

    fn primary(&self, chosen: usize) {
        for (index, node) in self.nodes.iter().enumerate() {
            assert_eq!(node.has_primary_focus(), index == chosen);
        }
        assert_eq!(self.scopes[0].has_focus(), chosen < 2);
        assert_eq!(self.scopes[1].has_focus(), chosen >= 2);
    }
}

#[test]
fn previous_child_unfocus_restores_history_and_crosses_to_the_preceding_scope() {
    let mut f = Fixture::new();
    for _ in 0..2 {
        f.nodes[0].unfocus_with_disposition(UnfocusDisposition::PreviouslyFocusedChild);
        assert!(
            f.nodes[0].has_primary_focus(),
            "focus settles at the next boundary"
        );
        f.pump();
        f.history(Some(1), Some(2));
        f.primary(1);
        f.focus(0);
        f.history(Some(0), Some(2));
        f.primary(0);
    }
    f.scopes[0].unfocus_with_disposition(UnfocusDisposition::PreviouslyFocusedChild);
    f.pump();
    f.history(Some(0), Some(2));
    f.primary(2);
}

#[test]
fn default_unfocus_parks_on_the_scope_and_only_clears_its_immediate_history() {
    let mut f = Fixture::new();
    for _ in 0..2 {
        f.nodes[0].unfocus();
        f.pump();
        f.history(None, Some(2));
        assert!(f.scopes[0].has_primary_focus());
        assert!(!f.scopes[1].has_focus());
        assert!(f.nodes.iter().all(|node| !node.has_primary_focus()));
        f.focus(0);
        f.history(Some(0), Some(2));
        f.primary(0);
    }
    f.scopes[0].unfocus();
    f.pump();
    f.history(Some(0), Some(2));
    assert!(f.root.has_primary_focus());
    assert!(f.scopes.iter().all(|scope| !scope.has_focus()));
    assert!(f.nodes.iter().all(|node| !node.has_primary_focus()));
}

#[test]
fn disabling_the_current_scope_moves_to_eligible_sibling_without_erasing_its_children() {
    let mut f = Fixture::new();
    f.scopes[0].set_can_request_focus(false);
    f.pump();
    f.history(Some(0), Some(2));
    f.primary(2);
    for disposition in [
        UnfocusDisposition::Scope,
        UnfocusDisposition::PreviouslyFocusedChild,
    ] {
        f.nodes[0].unfocus_with_disposition(disposition);
        f.pump();
        f.history(Some(0), Some(2));
        f.primary(2);
    }
}

#[test]
fn scope_restore_skips_disabled_children_and_uses_each_previous_eligible_child() {
    let mut f = Fixture::new();
    f.nodes[0].set_can_request_focus(false);
    f.nodes[2].set_can_request_focus(false);
    f.pump();

    assert!(f.scopes[0].request_focus_in_scope());
    f.pump();
    assert_eq!(f.scopes[0].focused_child(), Some(f.nodes[1].clone()));
    f.primary(1);
    assert!(f.scopes[1].request_focus_in_scope());
    f.pump();
    f.history(Some(1), Some(3));
    f.primary(3);
}
