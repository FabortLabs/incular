//! The native AccessKit adapter rejected the update of an application whose
//! semantic content sat under a layout-only widget: the projection published
//! every node but attached only the first root to the window. The real
//! `accesskit_consumer` tree validates each update here, as the adapter does.

use accesskit_consumer::{Node, Tree, TreeChangeHandler};
use incular_accessibility::AccessKitProjection;
use incular_config::Constraints;
use incular_core::Size;
use incular_semantics::Role as SemanticRole;
use incular_widgets::{
    Column, Text, TextDirection, Widget,
    internal::{ExplicitSemantics, WidgetTree},
};

struct Ignore;

impl TreeChangeHandler for Ignore {
    fn node_added(&mut self, _: &Node) {}
    fn node_updated(&mut self, _: &Node, _: &Node) {}
    fn focus_moved(&mut self, _: Option<&Node>, _: Option<&Node>) {}
    fn node_removed(&mut self, _: &Node) {}
}

fn content(labels: &[(u64, &str)]) -> Widget {
    Widget::from(Column::new(
        labels
            .iter()
            .map(|&(key, label)| {
                Widget::from(Text::new(label).text_direction(TextDirection::Ltr))
                    .with_key(key)
                    .semantics(ExplicitSemantics::new(SemanticRole::Group).label(label.to_owned()))
            })
            .collect::<Vec<_>>(),
    ))
}

fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(200.0, 120.0)))
        .expect("layout");
    tree.update_semantics();
}

fn window_children(tree: &Tree) -> Vec<String> {
    tree.state()
        .root()
        .children()
        .filter_map(|node| node.label())
        .collect()
}

#[test]
fn layout_only_root_content_reaches_the_native_tree() {
    let mut widgets = WidgetTree::new();
    let root = widgets
        .mount(content(&[(1, "play"), (2, "pause"), (3, "stop")]))
        .expect("mount");
    frame(&mut widgets);
    assert_eq!(widgets.semantics().roots().len(), 3);

    let mut projection = AccessKitProjection::new();
    projection.activate();
    let initial = projection
        .sync(widgets.semantics(), 1.0)
        .expect("initial tree")
        .into_accesskit();
    let mut native = Tree::new(initial, true);
    assert_eq!(window_children(&native), ["play", "pause", "stop"]);

    // Adding, dropping and reordering top-level content stays valid.
    widgets
        .update(root, content(&[(3, "stop"), (4, "seek"), (1, "play")]))
        .unwrap();
    frame(&mut widgets);
    let next = projection
        .sync(widgets.semantics(), 1.0)
        .expect("root change")
        .into_accesskit();
    native.update_and_process_changes(next, &mut Ignore);
    assert_eq!(window_children(&native), ["stop", "seek", "play"]);
}
