//! Flutter's `SliverChildBuilderDelegate` and `SliverChildListDelegate`:
//! child counts, and the semantic indexes their wrappers give children.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_semantics::SemanticRole;
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    CustomScrollView, Semantics, SliverChildBuilderDelegate, SliverChildDelegate,
    SliverChildListDelegate, SliverList, Widget,
};

fn row(index: usize) -> Widget {
    Semantics::new(Widget::box_(Size::new(100., 20.), Color::WHITE))
        .role(SemanticRole::ListItem)
        .label(format!("row {index}"))
        .into()
}

#[test]
fn a_builder_without_a_count_ends_at_the_first_missing_child() {
    let delegate = SliverChildBuilderDelegate::new(|index| (index < 7).then(|| row(index)));
    assert_eq!(delegate.estimated_child_count(), 7);
    assert!(delegate.build(6).is_some());
    assert!(delegate.build(7).is_none());
    let counted = SliverChildBuilderDelegate::new(|index| Some(row(index))).child_count(3);
    assert_eq!(counted.estimated_child_count(), 3);
    assert!(counted.build(3).is_none());
}

#[test]
fn a_list_delegate_counts_its_children() {
    let delegate = SliverChildListDelegate::new((0..4).map(row));
    assert_eq!(delegate.estimated_child_count(), 4);
    assert!(delegate.build(4).is_none());
}

/// The semantic item indexes of the rows laid out in a 100 px viewport.
fn item_indexes(list: SliverList) -> Vec<Option<usize>> {
    let mut tree = WidgetTree::new();
    tree.mount(
        CustomScrollView::new(vec![Box::new(list) as Box<dyn incular_widgets::Sliver>]).into(),
    )
    .expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree.update_semantics();
    let mut indexes: Vec<(String, Option<usize>)> = tree
        .semantics()
        .iter()
        .filter(|(_, node)| node.role == SemanticRole::ListItem)
        .map(|(_, node)| {
            (
                node.label.clone().unwrap_or_default(),
                node.state.item_index,
            )
        })
        .collect();
    indexes.sort();
    indexes
        .into_iter()
        .map(|(_, index)| index)
        .take(3)
        .collect()
}

#[test]
fn semantic_indexes_follow_the_callback_and_offset() {
    let plain =
        SliverList::delegate(SliverChildBuilderDelegate::new(|i| Some(row(i))).child_count(10));
    assert_eq!(item_indexes(plain), [Some(0), Some(1), Some(2)]);
    let offset = SliverList::delegate(
        SliverChildBuilderDelegate::new(|i| Some(row(i)))
            .child_count(10)
            .semantic_index_offset(5),
    );
    assert_eq!(item_indexes(offset), [Some(5), Some(6), Some(7)]);
    // A callback can skip children, as separators are skipped in Flutter.
    let callback = SliverList::delegate(
        SliverChildBuilderDelegate::new(|i| Some(row(i)))
            .child_count(10)
            .semantic_index_callback(|_, index| index.is_multiple_of(2).then_some(index / 2)),
    );
    assert_eq!(item_indexes(callback)[0], Some(0));
}
