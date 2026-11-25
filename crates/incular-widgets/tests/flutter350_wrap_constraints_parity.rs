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

const DISTRIBUTION: [WrapAlignment; 4] = [
    WrapAlignment::Center,
    WrapAlignment::SpaceBetween,
    WrapAlignment::SpaceAround,
    WrapAlignment::SpaceEvenly,
];

fn main_axis(direction: TextDirection, expected: [[f32; 3]; 4]) {
    let build = |step: usize| {
        key(wrap(&[
            (100., 10.),
            (200., 20.),
            (if step < 2 { 300. } else { 310. }, 30.),
        ])
        .text_direction(direction)
        .spacing(5.)
        .alignment(DISTRIBUTION[step]))
    };
    let mut tree = mount(build(0));
    for (step, xs) in expected.into_iter().enumerate() {
        let root = tree.root().unwrap();
        if step > 0 {
            tree.update(root, build(step)).unwrap();
            tree.layout(Constraints::tight(HOST)).unwrap();
        }
        check(&tree, HOST, &xs.map(|x| (x, 0.)));
    }
}
#[test]
fn main_alignment_geometry_left_to_right() {
    main_axis(
        TextDirection::Ltr,
        [
            [95., 200., 405.],
            [0., 200., 500.],
            [30., 195., 460.],
            [45., 195., 445.],
        ],
    );
}
#[test]
fn main_alignment_geometry_right_to_left() {
    main_axis(
        TextDirection::Rtl,
        [
            [605., 400., 95.],
            [700., 400., 0.],
            [670., 405., 30.],
            [655., 405., 45.],
        ],
    );
}

fn run_axis(direction: incular_config::VerticalDirection, expected: [[f32; 5]; 4]) {
    let build = |step: usize| {
        key(wrap(&[
            (100., 10.),
            (200., 20.),
            (300., 30.),
            (400., 40.),
            (500., if step == 2 { 70. } else { 60. }),
        ])
        .vertical_direction(direction)
        .run_spacing(5.)
        .run_alignment(DISTRIBUTION[step]))
    };
    let mut tree = mount(build(0));
    let x = [0., 100., 300., 0., 0.];
    for (step, ys) in expected.into_iter().enumerate() {
        let root = tree.root().unwrap();
        if step > 0 {
            tree.update(root, build(step)).unwrap();
            tree.layout(Constraints::tight(HOST)).unwrap();
        }
        let offsets = (0..5).map(|i| (x[i], ys[i])).collect::<Vec<_>>();
        check(&tree, HOST, &offsets);
    }
}
#[test]
fn run_distribution_down_aligns_the_rows() {
    run_axis(
        incular_config::VerticalDirection::Down,
        [
            [230., 230., 230., 265., 310.],
            [0., 0., 0., 265., 540.],
            [75., 75., 75., 260., 455.],
            [115., 115., 115., 265., 425.],
        ],
    );
}
#[test]
fn run_distribution_up_mirrors_runs_and_start_edges_inside_each_run() {
    run_axis(
        incular_config::VerticalDirection::Up,
        [
            [360., 350., 340., 295., 230.],
            [590., 580., 570., 295., 0.],
            [515., 505., 495., 300., 75.],
            [475., 465., 455., 295., 115.],
        ],
    );
}
#[test]
fn vertical_intrinsic_width_sums_three_columns() {
    use incular_widgets::IntrinsicWidth;
    // The Flutter fixture's Ahem text is 80 and 48 wide. Use the same
    // measured width as fixed box inputs without changing the font stack.
    let ahem_box = |width| SizedBox::new().width(width).height(16.);
    let w = Wrap::new([
        Widget::from(ahem_box(80.)),
        Widget::from(Column::new([SizedBox::new().width(60.).height(40.)])),
        Widget::from(ahem_box(48.)),
    ])
    .direction(incular_config::Axis::Vertical);
    let tree = mount(Directionality::new(
        TextDirection::Ltr,
        Center::new(Widget::from(IntrinsicWidth::new(w)).with_key(Key::Value(WRAP))),
    ));
    assert_eq!(bounds(&tree, WRAP).size.width, 188.);
}

