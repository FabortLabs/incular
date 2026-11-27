use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_controls::widget_previews::preview_soft_restart_button;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, SizedBox, Text, Widget, widget_previews::restart::PreviewSoftRestart,
};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(400., 180.)))
        .unwrap();
}

#[test]
fn soft_restart_keeps_the_old_extent_for_one_frame_and_then_remounts_only_that_preview() {
    let restart = PreviewSoftRestart::default();
    let starts = Rc::new(Cell::new(0));
    let events = Rc::new(RefCell::new(Vec::new()));
    let _observer = restart.observe({
        let events = events.clone();
        move |value| events.borrow_mut().push(value)
    });
    let original_size = Size::new(120., 42.);
    let preview = restart.wrap({
        let starts = starts.clone();
        move || {
            starts.set(starts.get() + 1);
            Widget::from(SizedBox::from_size(original_size).child(Text::new("Foo")))
                .with_key("preview.restart.content")
        }
    });
    let unrelated = Widget::from(Text::new("Another Preview")).with_key("other.preview");
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            Column::new([
                preview_soft_restart_button(restart.clone()),
                preview,
                unrelated,
            ]),
        )
        .into(),
    )
    .unwrap();
    runtime.set_platform_semantics_enabled(true);
    frame(&mut runtime);
    let old_preview = runtime
        .tree()
        .element_with_key(&"preview.restart.content".into())
        .unwrap();
    let other = runtime
        .tree()
        .element_with_key(&"other.preview".into())
        .unwrap();
    assert_eq!(starts.get(), 1);
    assert_eq!(restart.last_child_size(), original_size);

    let tree = runtime.tree();
    let button = tree
        .element_with_key(&"preview.restart.soft".into())
        .unwrap();
    let bounds = tree.element_bounds(button).unwrap();
    let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer { position, phase });
    }
    assert!(restart.is_restarting());
    assert_eq!(*events.borrow(), [true]);
    assert!(!restart.request());
    frame(&mut runtime);

    let tree = runtime.tree();
    let placeholder = tree
        .element_with_key(&"preview.restart.placeholder".into())
        .unwrap();
    assert_eq!(
        tree.element_bounds(placeholder).unwrap().size,
        original_size
    );
    assert!(
        tree.element_with_key(&"preview.restart.content".into())
            .is_none()
    );
    assert!(
        !tree
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("Foo"))
    );
    assert_eq!(tree.element_with_key(&"other.preview".into()), Some(other));
    assert_eq!(*events.borrow(), [true, false]);
    assert_eq!(starts.get(), 1);

    frame(&mut runtime);
    let tree = runtime.tree();
    let new_preview = tree
        .element_with_key(&"preview.restart.content".into())
        .unwrap();
    assert_ne!(new_preview, old_preview);
    assert!(
        tree.element_with_key(&"preview.restart.placeholder".into())
            .is_none()
    );
    assert!(
        tree.semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some("Foo"))
    );
    assert_eq!(tree.element_with_key(&"other.preview".into()), Some(other));
    assert_eq!(starts.get(), 2);
}
