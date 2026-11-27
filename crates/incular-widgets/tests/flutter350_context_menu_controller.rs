use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_widgets::internal::{Key, TextEditingController, WidgetTree};
use incular_widgets::{
    ContextMenuButtonItem, ContextMenuButtonType, ContextMenuController, EditableText, SizedBox,
    StateSetter, StatefulBuilder, TextDirection, TextSelectionToolbarAnchors, Widget,
};

fn view() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn child(key: &Key, width: f32) -> Widget {
    Widget::box_(Size::new(width, 20.), Color::WHITE).with_key(key.clone())
}

fn pump(tree: &mut WidgetTree) {
    tree.layout(view()).unwrap();
    let _ = tree.paint();
}

fn only(tree: &WidgetTree, first: &Key, second: &Key, visible: (bool, bool)) {
    assert_eq!(
        (
            tree.element_with_key(first).is_some(),
            tree.element_with_key(second).is_some()
        ),
        visible
    );
}

#[test]
fn only_one_context_menu_is_visible_and_replacement_removes_the_first() {
    let a = Key::from("first context menu");
    let b = Key::from("second context menu");
    let closed = Rc::new(Cell::new(0));
    let first = ContextMenuController::new().on_remove({
        let closed = closed.clone();
        move || closed.set(closed.get() + 1)
    });
    let second = ContextMenuController::new();
    let mut tree = WidgetTree::new();
    tree.mount(first.host(second.host(SizedBox::shrink())))
        .unwrap();
    pump(&mut tree);
    only(&tree, &a, &b, (false, false));

    let show_first = || {
        first.show({
            let a = a.clone();
            move |_| child(&a, 30.)
        })
    };
    show_first();
    pump(&mut tree);
    only(&tree, &a, &b, (true, false));
    let first_element = tree.element_with_key(&a).unwrap();
    show_first();
    pump(&mut tree);
    only(&tree, &a, &b, (true, false));
    assert_eq!(tree.element_with_key(&a), Some(first_element));

    second.show({
        let b = b.clone();
        move |_| child(&b, 30.)
    });
    pump(&mut tree);
    only(&tree, &a, &b, (false, true));
    assert_eq!(closed.get(), 1);
    assert!(!first.is_shown());
    assert!(!first.remove());
    assert!(second.remove());
    pump(&mut tree);
    only(&tree, &a, &b, (false, false));
}

#[test]
fn a_hidden_controller_can_show_its_menu_again() {
    let menu = ContextMenuController::new();
    let key = Key::from("reshown menu");
    let mut tree = WidgetTree::new();
    tree.mount(menu.host(SizedBox::shrink())).unwrap();
    pump(&mut tree);
    assert!(tree.element_with_key(&key).is_none());
    let show = || {
        menu.show({
            let key = key.clone();
            move |_| child(&key, 30.)
        })
    };
    show();
    pump(&mut tree);
    assert!(tree.element_with_key(&key).is_some());
    assert!(menu.remove());
    pump(&mut tree);
    assert!(tree.element_with_key(&key).is_none());
    show();
    pump(&mut tree);
    assert!(tree.element_with_key(&key).is_some());
    assert!(ContextMenuController::remove_any());
    pump(&mut tree);
    assert!(tree.element_with_key(&key).is_none());
}

#[test]
fn a_dirty_menu_builder_runs_once_on_the_next_frame_not_inline() {
    let menu = ContextMenuController::new();
    let builds = Rc::new(Cell::new(0));
    let mut tree = WidgetTree::new();
    tree.mount(menu.host(SizedBox::shrink())).unwrap();
    pump(&mut tree);
    assert!(!menu.mark_needs_build());
    menu.show({
        let builds = builds.clone();
        move |_| {
            builds.set(builds.get() + 1);
            SizedBox::square(30.).into()
        }
    });
    assert_eq!(builds.get(), 0);
    pump(&mut tree);
    assert_eq!(builds.get(), 1);
    assert!(menu.mark_needs_build());
    assert_eq!(builds.get(), 1);
    pump(&mut tree);
    assert_eq!(builds.get(), 2);
    assert!(menu.remove());
}

