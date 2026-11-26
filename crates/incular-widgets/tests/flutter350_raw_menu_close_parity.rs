use incular_config::Constraints;
use incular_core::{Color, Offset, PRIMARY_POINTER_BUTTON, PointerPhase, Size, time::Instant};
use incular_gestures::{PointerDeviceKind, RawPointerEvent};
use incular_widgets::{
    Center, GestureDetector, MenuController, RawMenuAnchor, RawMenuAnchorGroup, Widget,
    internal::{Key, WidgetTree},
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
    let child_key = Key::from("submenu-anchor");
    let button = Widget::from(GestureDetector::new(box_()).on_tap({
        let child = child.clone();
        move || child.open(None)
    }))
    .with_key(child_key.clone());
    let submenu = RawMenuAnchor::new(child.clone(), |_, _| box_())
        .child(button)
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
    let id = tree
        .element_with_key(&child_key)
        .expect("mounted submenu anchor");
    let bounds = tree.element_bounds(id).unwrap();
    let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let event = RawPointerEvent {
            buttons,
            button: Some(PRIMARY_POINTER_BUTTON),
            ..RawPointerEvent::new(
                0,
                1,
                PointerDeviceKind::Touch,
                0,
                position,
                phase,
                Instant::now(),
            )
        };
        let hit = tree.pointer_hit_test(position);
        let _ = tree.dispatch_raw_pointer_with_hit(1, event, &hit);
        let _ = tree.dispatch_pointer_gesture_raw_with_hit(1, event, &hit);
    }
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
    let rejected = assert_detached_close_children(&old);
    replacement.close();
    layout(&mut tree);
    replacement.open(None);
    layout(&mut tree);
    assert!(replacement.is_open(), "replacement must be attached");
    assert_eq!(
        (old_open, replacement_opened_from_old, rejected),
        (false, false, cfg!(debug_assertions)),
        "old controller must be detached; Flutter debug rejects closeChildren on it"
    );
}

#[test]
fn replacing_an_open_groups_controller_detaches_the_old_one_without_closing() {
    let old = MenuController::new();
    let replacement = MenuController::new();
    let child = MenuController::new();
    let content: Widget = RawMenuAnchor::new(child.clone(), |_, _| box_())
        .child(box_())
        .into();
    let group = |controller| Widget::from(RawMenuAnchorGroup::new(controller, content.clone()));
    let mut tree = WidgetTree::new();
    let id = tree.mount(group(old.clone())).unwrap();
    layout(&mut tree);
    old.close_children();
    child.open(None);
    layout(&mut tree);
    assert!(old.is_open());

    tree.update(id, group(replacement.clone())).unwrap();
    layout(&mut tree);
    assert!(!old.is_open());
    assert!(replacement.is_open() && child.is_open());
    old.close();
    assert!(replacement.is_open() && child.is_open());

    assert_detached_close_children(&old);
    replacement.close();
    layout(&mut tree);
    assert!(!replacement.is_open() && !child.is_open());
}

#[test]
fn a_deferred_parent_requests_children_but_its_hide_forces_descendant_cleanup_first() {
    let parent = MenuController::new();
    let child = MenuController::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let hide_parent: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::default();
    let submenu: Widget = RawMenuAnchor::new(child.clone(), |_, _| box_())
        .child(box_())
        .on_close_requested({
            let log = log.clone();
            move |_| log.borrow_mut().push("request child")
        })
        .on_close({
            let log = log.clone();
            move || log.borrow_mut().push("close child")
        })
        .into();
    let menu = RawMenuAnchor::new(parent.clone(), move |_, _| submenu.clone())
        .child(box_())
        .on_close_requested({
            let log = log.clone();
            let hide_parent = hide_parent.clone();
            move |hide| {
                log.borrow_mut().push("request parent");
                *hide_parent.borrow_mut() = Some(hide);
            }
        })
        .on_close({
            let log = log.clone();
            move || log.borrow_mut().push("close parent")
        });
    let mut tree = WidgetTree::new();
    tree.mount(Center::new(menu).into()).unwrap();
    layout(&mut tree);
    parent.open(None);
    layout(&mut tree);
    child.open(None);
    layout(&mut tree);
    log.borrow_mut().clear();

    parent.close();
    assert!(parent.is_open() && child.is_open());
    assert_eq!(&*log.borrow(), &["request parent", "request child"]);
    hide_parent.borrow_mut().take().unwrap()();
    layout(&mut tree);
    assert!(!parent.is_open() && !child.is_open());
    assert_eq!(
        &*log.borrow(),
        &[
            "request parent",
            "request child",
            "close child",
            "close parent"
        ]
    );
}

fn assert_detached_close_children(controller: &MenuController) -> bool {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| controller.close_children()));
    assert_eq!(result.is_err(), cfg!(debug_assertions));
    if let Err(ref error) = result {
        let diagnostic = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(diagnostic.contains("_anchor != null"));
    }
    result.is_err()
}

#[test]
fn removing_an_anchor_or_group_detaches_its_controller() {
    for is_group in [false, true] {
        let controller = MenuController::new();
        let child: Widget = if is_group {
            RawMenuAnchorGroup::new(controller.clone(), box_()).into()
        } else {
            RawMenuAnchor::new(controller.clone(), |_, _| box_())
                .child(box_())
                .into()
        };
        let mut tree = WidgetTree::new();
        let root = tree.mount(Center::new(child).into()).unwrap();
        layout(&mut tree);
        controller.close_children();
        tree.update(root, Center::new(box_()).into()).unwrap();
        layout(&mut tree);
        assert!(!controller.is_open());
        controller.close();
        assert_detached_close_children(&controller);
    }
}

#[test]
fn an_inherited_controller_obeys_the_same_detachment_as_the_anchor_controller() {
    let observed: Rc<RefCell<Vec<MenuController>>> = Rc::default();
    let child = incular_widgets::Builder::new({
        let observed = observed.clone();
        move |context| {
            observed
                .borrow_mut()
                .push(MenuController::maybe_of(context).unwrap());
            box_()
        }
    });
    let scene = |controller| {
        Widget::from(RawMenuAnchor::new(controller, |_, _| box_()).child(child.clone()))
    };
    let original = MenuController::new();
    let current = MenuController::new();
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(original.clone())).unwrap();
    layout(&mut tree);
    let inherited = observed.borrow().first().unwrap().clone();
    inherited.open(None);
    layout(&mut tree);
    assert!(original.is_open());

    tree.update(root, scene(current.clone())).unwrap();
    layout(&mut tree);
    assert!(!original.is_open() && !inherited.is_open());
    assert!(current.is_open());
    inherited.close();
    assert!(
        current.is_open(),
        "the former descendant handle has no authority"
    );
    current.close();
    layout(&mut tree);
    assert!(!current.is_open());
}
