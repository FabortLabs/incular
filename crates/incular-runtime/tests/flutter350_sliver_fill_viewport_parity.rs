//! Flutter 3.50.0 SliverFillViewport intrinsic paint extent, visibility, and semantics.
use incular_config::{Axis, Constraints};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, ScrollController, Sliver, SliverFillViewport, Text,
    TextDirection, Widget,
    internal::{Key, SliverConstraints},
};

fn viewport(sliver: SliverFillViewport, c: &ScrollController) -> Runtime {
    let view = CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>])
        .controller(c.clone())
        .cache_extent(250.);
    let mut r = Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).unwrap();
    frame(&mut r);
    r
}
fn frame(r: &mut Runtime) {
    r.run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}
#[test]
fn viewport_sized_children_enter_leave_and_reenter_the_painted_window() {
    let items = (0..20).map(|i| Widget::from(Text::new(i.to_string())).with_key(Key::Value(i)));
    let c = ScrollController::new();
    let mut r = viewport(SliverFillViewport::new(items), &c);
    let actual = [0., 700., 500., 0.].map(|offset| {
        let _ = c.jump_to(offset);
        frame(&mut r);
        let t = r.tree();
        (0..20)
            .filter(|&i| {
                t.element_with_key(&Key::Value(i))
                    .and_then(|id| t.element_bounds(id))
                    .is_some_and(|rect| {
                        assert_eq!(rect.size, Size::new(800., 600.));
                        rect.origin.y < 600. && rect.origin.y + rect.size.height > 0.
                    })
            })
            .collect::<Vec<_>>()
    });
    assert_eq!(actual, [vec![0], vec![1, 2], vec![0, 1], vec![0]]);
}
#[test]
fn end_padding_changes_one_half_fraction_pages_total_paint_extent() {
    let actual = [true, false].map(|ends| {
        let d = SliverFillViewport::new([Text::new("0")])
            .viewport_fraction(0.5)
            .pad_ends(ends);
        let mut sliver = d.create_render_sliver(&ScrollController::new(), Axis::Vertical, false);
        let constraints = SliverConstraints::new(
            Axis::Vertical,
            false,
            0.,
            0.,
            0.,
            600.,
            800.,
            600.,
            850.,
            0.,
        );
        sliver.perform_layout(constraints).geometry.paint_extent
    });
    assert_eq!(actual, [600., 300.]);
}
#[test]
fn default_implicit_scrolling_exposes_the_cached_second_page_in_semantics() {
    let r = viewport(
        SliverFillViewport::new([Text::new("Page 1"), Text::new("Page 2")]),
        &ScrollController::new(),
    );
    let t = r.tree();
    let actual = ["Page 1", "Page 2"].map(|label| {
        t.semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some(label))
    });
    assert_eq!(actual, [true, true]);
}

#[test]
fn disabling_implicit_scrolling_keeps_the_cached_page_out_of_runtime_semantics() {
    let c = ScrollController::new();
    let mut r = viewport(
        SliverFillViewport::new([
            Widget::from(Text::new("Page 1")).with_key(Key::Value(1)),
            Widget::from(Text::new("Page 2")).with_key(Key::Value(2)),
        ])
        .allow_implicit_scrolling(false),
        &c,
    );
    let cached = r
        .tree()
        .element_with_key(&Key::Value(2))
        .expect("cache keeps the second page");
    for (offset, expected) in [(0., [true, false]), (600., [false, true])] {
        if c.offset() != offset {
            assert!(c.jump_to(offset));
        }
        frame(&mut r);
        let tree = r.tree();
        let labels = ["Page 1", "Page 2"].map(|label| {
            tree.semantics()
                .iter()
                .any(|(_, node)| node.label.as_deref() == Some(label))
        });
        assert_eq!(labels, expected);
        assert_eq!(tree.element_with_key(&Key::Value(2)), Some(cached));
    }
}
