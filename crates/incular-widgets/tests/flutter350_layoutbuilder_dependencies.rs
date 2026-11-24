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

fn fixed(size: Size) -> Widget {
    Widget::from(SizedBox::new().width(size.width).height(size.height)).with_key(Key::Value(2))
}

#[test]
fn stateful_child_resizes_the_retained_layout_builder() {
    let child_size = Rc::new(Cell::new(Size::new(10., 20.)));
    let rev = Rc::new(Cell::new(0u64));
    let measured = Rc::new(Cell::new(Size::ZERO));
    let desc = LayoutBuilder::new({
        let child_size = child_size.clone();
        let rev = rev.clone();
        let measured = measured.clone();
        move |_, constraints| {
            measured.set(constraints.biggest());
            let child_size = child_size.clone();
            Widget::stateful_builder(rev.clone(), move |_| fixed(child_size.get()))
        }
    });
    let mut tree = mount(Center::new(Widget::from(desc).with_key(Key::Value(1))));
    assert_eq!(measured.get(), SURFACE);
    assert_eq!([size(&tree, 1), size(&tree, 2)], [Size::new(10., 20.); 2]);
    child_size.set(Size::new(100., 200.));
    rev.set(1);
    frame(&mut tree);
    assert_eq!([size(&tree, 1), size(&tree, 2)], [Size::new(100., 200.); 2]);
}

#[test]
fn stateful_parent_changes_constraints_seen_by_layout_builder() {
    let state = Rc::new(Cell::new(Size::new(10., 20.)));
    let rev = Rc::new(Cell::new(0u64));
    let measured = Rc::new(Cell::new(Size::ZERO));
    let parent = Widget::stateful_builder(rev.clone(), {
        let state = state.clone();
        let measured = measured.clone();
        move |_| {
            let observed = measured.clone();
            let physical = state.get();
            SizedBox::new()
                .width(physical.width)
                .height(physical.height)
                .child(LayoutBuilder::new(move |_, constraints| {
                    let limits = constraints.biggest();
                    observed.set(limits);
                    fixed(limits)
                }))
                .into()
        }
    });
    let mut tree = mount(Center::new(parent));
    assert_eq!([size(&tree, 2), measured.get()], [Size::new(10., 20.); 2]);
    state.set(Size::new(100., 200.));
    rev.set(1);
    frame(&mut tree);
    assert_eq!([size(&tree, 2), measured.get()], [Size::new(100., 200.); 2]);
}

#[test]
fn a_text_style_change_resizes_the_leaf_without_rebuilding_its_builder() {
    use incular_widgets::{DefaultTextStyle, TextStyle};
    let builds = Rc::new(Cell::new(0));
    let target: Widget = Widget::from(LayoutBuilder::new({
        let builds = builds.clone();
        move |_, _| {
            let builds = builds.clone();
            Builder::new(move |_| {
                builds.set(builds.get() + 1);
                Widget::from(Text::new("A")).with_key(Key::Value(2))
            })
            .into()
        }
    }))
    .with_key(Key::Value(1));
    let scene = |font_size| {
        Widget::from(Center::new(Directionality::new(
            TextDirection::Ltr,
            DefaultTextStyle::new(
                TextStyle::default().font_size(font_size).height(Some(1.)),
                target.clone(),
            ),
        )))
    };
    let mut tree = mount(scene(10.));
    let root = tree.root().unwrap();
    let first = (builds.get(), size(&tree, 1), size(&tree, 2));
    tree.update(root, scene(100.)).unwrap();
    frame(&mut tree);
    let second = (builds.get(), size(&tree, 1), size(&tree, 2));
    assert_eq!(
        (first.0, second.0),
        (1, 1),
        "original descendant builder count; {first:?} {second:?}"
    );
    assert_eq!((first.1, second.1), (first.2, second.2));
    assert_eq!((first.2.height, second.2.height), (10., 100.));
    assert!(
        second.2.width > first.2.width * 8.,
        "same glyph scales with new inherited style"
    );
}
