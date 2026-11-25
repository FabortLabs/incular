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
