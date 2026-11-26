use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_widgets::{
    IndexedStack, Semantics, Widget,
    internal::{IndexedStackChildDebugStyle as Style, WidgetTree},
};

fn stack(selection: Option<usize>) -> IndexedStack {
    IndexedStack::new([
        Semantics::new(Widget::box_(Size::new(100., 100.), Color::WHITE)).label("first"),
        Semantics::new(Widget::box_(Size::new(100., 100.), Color::WHITE)).label("second"),
        Semantics::new(Widget::box_(Size::new(100., 100.), Color::WHITE)).label("third"),
    ])
    .index(selection)
    .text_direction(TextDirection::Ltr)
}

fn mount(selection: Option<usize>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(stack(selection).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    tree.update_semantics();
    tree
}

fn descriptions(tree: &WidgetTree, styles: [Style; 3]) {
    let root = tree.root().unwrap();
    let render = tree.render_id(root).unwrap();
    let rendered = tree.indexed_stack_child_diagnostics(render).unwrap();
    assert_eq!(rendered.len(), 3);
    for (i, (item, style)) in rendered.iter().zip(styles).enumerate() {
        assert_eq!(item.style, style);
        assert_eq!(item.name, format!("child {}", i + 1));
        let child = tree.children(root).unwrap()[i];
        assert_eq!(item.render, tree.render_id(child).unwrap());
        assert_eq!(
            tree.element_bounds(child).unwrap().size,
            Size::new(100., 100.)
        );
    }
}

#[test]
fn semantics_visits_only_the_displayed_child() {
    let tree = mount(Some(1));
    let names: Vec<_> = tree
        .semantics()
        .iter()
        .filter_map(|(_, node)| node.label.as_deref())
        .collect();
    assert_eq!(names, ["second"]);
}

#[test]
fn the_retained_diagnostic_list_marks_only_the_nonselected_children_offstage() {
    descriptions(
        &mount(Some(2)),
        [Style::Offstage, Style::Offstage, Style::Sparse],
    );
}

#[test]
fn null_selection_marks_every_retained_child_offstage() {
    let tree = mount(None);
    descriptions(&tree, [Style::Offstage; 3]);
    assert!(
        tree.semantics()
            .iter()
            .all(|(_, node)| node.label.is_none())
    );
}
