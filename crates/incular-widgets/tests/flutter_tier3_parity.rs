//! Flutter's `LexicalFocusOrder`, `LogicalKeySet`, `InheritedNotifier` and
//! `CapturedThemes`.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Code, Color, KeyboardEvent, KeyboardKey, Modifiers, NamedKey, Size};
use incular_widgets::internal::{Widget, WidgetTree};
use incular_widgets::{
    Builder, CapturedThemes, FocusNode, FocusOrder, FocusScope, FocusTraversalOrder,
    FocusableActionDetector, InheritedNotifier, LogicalKeySet, Row, TextDirection,
};

fn ordered(node: &FocusNode, order: FocusOrder) -> Widget {
    FocusTraversalOrder::new(
        FocusableActionDetector::new(Widget::box_(Size::new(10., 10.), Color::WHITE))
            .focus_node(node.clone()),
    )
    .focus_order(order)
    .into()
}

#[test]
fn lexical_focus_orders_sort_by_string() {
    let nodes = [FocusNode::new(), FocusNode::new(), FocusNode::new()];
    let children = vec![
        ordered(&nodes[0], FocusOrder::lexical("c")),
        ordered(&nodes[1], FocusOrder::lexical("a")),
        ordered(&nodes[2], FocusOrder::lexical("b")),
    ];
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            FocusScope::new(Row::new(children).text_direction(TextDirection::Ltr))
                .ordered()
                .into(),
        )
        .unwrap();
    tree.layout(Constraints::tight(Size::new(30., 10.)))
        .unwrap();
    let children = tree.children(root).unwrap().to_vec();
    assert_eq!(
        tree.focusable_elements(),
        vec![children[1], children[2], children[0]]
    );
}

#[test]
fn focus_order_compare_matches_flutter() {
    use std::cmp::Ordering;
    assert_eq!(
        FocusOrder::numeric(1.).compare(&FocusOrder::numeric(2.)),
        Ordering::Less
    );
    assert_eq!(
        FocusOrder::lexical("b").compare(&FocusOrder::lexical("a")),
        Ordering::Greater
    );
    assert!(!FocusOrder::numeric(f64::NAN).is_valid());
}

fn key(key: KeyboardKey, modifiers: Modifiers) -> KeyboardEvent {
    let mut event = KeyboardEvent::key_down(key, Code::KeyC);
    event.modifiers = modifiers;
    event
}

#[test]
fn logical_key_set_matches_exact_modifiers() {
    let copy = LogicalKeySet::new([
        KeyboardKey::Named(NamedKey::Control),
        KeyboardKey::Character("c".into()),
    ]);
    assert!(copy.accepts(&key(KeyboardKey::Character("c".into()), Modifiers::CONTROL)));
    assert!(!copy.accepts(&key(KeyboardKey::Character("c".into()), Modifiers::empty())));
    assert!(!copy.accepts(&key(
        KeyboardKey::Character("c".into()),
        Modifiers::CONTROL | Modifiers::SHIFT
    )));
    // Order does not matter, and duplicates collapse.
    let same = LogicalKeySet::new([
        KeyboardKey::Character("c".into()),
        KeyboardKey::Named(NamedKey::Control),
        KeyboardKey::Named(NamedKey::Control),
    ]);
    assert_eq!(same.activator(), copy.activator());
    // Two non-modifier keys can never be one event.
    let impossible = LogicalKeySet::new([
        KeyboardKey::Character("a".into()),
        KeyboardKey::Character("b".into()),
    ]);
    assert!(impossible.activator().is_none());
}

#[derive(Clone, Debug, PartialEq)]
struct Accent(Color);

#[test]
fn inherited_notifier_and_captured_themes_publish_values() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let captured = Rc::new(RefCell::new(None::<CapturedThemes>));
    let capture = {
        let captured = captured.clone();
        Builder::new(move |context| {
            *captured.borrow_mut() = Some(CapturedThemes::new().capture::<Accent>(context));
            Widget::box_(Size::new(1., 1.), Color::WHITE)
        })
    };
    let mut tree = WidgetTree::new();
    tree.mount(InheritedNotifier::new(Accent(Color::BLACK), capture).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(10., 10.)))
        .unwrap();
    let themes = captured.borrow().clone().expect("captured");
    assert_eq!(themes.len(), 1);

    // Elsewhere, outside the publishing subtree, the captured value applies.
    let reader = {
        let seen = seen.clone();
        Builder::new(move |context| {
            seen.borrow_mut()
                .push(InheritedNotifier::<Accent>::of(context));
            Widget::box_(Size::new(1., 1.), Color::WHITE)
        })
    };
    let mut other = WidgetTree::new();
    other.mount(themes.wrap(reader)).unwrap();
    other
        .layout(Constraints::tight(Size::new(10., 10.)))
        .unwrap();
    assert_eq!(seen.borrow().last(), Some(&Some(Accent(Color::BLACK))));
}