#[test]
fn update_in_place_keeps_the_mounted_state_and_moves_to_the_new_content() {
    let menu = ContextMenuController::new();
    let key = Key::from("stateful menu");
    let setters = Rc::new(RefCell::new(Vec::<StateSetter>::new()));
    let mut tree = WidgetTree::new();
    tree.mount(menu.host(SizedBox::shrink())).unwrap();
    pump(&mut tree);

    let show = |value| {
        menu.show({
            let key = key.clone();
            let setters = setters.clone();
            move |_| {
                let key = key.clone();
                let setters = setters.clone();
                StatefulBuilder::new(move |_, setter| {
                    setters.borrow_mut().push(setter);
                    child(&key, value)
                })
                .into()
            }
        })
    };
    show(32.);
    pump(&mut tree);
    let old = setters.borrow()[0].clone();
    let element = tree.element_with_key(&key).unwrap();
    assert_eq!(tree.element_bounds(element).unwrap().size.width, 32.);
    show(64.);
    pump(&mut tree);
    assert_eq!(tree.element_with_key(&key), Some(element));
    assert_eq!(tree.element_bounds(element).unwrap().size.width, 64.);
    assert!(old.set_state(|| {}));
    assert!(menu.remove());
}

#[test]
fn removing_and_showing_before_a_frame_does_not_reuse_the_old_state() {
    let menu = ContextMenuController::new();
    let key = Key::from("fresh menu");
    let old = Rc::new(RefCell::new(None::<StateSetter>));
    let mut tree = WidgetTree::new();
    tree.mount(menu.host(SizedBox::shrink())).unwrap();
    pump(&mut tree);
    menu.show({
        let key = key.clone();
        let old = old.clone();
        move |_| {
            let key = key.clone();
            let old = old.clone();
            StatefulBuilder::new(move |_, setter| {
                old.replace(Some(setter));
                child(&key, 32.)
            })
            .into()
        }
    });
    pump(&mut tree);
    let original = tree.element_with_key(&key).unwrap();
    let original_state = old.borrow().as_ref().unwrap().clone();

    assert!(menu.remove());
    menu.show({
        let key = key.clone();
        move |_| {
            StatefulBuilder::new({
                let key = key.clone();
                move |_, _| child(&key, 64.)
            })
            .into()
        }
    });
    pump(&mut tree);
    let second = tree.element_with_key(&key).unwrap();
    assert_ne!(original, second);
    assert_eq!(tree.element_bounds(second).unwrap().size.width, 64.);
    assert!(!original_state.set_state(|| panic!("old state must not run")));
    assert!(menu.remove());
    pump(&mut tree);
    assert!(tree.element_with_key(&key).is_none());
}

#[test]
fn direct_and_mounted_editable_menus_replace_each_other() {
    let direct = ContextMenuController::new();
    let field_key = Key::from("editable");
    let built_in = Key::from("built-in context menu");
    let manual = Key::from("direct context menu");
    let mut tree = WidgetTree::new();

    let edit = EditableText::new(TextEditingController::with_text("words"))
        .text_direction(TextDirection::Ltr)
        .height(40.)
        .context_menu_builder({
            let built_in = built_in.clone();
            move |_, _| child(&built_in, 30.)
        });
    tree.mount(direct.host(Widget::from(edit).with_key(field_key.clone())))
        .unwrap();
    pump(&mut tree);
    only(&tree, &built_in, &manual, (false, false));
    let mut descendants = vec![tree.element_with_key(&field_key).unwrap()];
    let editor = loop {
        let element = descendants.pop().expect("mounted toolbar state");
        if let Some(state) = tree.text_field_context_menu(element) {
            break state;
        }
        descendants.extend(tree.children(element).unwrap_or_default());
    };
    let toolbar = || {
        editor.show(
            TextSelectionToolbarAnchors::at(Offset::new(80., 80.)),
            vec![ContextMenuButtonItem::new(
                ContextMenuButtonType::Copy,
                || {},
            )],
        )
    };
    assert!(toolbar());
    pump(&mut tree);
    only(&tree, &built_in, &manual, (true, false));

    direct.show({
        let manual = manual.clone();
        move |_| child(&manual, 30.)
    });
    pump(&mut tree);
    only(&tree, &built_in, &manual, (false, true));
    assert!(!editor.is_showing());
    assert!(direct.is_shown());

    assert!(toolbar());
    pump(&mut tree);
    only(&tree, &built_in, &manual, (true, false));
    assert!(!direct.is_shown());
    assert!(!direct.remove());
    pump(&mut tree);
    only(&tree, &built_in, &manual, (true, false));
    assert!(editor.hide());
    pump(&mut tree);
    only(&tree, &built_in, &manual, (false, false));
}
