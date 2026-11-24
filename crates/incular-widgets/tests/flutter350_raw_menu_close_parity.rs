use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::{
    Center, MenuController, RawMenuAnchor, RawMenuAnchorGroup, Widget, internal::WidgetTree,
};
use std::{cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

fn box_() -> Widget {
    Widget::box_(Size::new(30., 20.), Color::WHITE)
}

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = tree.paint();
}

fn paired_closure(grouped: bool, refuses: bool) {
    let group = MenuController::new();
    let root = MenuController::new();
    let child = MenuController::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let child_log = log.clone();
    let submenu = RawMenuAnchor::new(child.clone(), |_, _| box_())
        .child(box_())
        .on_close(move || child_log.borrow_mut().push("child"));
    let submenu = if refuses {
        submenu.on_close_requested(|_| {})
    } else {
        submenu
    };
    let content: Widget = submenu.into();
    let root_log = log.clone();
    let menu = RawMenuAnchor::new(root.clone(), move |_, _| content.clone())
        .child(box_())
        .on_close(move || root_log.borrow_mut().push("root"));
    let scene = if grouped {
        Widget::from(RawMenuAnchorGroup::new(group.clone(), menu))
    } else {
        menu.into()
    };
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(scene).into()).unwrap();
    layout(&mut tree);
    root.open(None);
    layout(&mut tree);
    child.open(None);
    layout(&mut tree);
    assert!(
        root.is_open() && child.is_open(),
        "both mounted menus must be open"
    );
    if grouped {
        group.close();
    } else {
        root.close();
    }
    layout(&mut tree);
    let observations = (root.is_open(), child.is_open(), log.borrow().clone());
    assert_eq!(observations, (false, false, vec!["child", "root"]));
}

#[test]
fn parent_closure_notifies_child_first_even_if_child_close_is_deferred() {
    paired_closure(false, true);
}

#[test]
fn group_closure_notifies_child_before_parent() {
    paired_closure(true, false);
}

#[test]
fn detached_close_is_safe() {
    MenuController::new().close();
}

#[test]
fn detached_controller_is_not_open() {
    assert!(!MenuController::new().is_open());
}

#[test]
fn close_children_on_replaced_controller_rejects_detachment_in_debug() {
    let old = MenuController::new();
    let mount =
        |controller| Widget::from(RawMenuAnchor::new(controller, |_, _| box_()).child(box_()));
    let mut tree = WidgetTree::new();
    let root = tree.mount(mount(old.clone())).unwrap();
    layout(&mut tree);
    old.close_children();
    let replacement = MenuController::new();
    tree.update(root, mount(replacement.clone())).unwrap();
    layout(&mut tree);
    old.open(None);
    layout(&mut tree);
    let old_open = old.is_open();
    let replacement_opened_from_old = replacement.is_open();
    let rejected = std::panic::catch_unwind(AssertUnwindSafe(|| old.close_children())).is_err();
    replacement.close();
    layout(&mut tree);
    replacement.open(None);
    layout(&mut tree);
    assert!(replacement.is_open(), "replacement must be attached");
    assert_eq!(
        (old_open, replacement_opened_from_old, rejected),
        (false, false, true),
        "old controller must be detached; Flutter debug rejects closeChildren on it"
    );
}
