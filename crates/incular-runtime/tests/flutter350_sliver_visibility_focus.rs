//! SliverVisibility is not box Visibility: it keeps focus and has no box scope.
use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, CustomScrollView, Directionality, Focus, FocusNode, ScrollController, SizedBox,
    Sliver, SliverToBoxAdapter, SliverVisibility, Text, TextDirection, TickerMode, Visibility,
    Widget, internal::Key,
};
use std::{cell::Cell, rc::Rc};

const READER: Key = Key::Value(41);
const NEXT: Key = Key::Value(42);

fn check_sliver_visibility(keep_size: bool) {
    let controller = ScrollController::new();
    let focus = FocusNode::new();
    let observed = Rc::new(Cell::new(None));
    let reader = Widget::from(Builder::new({
        let observed = Rc::clone(&observed);
        move |context| {
            let visible = Visibility::of(context);
            let tick = TickerMode::of(context);
            observed.set(Some((visible, tick)));
            SizedBox::new()
                .height(120.)
                .child(Text::new(format!("{visible} {tick}")))
                .into()
        }
    }))
    .with_key(READER);
    let child = Widget::from(Focus::new(reader).node(focus.clone()));
    let build = |visible| {
        let first = SliverVisibility::new(visible, SliverToBoxAdapter::new(child.clone()))
            .maintain_state(true)
            .maintain_animation(keep_size)
            .maintain_size(keep_size);
        let next = Widget::from(SizedBox::new().height(600.)).with_key(NEXT);
        let slivers: Vec<Box<dyn Sliver>> =
            vec![Box::new(first), Box::new(SliverToBoxAdapter::new(next))];
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers).controller(controller.clone()),
        ))
    };

    let mut runtime = Runtime::new(build(true)).unwrap();
    let frame = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(frame).unwrap();
    let reader = runtime.tree().element_with_key(&READER).unwrap();
    let next_y = |runtime: &Runtime| {
        let tree = runtime.tree();
        tree.element_bounds(tree.element_with_key(&NEXT).unwrap())
            .unwrap()
            .origin
            .y
    };
    assert_eq!((next_y(&runtime), controller.max_offset()), (120., 120.));
    focus.request_focus();
    runtime.run_frame(frame).unwrap();
    assert!(focus.has_primary_focus());
    assert_eq!(observed.get(), Some((true, true)));

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(false)).unwrap();
    runtime.run_frame(frame).unwrap();
    let hidden = (
        runtime.tree().element_with_key(&READER) == Some(reader),
        focus.has_primary_focus(),
        observed.get(),
        next_y(&runtime),
        controller.max_offset(),
    );
    focus.unfocus();
    runtime.run_frame(frame).unwrap();
    assert!(!focus.has_focus());
    focus.request_focus();
    runtime.run_frame(frame).unwrap();
    let can_refocus_when_hidden = focus.has_primary_focus();

    runtime.schedule_update(root, build(true)).unwrap();
    runtime.run_frame(frame).unwrap();
    let shown = (
        runtime.tree().element_with_key(&READER) == Some(reader),
        focus.has_primary_focus(),
        observed.get(),
        next_y(&runtime),
        controller.max_offset(),
    );
    let extent = if keep_size { 120. } else { 0. };
    assert_eq!(
        (hidden, can_refocus_when_hidden, shown),
        (
            (true, true, Some((true, keep_size)), extent, extent),
            true,
            (true, true, Some((true, true)), 120., 120.),
        ),
    );
}

#[test]
fn retained_offstage_sliver_preserves_focus_but_mutes_tickers() {
    check_sliver_visibility(false);
}

#[test]
fn maintained_size_sliver_preserves_focus_and_ticker_mode() {
    check_sliver_visibility(true);
}

#[test]
fn default_hidden_lazy_sliver_never_materializes_its_children() {
    use incular_widgets::SliverFixedExtentList;
    let builds = Rc::new(Cell::new(0));
    let build = |visible| {
        let list = SliverFixedExtentList::new(30, 100., {
            let builds = Rc::clone(&builds);
            move |index| {
                builds.set(builds.get() + 1);
                Widget::from(Text::new(format!("Item {index}")))
                    .with_key(Key::Value(1000 + index as u64))
            }
        });
        Widget::from(Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new([
                Box::new(SliverVisibility::new(visible, list)) as Box<dyn Sliver>
            ]),
        ))
    };
    let mut runtime = Runtime::new(build(false)).unwrap();
    let frame = Constraints::tight(Size::new(800., 600.));
    let item = |r: &Runtime| r.tree().element_with_key(&Key::Value(1000));
    runtime.run_frame(frame).unwrap();
    let initial = builds.get();
    assert!(item(&runtime).is_none());
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(true)).unwrap();
    runtime.run_frame(frame).unwrap();
    let shown = builds.get();
    assert!(shown > initial);
    assert!(item(&runtime).is_some());
    runtime.schedule_update(root, build(false)).unwrap();
    runtime.run_frame(frame).unwrap();
    let hidden_again = builds.get();
    assert!(item(&runtime).is_none());
    assert_eq!((initial, hidden_again), (0, shown));
}
