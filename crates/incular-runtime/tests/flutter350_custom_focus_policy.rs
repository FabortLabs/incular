use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, CustomFocusTraversalPolicy, Focus, FocusNode, FocusScope, FocusTraversalGroup,
    GlobalKey, SizedBox, Widget,
};

fn first_current_last() -> CustomFocusTraversalPolicy {
    CustomFocusTraversalPolicy::new(|candidates, current| {
        let Some(last) = candidates.len().checked_sub(1) else {
            return Vec::new();
        };
        let mut order = vec![0];
        if let Some(current) = current
            && let Some(index) = candidates
                .iter()
                .position(|item| item.node.as_ref() == Some(current))
            && index != 0
            && index != last
        {
            order.push(index);
        }
        if last != 0 {
            order.push(last);
        }
        order
    })
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn traversal_can_skip_every_member_except_the_first_current_and_last() {
    let nodes: [FocusNode; 7] = std::array::from_fn(|_| FocusNode::new());
    let scene = FocusTraversalGroup::new(Column::new(
        nodes
            .iter()
            .map(|node| Focus::new(SizedBox::square(20.)).node(node.clone())),
    ))
    .custom_policy(first_current_last());
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.tree().focusable_elements().len(), 7);
    nodes[2].request_focus();
    frame(&mut runtime);
    assert!(nodes[2].has_primary_focus());

    assert!(runtime.next_focus_from(&nodes[2]));
    frame(&mut runtime);
    assert!(nodes[6].has_primary_focus());
    assert!(runtime.previous_focus_from(&nodes[6]));
    frame(&mut runtime);
    assert!(nodes[0].has_primary_focus());
}

#[test]
fn custom_policy_lookup_follows_the_mounted_focus_parent_not_the_visual_one() {
    let child = FocusNode::new();
    let parent = FocusNode::new();
    let child_key = GlobalKey::new();
    let parent_key = GlobalKey::new();
    let policy = first_current_last();

    let visual = FocusTraversalGroup::new(Focus::new(
        Focus::with_external_focus_node(
            Widget::from(SizedBox::shrink()).with_key(child_key),
            child.clone(),
        )
        .parent_node(parent.clone()),
    ))
    .widget_order();
    let logical = FocusTraversalGroup::new(FocusScope::new(Focus::with_external_focus_node(
        Widget::from(SizedBox::shrink()).with_key(parent_key),
        parent.clone(),
    )))
    .custom_policy(policy.clone());
    let scene = Column::new([Widget::from(visual), Widget::from(logical)]);
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    assert_eq!(child.parent(), Some(parent.clone()));
    let child_element = runtime.tree().element_with_key(&child_key.into()).unwrap();
    let parent_element = runtime.tree().element_with_key(&parent_key.into()).unwrap();
    assert_eq!(
        FocusTraversalGroup::maybe_custom_policy_of(runtime.tree(), child_element),
        Some(policy.clone())
    );
    assert_eq!(
        FocusTraversalGroup::maybe_custom_policy_of(runtime.tree(), parent_element),
        Some(policy)
    );
}

#[test]
fn malformed_custom_indices_cannot_duplicate_or_invent_a_focus_stop() {
    let [first, last] = std::array::from_fn(|_| FocusNode::new());
    let policy = CustomFocusTraversalPolicy::new(|_, _| vec![usize::MAX, 1, 1, 0, 0]);
    let scene = FocusTraversalGroup::new(Column::new(
        [first.clone(), last.clone()].map(|node| Focus::new(SizedBox::square(20.)).node(node)),
    ))
    .custom_policy(policy);
    let mut runtime = Runtime::new(scene.into()).unwrap();
    frame(&mut runtime);
    assert!(runtime.next_focus_from(&first));
    frame(&mut runtime);
    assert!(last.has_primary_focus());
    assert!(runtime.next_focus_from(&last));
    frame(&mut runtime);
    assert!(first.has_primary_focus());
}