fn basic(reversed: bool) {
    use incular_config::VerticalDirection;
    let uniform = [(300., 100.); 4];
    let uneven = [(300., 50.), (300., 100.), (300., 100.), (300., 50.)];
    let dir = if reversed {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    };
    let build = |step: usize| {
        if step < 3 {
            key(wrap(&uniform).text_direction(dir).alignment(
                [
                    WrapAlignment::Start,
                    WrapAlignment::Center,
                    WrapAlignment::End,
                ][step],
            ))
        } else {
            key(wrap(&uneven)
                .cross_axis_alignment(
                    [
                        WrapCrossAlignment::Start,
                        WrapCrossAlignment::Center,
                        WrapCrossAlignment::End,
                    ][step - 3],
                )
                .vertical_direction(if reversed {
                    VerticalDirection::Up
                } else {
                    VerticalDirection::Down
                }))
        }
    };
    let expected = if reversed {
        [
            [(500., 0.), (200., 0.), (500., 100.), (200., 100.)],
            [(400., 0.), (100., 0.), (400., 100.), (100., 100.)],
            [(300., 0.), (0., 0.), (300., 100.), (0., 100.)],
            [(0., 550.), (300., 500.), (0., 400.), (300., 450.)],
            [(0., 525.), (300., 500.), (0., 400.), (300., 425.)],
            [(0., 500.), (300., 500.), (0., 400.), (300., 400.)],
        ]
    } else {
        [
            [(0., 0.), (300., 0.), (0., 100.), (300., 100.)],
            [(100., 0.), (400., 0.), (100., 100.), (400., 100.)],
            [(200., 0.), (500., 0.), (200., 100.), (500., 100.)],
            [(0., 0.), (300., 0.), (0., 100.), (300., 100.)],
            [(0., 25.), (300., 0.), (0., 100.), (300., 125.)],
            [(0., 50.), (300., 0.), (0., 100.), (300., 150.)],
        ]
    };
    let mut tree = mount(build(0));
    let root = tree.root().unwrap();
    for (i, point) in expected.iter().enumerate() {
        if i > 0 {
            tree.update(root, build(i)).unwrap();
            tree.layout(Constraints::tight(HOST)).unwrap();
        }
        check(&tree, HOST, point);
    }
}
#[test]
fn basic_wrap_offsets_ltr_and_cross_axis_down() {
    basic(false);
}
#[test]
fn basic_wrap_offsets_rtl_and_cross_axis_up() {
    basic(true);
}

#[test]
fn empty_centered_wrap_has_zero_size() {
    let tree = mount(Center::new(key(
        Wrap::new(Vec::<Widget>::new()).alignment(WrapAlignment::Center)
    )));
    assert_eq!(bounds(&tree, WRAP).size, Size::ZERO);
}

#[test]
fn wrap_propagates_its_child_text_baseline() {
    use incular_widgets::Baseline;
    let text = Widget::from(
        Text::new("X")
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::default().font_size(100.).height(Some(1.))),
    )
    .with_key(Key::Value(0));
    let direct = mount(Center::new(
        Widget::from(Baseline::new(175., text.clone())).with_key(Key::Value(WRAP)),
    ));
    let tree = mount(Center::new(
        Widget::from(Baseline::new(
            175.,
            Wrap::new([text]).text_direction(TextDirection::Ltr),
        ))
        .with_key(Key::Value(WRAP)),
    ));
    let glyph = bounds(&tree, 0).size;
    let text_id = tree.element_with_key(&Key::Value(0)).unwrap();
    let text_baseline = tree
        .baseline(tree.render_id(text_id).unwrap())
        .expect("actual text baseline control");
    let outer = bounds(&tree, WRAP).size;
    assert_eq!(outer.width, glyph.width);
    let expected = 175. + glyph.height - text_baseline;
    assert!(
        (bounds(&direct, WRAP).size.height - expected).abs() < 0.01,
        "direct text positive control"
    );
    assert!(
        (outer.height - expected).abs() < 0.01,
        "outer height {} expected {} using text height {} and baseline {}",
        outer.height,
        expected,
        glyph.height,
        text_baseline
    );
}

#[test]
fn wrap_uses_the_topmost_actual_baseline_across_reversed_runs() {
    use incular_config::VerticalDirection;
    let line = |key, height, font_size| {
        Widget::from(
            SizedBox::new().width(80.).height(height).child(
                Text::new("X")
                    .text_direction(TextDirection::Ltr)
                    .style(TextStyle::default().font_size(font_size).height(Some(1.))),
            ),
        )
        .with_key(Key::Value(key))
    };
    let scene = Align::new(
        Alignment::TOP_LEFT,
        SizedBox::new()
            .width(100.)
            .child(key(Wrap::new([line(0, 40., 30.), line(1, 20., 12.)])
                .text_direction(TextDirection::Ltr)
                .vertical_direction(VerticalDirection::Up)
                .run_spacing(7.))),
    );
    let mut t = mount(scene);
    let baseline = |tree: &WidgetTree, i| {
        let element = tree.element_with_key(&Key::Value(i)).unwrap();
        tree.baseline(tree.render_id(element).unwrap())
    };
    let topmost = baseline(&t, 1).unwrap() + bounds(&t, 1).origin.y - bounds(&t, WRAP).origin.y;
    let logical_first =
        baseline(&t, 0).unwrap() + bounds(&t, 0).origin.y - bounds(&t, WRAP).origin.y;
    assert!(topmost < logical_first && bounds(&t, 0).origin.y == 27.);
    assert_eq!(baseline(&t, WRAP), Some(topmost));

    let wrap = t.element_with_key(&Key::Value(WRAP)).unwrap();
    t.update(wrap, key(wrap_without_text())).unwrap();
    t.layout(Constraints::tight(HOST)).unwrap();
    assert_eq!(baseline(&t, WRAP), None);
}

fn wrap_without_text() -> Wrap {
    Wrap::new([SizedBox::new().width(80.).height(20.)]).text_direction(TextDirection::Ltr)
}
