use incular_config::{
    Constraints, CrossAxisAlignment, MainAxisAlignment, MainAxisSize, TextDirection,
    VerticalDirection,
};
use incular_core::{Color, Size};
use incular_runtime::Runtime;
use incular_text::TextStyle;
use incular_widgets::{Baseline, ConstrainedBox, Flex, Flexible, Text, Widget, internal::Key};

fn marker(height: f32, baseline: f32) -> Widget {
    ConstrainedBox::new(
        Constraints::tight(Size::new(100., height)),
        Baseline::new(
            baseline,
            Text::new("A")
                .text_direction(TextDirection::Ltr)
                .style(TextStyle {
                    size: 4.,
                    ..Default::default()
                }),
        ),
    )
    .into()
}

fn plain(height: f32) -> Widget {
    Widget::box_(Size::new(100., height), Color::TRANSPARENT)
}

fn outer(child: impl Into<Widget>, baseline: f32) -> Widget {
    Flex::horizontal([child.into().with_key(Key::Value(1)), marker(100., baseline)])
        .cross_axis_alignment(CrossAxisAlignment::Baseline)
        .text_direction(TextDirection::Ltr)
        .into()
}

fn nested_y(runtime: &mut Runtime) -> f32 {
    runtime.run_frame(Constraints::at_most(800., 600.)).unwrap();
    let tree = runtime.tree();
    let nested = tree.element_with_key(&Key::Value(1)).unwrap();
    tree.element_bounds(nested).unwrap().origin.y
}

#[test]
fn a_nested_column_reports_its_first_real_baseline_in_logical_child_order() {
    let build = |main, vertical| {
        outer(
            Flex::vertical([
                plain(100.),
                marker(100., 10.),
                plain(100.),
                marker(100., 10.),
                plain(100.),
            ])
            .main_axis_alignment(main)
            .vertical_direction(vertical)
            .text_direction(TextDirection::Ltr),
            500.,
        )
    };
    let states = [
        (MainAxisAlignment::Start, VerticalDirection::Down, 110.),
        (MainAxisAlignment::End, VerticalDirection::Down, 210.),
        (MainAxisAlignment::End, VerticalDirection::Up, 310.),
        (MainAxisAlignment::Start, VerticalDirection::Up, 410.),
    ];
    let mut runtime = Runtime::new(build(states[0].0, states[0].1)).unwrap();
    let root = runtime.tree().root().unwrap();
    let mut measured = Vec::new();
    for (main, vertical, expected) in states {
        runtime
            .schedule_update(root, build(main, vertical))
            .unwrap();
        measured.push((500. - nested_y(&mut runtime), expected));
    }
    assert_eq!(
        measured,
        [(110., 110.), (210., 210.), (310., 310.), (410., 410.)]
    );
}

#[test]
fn a_nested_row_reports_the_highest_positioned_baseline_and_clears_it_on_update() {
    let build = |real| {
        let (tall, short) = if real {
            (marker(100., 40.), marker(60., 5.))
        } else {
            (plain(100.), plain(60.))
        };
        outer(
            Flex::horizontal([tall, Flexible::new(short).into()])
                .main_axis_size(MainAxisSize::Min)
                .text_direction(TextDirection::Ltr),
            80.,
        )
    };
    let mut runtime = Runtime::new(build(true)).unwrap();
    let initial = nested_y(&mut runtime);
    let root = runtime.tree().root().unwrap();
    let nested = runtime.tree().element_with_key(&Key::Value(1)).unwrap();
    runtime.schedule_update(root, build(false)).unwrap();
    let cleared = nested_y(&mut runtime);
    assert_eq!(
        runtime.tree().element_with_key(&Key::Value(1)),
        Some(nested)
    );
    // The short child's baseline is at (100 - 60)/2 + 5 = 25, higher
    // than the first child's 40px. When both are removed, the row top-aligns.
    assert_eq!([initial, cleared], [55., 0.]);
}

#[test]
fn row_unions_real_multiline_font_extents_and_respects_a_larger_baselineless_child() {
    use incular_config::Alignment;
    use incular_widgets::{Align, Row, SizedBox};

    let Some(path) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FONT: &str = "Pinned FlutterTest for row baseline";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let text = |value, size, key| {
        Widget::from(
            Text::new(value).text_direction(TextDirection::Ltr).style(
                TextStyle::new()
                    .font_family(FONT)
                    .font_size(size)
                    .height(Some(1.)),
            ),
        )
        .with_key(key)
    };
    let scene = |tall_child| {
        let mut children = vec![
            text("big text", 52., "large"),
            text("one\ntwo\nthree\nfour\nfive\nsix\nseven", 12., "multiline"),
        ];
        if tall_child {
            children.push(Widget::from(SizedBox::square(250.)).with_key("baselineless"));
        }
        let row = Widget::from(
            Row::new(children)
                .cross_axis_alignment(CrossAxisAlignment::Baseline)
                .text_direction(TextDirection::Ltr),
        )
        .with_key("row");
        Widget::from(Align::new(Alignment::TOP_LEFT, row))
    };
    let mut runtime = Runtime::new(scene(false)).unwrap();
    let root = runtime.tree().root().unwrap();
    for (tall_child, row_height) in [(false, 114.), (true, 250.), (false, 114.)] {
        runtime.schedule_update(root, scene(tall_child)).unwrap();
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let bounds = |key| {
            let tree = runtime.tree();
            tree.element_bounds(tree.element_with_key(&Key::from(key)).unwrap())
                .unwrap()
        };
        let row = bounds("row");
        let large = bounds("large");
        let multiline = bounds("multiline");
        assert_eq!(
            row.size.height, row_height,
            "{row:?} {large:?} {multiline:?}"
        );
        assert_eq!((large.size.height, multiline.size.height), (52., 84.));
        assert_eq!((large.origin.y, multiline.origin.y), (0., 30.));
        if tall_child {
            assert_eq!(bounds("baselineless").origin.y, 0.);
        }
    }
}
