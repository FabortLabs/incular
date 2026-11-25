#![cfg(debug_assertions)]

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{
    Color, Size, Transform,
    memory::allocations::{MemoryAllocations, ObjectEventKind},
};
use incular_rendering::LayerTree;
use incular_widgets::{Widget, internal::WidgetTree};

#[test]
fn real_render_object_and_layer_publish_balanced_lifetimes() {
    let registry = MemoryAllocations::current();
    let events = Rc::new(RefCell::new(Vec::new()));
    let _subscription = registry.add_listener({
        let events = events.clone();
        move |event| events.borrow_mut().push(event)
    });

    let mut tree = WidgetTree::new();
    let make_box = |color| Widget::box_(Size::new(30., 40.), color);
    let root = tree.mount(make_box(Color::WHITE)).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    let render_events = || {
        events
            .borrow()
            .iter()
            .filter(|event| event.library == "rendering" && event.class_name == "RenderObject")
            .copied()
            .collect::<Vec<_>>()
    };
    let created = render_events()[0];
    assert_eq!(created.kind, ObjectEventKind::Created);
    tree.update(root, make_box(Color::BLACK)).unwrap();
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    assert_eq!(render_events(), [created]);

    let before = events.borrow().len();
    let mut layers = LayerTree::new();
    let layer = layers.create_transform(Transform::IDENTITY);
    assert_eq!(events.borrow().len(), before + 1);
    let layer_created = events.borrow()[before];
    assert_eq!(layer_created.kind, ObjectEventKind::Created);
    assert_eq!(
        (layer_created.library, layer_created.class_name),
        ("rendering", "Layer")
    );
    layers.remove(layer);
    let layer_disposed = events.borrow()[before + 1];
    assert_eq!(layer_disposed.kind, ObjectEventKind::Disposed);
    assert_eq!(layer_disposed.id, layer_created.id);
    assert_ne!(layer_disposed.id, created.id);

    drop(tree);
    assert_eq!(render_events().len(), 2);
    let disposed = render_events()[1];
    assert_eq!(disposed.kind, ObjectEventKind::Disposed);
    assert_eq!(disposed.id, created.id);
}
