use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Column, Focus, FocusNode, FocusScope, FocusScopeNode, SizedBox, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(320., 220.)))
        .unwrap();
}

fn leaf(node: &FocusNode, autofocus: bool) -> Focus {
    Focus::new(SizedBox::square(20.))
        .node(node.clone())
        .autofocus(autofocus)
}

#[test]
fn choosing_a_first_scope_restores_its_last_child_or_parks_on_the_scope() {
    let [root, parent, first, second] = std::array::from_fn(|_| FocusScopeNode::new());
    let [a, b, c] = std::array::from_fn(|_| FocusNode::new());
    let scopes = Column::new([
        FocusScope::new(Column::new([leaf(&a, true), leaf(&b, false)])).node(first.clone()),
        FocusScope::new(leaf(&c, false)).node(second.clone()),
    ]);
    let scene = FocusScope::new(FocusScope::new(scopes).node(parent.clone())).node(root.clone());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    assert!(a.has_primary_focus());

    assert!(parent.set_first_focus(&second));
    frame(&mut runtime);
    assert!(!a.has_focus());
    assert!(!c.has_focus());
    assert!(second.has_primary_focus());

    assert!(parent.set_first_focus(&first));
    frame(&mut runtime);
    assert!(a.has_primary_focus());

    b.request_focus();
    frame(&mut runtime);
    assert!(b.has_primary_focus());
    assert!(parent.is_first_focus());
    assert!(first.is_first_focus());

    assert!(parent.set_first_focus(&second));
    frame(&mut runtime);
    assert!(!b.has_focus());
    assert!(parent.is_first_focus());
    assert!(!first.is_first_focus());
    assert!(second.is_first_focus());

    c.request_focus();
    frame(&mut runtime);
    assert!(c.has_primary_focus());
    assert!(parent.is_first_focus());
    assert!(!first.is_first_focus());
    assert!(second.is_first_focus());

    assert!(first.request_focus_in_scope());
    frame(&mut runtime);
    assert!(b.has_primary_focus());
    assert!(!c.has_focus());
    assert!(parent.is_first_focus());
    assert!(first.is_first_focus());
    assert!(!second.is_first_focus());
}

#[test]
fn selecting_an_inactive_scope_updates_history_without_stealing_focus() {
    let parent = FocusScopeNode::new();
    let first = FocusScopeNode::new();
    let second = FocusScopeNode::new();
    let other = FocusNode::new();
    let tree = Column::new([
        Widget::from(leaf(&other, true)),
        Widget::from(
            FocusScope::new(Column::new([
                FocusScope::new(SizedBox::square(20.)).node(first.clone()),
                FocusScope::new(SizedBox::square(20.)).node(second.clone()),
            ]))
            .node(parent.clone()),
        ),
    ]);
    let mut runtime = Runtime::new(tree.into()).unwrap();
    frame(&mut runtime);
    assert!(other.has_primary_focus());
    assert!(parent.set_first_focus(&first));
    assert!(first.is_first_focus());
    assert!(parent.set_first_focus(&second));
    assert!(!first.is_first_focus());
    assert!(second.is_first_focus());

    assert!(!first.set_first_focus(&second));
    assert!(!parent.set_first_focus(&parent));
    assert!(!parent.set_first_focus(&FocusScopeNode::new()));
    frame(&mut runtime);
    assert!(other.has_primary_focus());
    assert!(!second.has_focus());

    assert!(parent.request_focus_in_scope());
    frame(&mut runtime);
    assert!(second.has_primary_focus());
    assert!(!other.has_focus());
}
