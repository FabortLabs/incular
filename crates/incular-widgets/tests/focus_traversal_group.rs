//! Flutter `FocusTraversalGroup.descendantsAreFocusable` and
//! `descendantsAreTraversable`.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ActionId, WidgetTree, action};
use incular_widgets::{Column, FocusTraversalGroup};

fn group(
    configure: impl Fn(FocusTraversalGroup) -> FocusTraversalGroup,
) -> (WidgetTree, Vec<bool>, usize) {
    let buttons = Column::new(vec![
        action(Size::new(20., 20.), Color::WHITE, ActionId(1)),
        action(Size::new(20., 20.), Color::WHITE, ActionId(2)),
    ]);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(configure(FocusTraversalGroup::new(buttons)).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .expect("layout");
    let column = root;
    let children = tree.children(column).expect("buttons").to_vec();
    let focusable = children
        .iter()
        .map(|child| tree.can_request_focus(*child))
        .collect();
    let traversable = tree.focusable_elements().len();
    (tree, focusable, traversable)
}

#[test]
fn descendants_are_focusable_and_traversable_by_default() {
    let (_, focusable, traversable) = group(|group| group);
    assert_eq!(focusable, [true, true]);
    assert_eq!(traversable, 2);
}

#[test]
fn untraversable_descendants_stay_directly_focusable() {
    let (_, focusable, traversable) = group(|group| group.descendants_are_traversable(false));
    assert_eq!(focusable, [true, true]);
    assert_eq!(traversable, 0);
}

#[test]
fn unfocusable_descendants_cannot_take_focus() {
    let (_, focusable, traversable) = group(|group| group.descendants_are_focusable(false));
    assert_eq!(focusable, [false, false]);
    assert_eq!(traversable, 0);
}
