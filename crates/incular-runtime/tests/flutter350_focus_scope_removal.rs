use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Focus, FocusNode, FocusScope, FocusScopeNode, GlobalKey, SizedBox, Widget,
};

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

fn removal(pinned: bool) {
    let [root, scope_a, scope_b] = std::array::from_fn(|_| FocusScopeNode::new());
    let [a, b] = std::array::from_fn(|_| FocusNode::new());
    let [key_a, key_b, scope_key_a, scope_key_b] = std::array::from_fn(|_| GlobalKey::new());
    let content = |include_a, autofocus_b| {
        let make = |node: &FocusNode, scope: &FocusScopeNode, key, scope_key, autofocus| {
            let child = Widget::from(leaf(node, autofocus)).with_key(key);
            let widget = Widget::from(FocusScope::new(child).node(scope.clone()));
            if pinned {
                widget.with_key(scope_key)
            } else {
                widget
            }
        };
        let mut children = Vec::new();
        if include_a {
            children.push(make(&a, &scope_a, key_a, scope_key_a, false));
        }
        children.push(make(&b, &scope_b, key_b, scope_key_b, autofocus_b));
        FocusScope::new(Column::new(children)).node(root.clone())
    };
    let mut runtime = Runtime::new(content(true, false).into()).unwrap();
    frame(&mut runtime);
    assert!(scope_b.request_focus(&b));
    frame(&mut runtime);
    assert!(scope_a.request_focus(&a));
    frame(&mut runtime);
    assert!(root.set_first_focus(&scope_b));
    assert!(root.set_first_focus(&scope_a));
    frame(&mut runtime);
    assert!(scope_a.is_first_focus());
    assert!(a.has_primary_focus());
    assert!(!b.has_focus());

    let root_id = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root_id, content(false, true).into())
        .unwrap();
    frame(&mut runtime);
    assert!(!a.has_focus());
    assert!(b.has_primary_focus());
    assert!(scope_b.is_first_focus());
}

#[test]
fn surviving_scope_receives_focus_when_the_globally_keyed_active_scope_is_removed() {
    removal(true);
}

#[test]
fn surviving_scope_receives_focus_when_the_unkeyed_active_scope_is_removed() {
    removal(false);
}

#[test]
fn a_focused_keyed_editor_moved_into_layout_keeps_selection_and_composing_range() {
    use incular_config::TextDirection;
    use incular_text::{ComposingRange, TextEditingController, TextSelection};
    use incular_widgets::{EditableText, LayoutBuilder};

    let key = GlobalKey::new();
    let node = FocusNode::new();
    let controller = TextEditingController::with_text("abcd");
    let editor = Widget::from(
        EditableText::new(controller.clone())
            .focus_node(node.clone())
            .text_direction(TextDirection::Ltr),
    )
    .with_key(key);
    let mut runtime = Runtime::new(Column::new([editor.clone()]).into()).unwrap();
    node.request_focus();
    frame(&mut runtime);
    let original = runtime.focused_element().unwrap();
    controller.set_selection(TextSelection::new(1, 3));
    controller.set_composing(Some(ComposingRange::new(1, 3)));

    let builder = LayoutBuilder::new(move |_, _| editor.clone());
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, Column::new([Widget::from(builder)]).into())
        .unwrap();
    frame(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(original));
    assert!(node.has_primary_focus());
    assert_eq!(controller.selection(), TextSelection::new(1, 3));
    assert_eq!(controller.composing(), Some(ComposingRange::new(1, 3)));
}

#[test]
fn a_global_key_moves_a_focus_node_into_and_out_of_an_inactive_branch() {
    let parent = FocusScopeNode::new();
    let child = FocusScopeNode::new();
    let node = FocusNode::new();
    let key = GlobalKey::new();
    let scope_key = GlobalKey::new();
    let branch = |state| {
        let in_scope = |widget: Widget| {
            Widget::from(FocusScope::new(widget).node(child.clone())).with_key(scope_key)
        };
        let moving = Widget::from(leaf(&node, false)).with_key(key);
        let children = match state {
            0 => vec![moving],
            1 => vec![moving, in_scope(SizedBox::shrink().into())],
            2 => vec![in_scope(moving)],
            _ => unreachable!(),
        };
        Widget::from(FocusScope::new(Column::new(children)).node(parent.clone()))
    };
    let mut runtime = Runtime::new(branch(0)).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let retained_leaf = runtime.tree().element_with_key(&key.into()).unwrap();
    // The Incular convenience on a widget's scope autofocus finds an
    // eligible descendant. Request the scope itself to match the
    // pinned fixture's starting focus before moving its leaf.
    parent.request_scope_focus();
    frame(&mut runtime);
    assert!(parent.has_primary_focus());
    assert!(!node.has_focus());
    assert!(parent.request_focus(&node));
    frame(&mut runtime);
    assert!(node.has_primary_focus());
    assert_eq!(parent.focused_child(), Some(node.clone()));

    runtime.schedule_update(root, branch(1)).unwrap();
    frame(&mut runtime);
    assert!(parent.set_first_focus(&child));
    frame(&mut runtime);
    assert!(child.is_first_focus() && child.has_primary_focus());
    assert!(!node.has_focus());

    runtime.schedule_update(root, branch(2)).unwrap();
    frame(&mut runtime);
    assert_eq!(
        runtime.tree().element_with_key(&key.into()),
        Some(retained_leaf)
    );
    assert_eq!(node.parent(), Some(child.scope_focus_node()));
    assert!(!node.has_focus());

    runtime.schedule_update(root, branch(0)).unwrap();
    frame(&mut runtime);
    assert_eq!(
        runtime.tree().element_with_key(&key.into()),
        Some(retained_leaf)
    );
    assert_eq!(node.parent(), Some(parent.scope_focus_node()));
    assert_eq!(child.scope_focus_node().parent(), None);
    assert!(!node.has_focus());
}

#[test]
fn removing_all_widget_focuses_clears_primary_focus_and_retained_host_links() {
    use std::{cell::Cell, rc::Rc};

    let changes = Rc::new(Cell::new(false));
    let focus_key = GlobalKey::new();
    let scope = FocusScopeNode::new();
    let wrapped = {
        let changes = changes.clone();
        Focus::new(Widget::from(SizedBox::square(20.)).with_key(focus_key))
            .on_focus_change(move |focused| changes.set(focused))
    };
    let view = FocusScope::new(wrapped).node(scope.clone());
    let mut runtime = Runtime::new(Column::new([Widget::from(view)]).into()).unwrap();
    frame(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let element = runtime.tree().element_with_key(&focus_key.into()).unwrap();
    let node = Focus::of(runtime.tree(), element, false);
    let base = scope.scope_focus_node();
    assert_eq!(base.descendants(), [node.clone()]);
    node.request_focus();
    frame(&mut runtime);
    assert!(changes.get() && node.has_primary_focus());

    runtime
        .schedule_update(root, Column::new([Widget::from(SizedBox::shrink())]).into())
        .unwrap();
    frame(&mut runtime);
    assert!(runtime.focused_element().is_none());
    assert_eq!(runtime.tree().element_with_key(&focus_key.into()), None);
    assert!(!node.has_focus() && !scope.has_focus());
    assert_eq!(node.parent(), None);
    assert!(base.descendants().is_empty());
    assert_eq!(scope.focused_child(), None);
}
