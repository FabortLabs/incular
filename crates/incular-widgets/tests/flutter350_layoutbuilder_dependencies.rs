//! Flutter 3.50.0 LayoutBuilder geometry and rebuild dependence.
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    Builder, Center, ConstrainedBox, Directionality, LayoutBuilder, MediaQuery, MediaQueryData,
    Placeholder, SizedBox, Text, TextDirection, Widget,
    internal::{Key, WidgetTree},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const SURFACE: Size = Size {
    width: 800.,
    height: 600.,
};
fn frame(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(SURFACE)).unwrap();
    let _ = tree.paint();
}
fn mount(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    frame(&mut tree);
    tree
}
fn size(tree: &WidgetTree, key: u64) -> Size {
    let id = tree.element_with_key(&Key::Value(key)).unwrap();
    tree.element_bounds(id).unwrap().size
}
fn environment(viewport: Size) -> MediaQueryData {
    MediaQueryData {
        viewport,
        ..MediaQueryData::default()
    }
}

#[test]
fn layout_builder_sees_parent_constraints_and_adopts_generated_size() {
    let measured = Rc::new(Cell::new(None));
    let capture = measured.clone();
    let builder = LayoutBuilder::new(move |_, constraints| {
        let available = constraints.biggest();
        capture.set(Some(available));
        Widget::from(
            SizedBox::new()
                .width(available.width / 2.)
                .height(available.height / 2.),
        )
        .with_key(Key::Value(2))
    });
    let tree = mount(Center::new(ConstrainedBox::new(
        Constraints::loose(Size::new(100., 200.)),
        Widget::from(builder).with_key(Key::Value(1)),
    )));
    assert_eq!(measured.get(), Some(Size::new(100., 200.)));
    assert_eq!(size(&tree, 1), Size::new(50., 100.));
    assert_eq!(size(&tree, 2), Size::new(50., 100.));
}

#[test]
fn layout_builder_with_text_still_completes_at_zero_area() {
    let target =
        Widget::from(LayoutBuilder::new(|_, _| Text::new("X").into())).with_key(Key::Value(1));
    let tree = mount(Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::shrink().child(target)),
    ));
    assert_eq!(size(&tree, 1), Size::ZERO);
}

fn count_queries(depend: bool, expected_after: usize) {
    let count = Rc::new(Cell::new(0));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let target: Widget = LayoutBuilder::new({
        let count = count.clone();
        let seen = seen.clone();
        move |ctx, _| {
            count.set(count.get() + 1);
            if depend {
                seen.borrow_mut()
                    .push(ctx.depend_on::<MediaQueryData>().unwrap().viewport);
            }
            SizedBox::shrink().into()
        }
    })
    .into();
    let scoped = |viewport| Widget::from(MediaQuery::new(environment(viewport), target.clone()));
    assert_eq!(count.get(), 0);
    let mut tree = mount(scoped(Size::new(400., 300.)));
    let root = tree.root().unwrap();
    assert_eq!(count.get(), 1);
    tree.update(root, scoped(Size::new(300., 400.))).unwrap();
    frame(&mut tree);
    assert_eq!(count.get(), expected_after);
    if depend {
        assert_eq!(
            seen.borrow().as_slice(),
            [Size::new(400., 300.), Size::new(300., 400.)]
        );
    }
}
#[test]
fn unread_inherited_media_query_does_not_rebuild() {
    count_queries(false, 1);
}
#[test]
fn inherited_media_query_reader_rebuilds_once() {
    count_queries(true, 2);
}

#[test]
fn changing_constraints_and_inherited_scope_rebuilds_descendant_once() {
    let count = Rc::new(Cell::new(0));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let target: Widget = LayoutBuilder::new({
        let count = count.clone();
        let seen = seen.clone();
        move |_, constraints| {
            let count = count.clone();
            let seen = seen.clone();
            Builder::new(move |ctx| {
                count.set(count.get() + 1);
                let query = ctx.depend_on::<MediaQueryData>().unwrap().viewport;
                seen.borrow_mut().push((constraints.max_width(), query));
                Placeholder::new().into()
            })
            .into()
        }
    })
    .into();
    let scoped = |width, height| {
        Widget::from(MediaQuery::new(
            environment(Size::new(width, height)),
            Center::new(SizedBox::new().width(width).child(target.clone())),
        ))
    };
    assert_eq!(count.get(), 0);
    let mut tree = mount(scoped(400., 300.));
    let root = tree.root().unwrap();
    assert_eq!(count.get(), 1);
    tree.update(root, scoped(300., 400.)).unwrap();
    frame(&mut tree);
    assert_eq!(
        (count.get(), seen.borrow().clone()),
        (
            2,
            vec![(400., Size::new(400., 300.)), (300., Size::new(300., 400.))]
        )
    );
}
