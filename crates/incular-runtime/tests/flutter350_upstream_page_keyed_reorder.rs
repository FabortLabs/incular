//! PageView's reverse lookup is tied to a lazy delegate update, not scrolling.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Directionality, KeepAlive, PageController, PageView, Text, TextDirection, Widget, internal::Key,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn replace(runtime: &mut Runtime, widget: Widget) {
    runtime
        .schedule_update(runtime.tree().root().unwrap(), widget)
        .unwrap();
    frame(runtime);
}

#[test]
fn reverse_lookup_is_called_when_a_keyed_builder_delegate_changes_item_count() {
    let queries = Rc::new(RefCell::new(Vec::new()));
    let view = |count| {
        let pages = PageView::builder(count, |index| {
            Widget::from(Text::new(index.to_string())).with_key(Key::String(index.to_string()))
        })
        .find_child_index_callback({
            let queries = queries.clone();
            move |key| {
                queries.borrow_mut().push(key.clone());
                None
            }
        });
        Directionality::new(TextDirection::Ltr, pages).into()
    };
    let mut runtime = Runtime::new(view(7)).unwrap();
    frame(&mut runtime);
    assert!(queries.borrow().is_empty());

    replace(&mut runtime, view(77));
    assert_eq!(&*queries.borrow(), &[Key::from("0")]);
}

#[test]
fn parked_page_follows_its_key_across_an_offscreen_move_without_remounting() {
    let controller = PageController::new();
    let queries = Rc::new(RefCell::new(Vec::new()));
    let view = |items: &'static [&str]| {
        let pages = PageView::builder(items.len(), move |index| {
            let item = items[index];
            Widget::from(KeepAlive::new(true, Text::new(item))).with_key(Key::from(item))
        })
        .controller(controller.clone())
        .find_child_index_callback({
            let queries = queries.clone();
            move |key| {
                queries.borrow_mut().push(key.clone());
                items.iter().position(|&item| Key::from(item) == *key)
            }
        });
        Directionality::new(TextDirection::Ltr, pages).into()
    };
    let a = Key::from("a");
    let mut runtime = Runtime::new(view(&["a", "b", "c"])).unwrap();
    frame(&mut runtime);
    let first = runtime.tree().element_with_key(&a).unwrap();
    assert!(queries.borrow().is_empty());

    assert!(controller.jump_to_page(1));
    frame(&mut runtime);
    assert_eq!(runtime.tree().element_with_key(&a), Some(first));
    assert!(queries.borrow().is_empty());

    replace(&mut runtime, view(&["c", "b", "a"]));
    assert_eq!(&*queries.borrow(), &[Key::from("a"), Key::from("b")]);
    assert_eq!(controller.page(), Some(1.));
    assert_eq!(runtime.tree().element_with_key(&a), Some(first));
    assert!(controller.jump_to_page(2));
    frame(&mut runtime);
    assert_eq!(runtime.tree().element_with_key(&a), Some(first));

    assert!(controller.jump_to_page(1));
    frame(&mut runtime);
    replace(&mut runtime, view(&["c", "b"]));
    assert_eq!(runtime.tree().element_with_key(&a), None);
}
