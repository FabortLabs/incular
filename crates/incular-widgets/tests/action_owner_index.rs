//! Callback cleanup follows live action owners across retained-tree changes.

use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::internal::{ActionId, ActionSurface, Key, WidgetTree};
use incular_widgets::{Column, GlobalKey, SizedBox, Widget};
use std::collections::HashSet;

fn surface() -> ActionSurface {
    ActionSurface::with_child(SizedBox::shrink())
}

fn callbacks() -> Widget {
    surface()
        .on_press(|| {})
        .on_hover(|| {})
        .on_exit(|| {})
        .into()
}

fn pending_actions(tree: &mut WidgetTree) -> HashSet<ActionId> {
    tree.take_pending_handlers()
        .into_iter()
        .map(|(action, _)| action)
        .collect()
}

#[test]
fn callback_replacement_and_removal_keep_only_current_actions() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(callbacks()).expect("mount callbacks");
    let original = pending_actions(&mut tree);
    assert_eq!(original.len(), 3, "press, hover and exit are bound");
    assert_eq!(tree.action_ids(), original);

    tree.update(root, surface().on_hover(|| {}).into())
        .expect("replace callbacks");
    let replacement = pending_actions(&mut tree);
    assert_eq!(replacement.len(), 1);
    assert!(replacement.is_disjoint(&original));
    assert_eq!(tree.action_ids(), replacement);

    tree.update(root, surface().into())
        .expect("remove callbacks while retaining the button");
    assert!(tree.action_ids().is_empty());

    tree.update(root, surface().on_exit(|| {}).into())
        .expect("add a callback to the retained button");
    let restored = pending_actions(&mut tree);
    assert_eq!(restored.len(), 1);
    assert_eq!(tree.action_ids(), restored);
}

#[test]
fn unmount_and_arena_reuse_do_not_retain_stale_actions() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Column::new(vec![callbacks()]).into())
        .expect("mount action owner");
    let original_element = tree.children(root).expect("children")[0];
    let original = pending_actions(&mut tree);
    assert_eq!(tree.action_ids(), original);

    tree.update(root, Column::new(Vec::<Widget>::new()).into())
        .expect("unmount action owner");
    assert!(!tree.element_exists(original_element));
    assert!(tree.action_ids().is_empty());

    tree.update(
        root,
        Column::new(vec![SizedBox::shrink().into(), callbacks()]).into(),
    )
    .expect("reuse released arena slots");
    let replacement_element = tree.children(root).expect("children")[1];
    assert_ne!(replacement_element, original_element);
    let replacement = pending_actions(&mut tree);
    assert_eq!(replacement.len(), 3);
    assert!(replacement.is_disjoint(&original));
    assert_eq!(tree.action_ids(), replacement);
}

#[test]
fn global_key_moves_preserve_bound_actions_until_final_unmount() {
    let key = GlobalKey::new();
    let mut button = callbacks().with_key(key);
    let mut next = 100_u64;
    button.bind_callbacks(&mut |_| {
        let action = ActionId(next);
        next += 1;
        action
    });
    let expected = HashSet::from([ActionId(100), ActionId(101), ActionId(102)]);
    let empty = || -> Widget { Column::new(Vec::<Widget>::new()).into() };
    let containing = || -> Widget { Column::new(vec![button.clone()]).into() };
    let constraints = Constraints::tight(Size::new(100., 100.));
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Column::new(vec![containing(), empty()]).into())
        .expect("mount globally keyed owner");
    tree.layout(constraints).expect("initial layout");
    let owner = tree.element_with_key(&Key::Global(key)).expect("owner");
    assert_eq!(tree.action_ids(), expected);

    tree.update(root, Column::new(vec![empty(), containing()]).into())
        .expect("move owner to later sibling");
    tree.layout(constraints).expect("layout after move");
    assert_eq!(tree.element_with_key(&Key::Global(key)), Some(owner));
    assert_eq!(tree.action_ids(), expected);

    tree.update(root, Column::new(vec![containing(), empty()]).into())
        .expect("move owner to earlier sibling");
    tree.layout(constraints).expect("layout after move back");
    assert_eq!(tree.element_with_key(&Key::Global(key)), Some(owner));
    assert_eq!(tree.action_ids(), expected);

    tree.update(root, Column::new(vec![empty(), empty()]).into())
        .expect("park removed owner");
    tree.layout(constraints).expect("finalize parked owner");
    assert!(!tree.element_exists(owner));
    assert!(tree.action_ids().is_empty());
}
