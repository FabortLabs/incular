use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Focus, FocusNode, FocusScope, FocusScopeNode, FocusTraversalGroup, SizedBox, Widget,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn mounted_group_calls_creation_once_with_its_real_nontraversable_focus_node() {
    let child = FocusNode::new();
    let created = Rc::new(RefCell::new(Vec::<FocusNode>::new()));
    let scene = || {
        FocusTraversalGroup::new(Focus::new(SizedBox::shrink()).node(child.clone()))
            .on_focus_node_created({
                let created = created.clone();
                move |node| created.borrow_mut().push(node)
            })
    };
    let mut runtime = Runtime::new(scene().into()).unwrap();
    frame(&mut runtime);
    let [group]: [FocusNode; 1] = created.take().try_into().expect("one group created");
    assert_eq!(child.parent(), Some(group.clone()));
    assert!(group.skip_traversal());
    assert!(!group.can_request_focus());
    assert!(child.can_request_focus());

    let root = runtime.tree().root().unwrap();
    runtime
        .tree_mut()
        .update(root, scene().widget_order().into())
        .unwrap();
    frame(&mut runtime);
    assert!(created.borrow().is_empty());
    assert_eq!(child.parent(), Some(group.clone()));
    group.request_focus();
    frame(&mut runtime);
    assert!(!group.has_primary_focus());
    child.request_focus();
    frame(&mut runtime);
    assert!(child.has_primary_focus());
    assert!(group.has_focus());
}

#[test]
fn group_explicit_parent_moves_its_focus_descendants_out_of_the_visual_scope() {
    let root_scope = FocusScopeNode::new();
    let enclosing_scope = FocusScopeNode::new();
    let child = FocusNode::new();
    let widget: Widget = FocusScope::with_external_focus_node(
        FocusScope::with_external_focus_node(
            FocusScope::new(
                FocusTraversalGroup::new(Focus::new(SizedBox::shrink()).node(child.clone()))
                    .parent_node(root_scope.scope_focus_node()),
            ),
            enclosing_scope.clone(),
        ),
        root_scope.clone(),
    )
    .into();
    let mut runtime = Runtime::new(widget).unwrap();
    frame(&mut runtime);
    let group = child.parent().expect("group node");
    assert_eq!(group.parent(), Some(root_scope.scope_focus_node()));
    assert!(
        !enclosing_scope
            .scope_focus_node()
            .descendants()
            .contains(&child)
    );
    assert!(root_scope.scope_focus_node().descendants().contains(&child));
    child.request_focus();
    frame(&mut runtime);
    assert!(child.has_primary_focus());
    assert!(!enclosing_scope.has_focus());
}
