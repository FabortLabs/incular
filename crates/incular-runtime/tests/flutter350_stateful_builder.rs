use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Column, Directionality, Placeholder, SizedBox, StateSetter, StatefulBuilder, Text,
    TextDirection, Widget,
};

fn frame(runtime: &mut Runtime, size: Size) {
    runtime.run_frame(Constraints::tight(size)).unwrap();
}

fn view() -> Size {
    Size::new(200., 100.)
}

#[test]
fn stateful_builder_mounts_with_zero_area() {
    let child =
        Widget::from(StatefulBuilder::new(|_, _| Placeholder::new().into())).with_key("stateful");
    assert_eq!(child.debug_type_name(), "StatefulBuilder");
    let pane = Directionality::new(TextDirection::Ltr, Center::new(child));
    let mut runtime = Runtime::new(pane.into()).unwrap();
    frame(&mut runtime, Size::ZERO);
    let tree = runtime.tree();
    let id = tree.element_with_key(&"stateful".into()).unwrap();
    assert_eq!(tree.element_bounds(id).unwrap().size, Size::ZERO);
}

#[test]
fn cloned_descriptors_have_independent_setters_and_resize_does_not_rebuild_them() {
    let setters = Rc::new(RefCell::new(Vec::<StateSetter>::new()));
    let template = Widget::from(StatefulBuilder::new({
        let setters = setters.clone();
        move |_, setter| {
            setters.borrow_mut().push(setter);
            SizedBox::square(20.).into()
        }
    }));
    let mut runtime = Runtime::new(
        Column::new([
            template.clone().with_key("first"),
            template.with_key("second"),
        ])
        .into(),
    )
    .unwrap();
    frame(&mut runtime, view());
    assert_eq!(setters.borrow().len(), 2);
    frame(&mut runtime, Size::new(300., 200.));
    assert_eq!(setters.borrow().len(), 2);

    let first = setters.borrow()[0].clone();
    let second = setters.borrow()[1].clone();
    assert!(first.set_state(|| {}));
    assert!(first.set_state(|| {}));
    frame(&mut runtime, view());
    assert_eq!(setters.borrow().len(), 3, "only one builder runs once");
    assert!(second.set_state(|| {}));
    frame(&mut runtime, view());
    assert_eq!(setters.borrow().len(), 4, "only the other builder runs");
}

#[test]
fn retained_setter_survives_parent_update_but_not_its_own_unmount() {
    let count = Rc::new(Cell::new(0));
    let received = Rc::new(RefCell::new(None::<StateSetter>));
    let scene = |prefix: Option<&'static str>| {
        let children = prefix.map(|prefix| {
            let (count, received) = (count.clone(), received.clone());
            Widget::from(StatefulBuilder::new(move |_, setter| {
                received.borrow_mut().get_or_insert(setter);
                Text::new(format!("{prefix} {}", count.get())).into()
            }))
            .with_key("retained")
        });
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            Column::new(children),
        ))
    };
    let mut runtime = Runtime::new(scene(Some("before"))).unwrap();
    frame(&mut runtime, view());
    let initial_id = runtime.tree().element_with_key(&"retained".into()).unwrap();
    let setter = received.borrow().clone().unwrap();
    let root = runtime.tree().root().unwrap();
    let assert_label = |runtime: &Runtime, text: &str| {
        assert!(
            runtime
                .tree()
                .semantics()
                .iter()
                .any(|(_, node)| node.label.as_deref() == Some(text))
        );
    };
    assert_label(&runtime, "before 0");
    runtime.schedule_update(root, scene(Some("after"))).unwrap();
    frame(&mut runtime, view());
    assert_eq!(
        runtime.tree().element_with_key(&"retained".into()),
        Some(initial_id)
    );
    assert_label(&runtime, "after 0");

    assert!(setter.set_state(|| count.set(1)));
    frame(&mut runtime, view());
    assert_label(&runtime, "after 1");
    runtime.schedule_update(root, scene(None)).unwrap();
    frame(&mut runtime, view());
    assert!(!setter.set_state(|| count.set(2)));
    assert_eq!(count.get(), 1);
}
