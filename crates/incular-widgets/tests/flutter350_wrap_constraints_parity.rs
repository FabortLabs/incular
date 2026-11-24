//! Original Flutter 3.50.0 Wrap boundary geometry and painting.
use incular_config::{Alignment, Constraints, WrapAlignment, WrapCrossAlignment};
use incular_core::{Color, Offset, Size};
use incular_rendering::PaintCommand;
use incular_widgets::{
    Align, Center, Column, Directionality, IntrinsicHeight, Row, SizedBox, Text, TextDirection,
    TextStyle, Widget, Wrap,
    internal::{Key, WidgetTree},
};

const HOST: Size = Size {
    width: 800.,
    height: 600.,
};
const WRAP: u64 = 100;
fn bounds(tree: &WidgetTree, key: u64) -> incular_core::Rect {
    tree.element_bounds(tree.element_with_key(&Key::Value(key)).unwrap())
        .unwrap()
}
fn wrap(sizes: &[(f32, f32)]) -> Wrap {
    Wrap::new(sizes.iter().enumerate().map(|(i, &(x, y))| {
        Widget::box_(Size::new(x, y), Color::WHITE).with_key(Key::Value(i as u64))
    }))
    .text_direction(TextDirection::Ltr)
}
fn key(value: Wrap) -> Widget {
    Widget::from(value).with_key(Key::Value(WRAP))
}
fn mount(widget: impl Into<Widget>) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget.into()).unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    tree
}
fn check(tree: &WidgetTree, size: Size, offsets: &[(f32, f32)]) {
    assert_eq!(bounds(tree, WRAP).size, size);
    let actual: Vec<_> = (0..offsets.len())
        .map(|i| bounds(tree, i as u64).origin)
        .collect();
    let expected: Vec<_> = offsets.iter().map(|&(x, y)| Offset::new(x, y)).collect();
    assert_eq!(actual, expected);
}

#[test]
fn horizontal_wrap_shrinks_to_runs_and_aligns_ends() {
    let w = |sizes: &[(f32, f32)]| {
        Align::new(
            Alignment::TOP_LEFT,
            key(wrap(sizes)
                .alignment(WrapAlignment::End)
                .cross_axis_alignment(WrapCrossAlignment::End)),
        )
    };
    let a = [(100., 10.), (200., 20.), (300., 30.), (400., 40.)];
    let mut tree = mount(w(&a));
    check(
        &tree,
        Size::new(600., 70.),
        &[(0., 20.), (100., 10.), (300., 0.), (200., 30.)],
    );
    let root = tree.root().unwrap();
    tree.update(root, w(&[a[3], a[2], a[1], a[0]]).into())
        .unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    check(
        &tree,
        Size::new(700., 60.),
        &[(0., 0.), (400., 10.), (400., 40.), (600., 50.)],
    );
}
#[test]
fn nonoverflowing_default_wrap_does_not_record_a_clip() {
    let mut tree = mount(key(wrap(&[(500., 500.)])));
    assert!(
        !tree
            .paint()
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, PaintCommand::PushClip { .. }))
    );
}
#[test]
fn one_pixel_of_overflow_pushes_the_last_child_to_a_new_run() {
    let value = wrap(&[(200., 10.), (200., 10.), (200., 10.), (171., 10.)])
        .spacing(10.)
        .run_spacing(10.);
    let tree = mount(key(value));
    check(&tree, HOST, &[(0., 0.), (210., 0.), (420., 0.), (0., 20.)]);
}
#[test]
fn an_exact_width_child_uses_one_whole_run_without_an_extra_gap() {
    let build =
        |sizes: &[(f32, f32)]| Column::new([key(wrap(sizes).spacing(10.).run_spacing(10.))]);
    let mut tree = mount(build(&[(800., 10.)]));
    check(&tree, Size::new(800., 10.), &[(0., 0.)]);
    let root = tree.root().unwrap();
    tree.update(root, build(&[(800., 10.), (800., 10.)]).into())
        .unwrap();
    tree.layout(Constraints::tight(HOST)).unwrap();
    check(&tree, Size::new(800., 30.), &[(0., 0.), (0., 20.)]);
}
#[test]
fn intrinsic_height_counts_two_text_runs_and_the_full_width_row() {
    let text = |text| {
        Widget::from(Text::new(text).style(TextStyle::default().font_size(16.).height(Some(1.))))
    };
    let wrap = Wrap::new([
        text("Start"),
        Widget::from(Row::new([SizedBox::new().width(60.).height(40.)])),
        text("End"),
    ]);
    let tree = mount(Directionality::new(
        TextDirection::Ltr,
        Center::new(Widget::from(IntrinsicHeight::new(wrap)).with_key(Key::Value(WRAP))),
    ));
    assert_eq!(bounds(&tree, WRAP).size.height, 72.);
}
