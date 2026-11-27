use incular_config::Constraints;
use incular_core::{DiagnosticsSerializationOptions, Size};
use incular_widgets::{
    Builder, Column, Placeholder, SizedBox, StatefulBuilder, Widget, internal::WidgetTree,
};

#[test]
fn placeholder_element_reports_its_widget_type_not_its_layout_delegate() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(Placeholder::new().into()).unwrap();

    let diagnostics = tree.element_diagnostics(root).unwrap();
    let json = diagnostics.to_json(DiagnosticsSerializationOptions::default(), &());
    assert_eq!(json["widgetRuntimeType"], "Placeholder");
    assert_eq!(json["stateful"], false);
    assert_eq!(json["type"], "_ElementDiagnosticableTreeNode");
    assert!(!json.contains_key("children"));
}

#[test]
fn retained_local_state_is_reported_before_the_first_layout() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(StatefulBuilder::new(|_, _| SizedBox::square(10.).into()).into())
        .unwrap();

    let diagnostics = tree.element_diagnostics(root).unwrap();
    let json = diagnostics.to_json(DiagnosticsSerializationOptions::default(), &());
    assert_eq!(json["widgetRuntimeType"], "StatefulBuilder");
    assert_eq!(json["stateful"], true);
}

#[test]
fn reports_retained_children_in_order_and_rejects_stale_ids() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            Column::new([
                Widget::from(Builder::new(|_| Placeholder::new().into())),
                Widget::from(StatefulBuilder::new(|_, _| SizedBox::square(10.).into())),
                Widget::from(Placeholder::new()),
            ])
            .into(),
        )
        .unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();

    let diagnostics = tree.element_diagnostics(root).unwrap();
    let json = diagnostics.to_json(
        DiagnosticsSerializationOptions {
            subtree_depth: 1,
            include_properties: false,
        },
        &(),
    );
    assert_eq!(json["widgetRuntimeType"], "Column");
    let children = json["children"].as_array().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0]["stateful"], false);
    assert_eq!(children[1]["stateful"], true);
    assert_eq!(children[2]["widgetRuntimeType"], "Placeholder");
    let new_root = tree.mount(SizedBox::square(2.).into()).unwrap();
    assert_ne!(root, new_root);
    assert!(tree.element_diagnostics(root).is_none());
}
