use incular_config::{Alignment, Constraints, EdgeInsets, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Baseline, ConstrainedBox, FractionallySizedBox, Padding, Text, TextStyle, Widget,
    internal::Key,
};

const CHILD: Key = Key::Value(1);

fn marker(real: bool) -> Widget {
    let content = if real {
        Baseline::new(
            40.,
            Text::new("A")
                .text_direction(TextDirection::Ltr)
                .style(TextStyle::default().font_size(4.)),
        )
        .into()
    } else {
        Widget::box_(Size::new(100., 100.), Color::TRANSPARENT)
    };
    Widget::from(ConstrainedBox::new(
        Constraints::tight(Size::new(100., 100.)),
        content,
    ))
    .with_key(CHILD)
}

fn read(runtime: &mut Runtime, constraints: Constraints) -> (Size, Offset, Option<f32>) {
    runtime.run_frame(constraints).unwrap();
    let tree = runtime.tree();
    let wrapper = tree.root().unwrap();
    let child = tree.element_with_key(&CHILD).unwrap();
    let outer = tree.element_bounds(wrapper).unwrap();
    let inner = tree.element_bounds(child).unwrap();
    (
        outer.size,
        inner.origin - outer.origin,
        tree.baseline(tree.render_id(wrapper).unwrap()),
    )
}

#[test]
fn align_offsets_and_clears_the_actual_baseline() {
    let build = |alignment, real| Align::new(alignment, marker(real)).into();
    let mut runtime = Runtime::new(build(Alignment::TOP_LEFT, true)).unwrap();
    let root = runtime.tree().root().unwrap();
    let constraints = Constraints::at_most(200., 200.);
    for (alignment, xy, metric, real) in [
        (Alignment::TOP_LEFT, 0., Some(40.), true),
        (Alignment::CENTER, 50., Some(90.), true),
        (Alignment::BOTTOM_RIGHT, 100., Some(140.), true),
        (Alignment::BOTTOM_RIGHT, 100., None, false),
    ] {
        runtime
            .schedule_update(root, build(alignment, real))
            .unwrap();
        assert_eq!(
            read(&mut runtime, constraints),
            (Size::new(200., 200.), Offset::new(xy, xy), metric)
        );
    }
}

#[test]
fn padding_forwards_the_baseline_plus_top_padding() {
    let build = |real| Padding::new(EdgeInsets::from_ltrb(7., 13., 15., 11.), marker(real)).into();
    let mut runtime = Runtime::new(build(true)).unwrap();
    let constraints = Constraints::at_most(200., 200.);
    assert_eq!(
        read(&mut runtime, constraints),
        (Size::new(122., 124.), Offset::new(7., 13.), Some(53.))
    );
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, build(false)).unwrap();
    assert_eq!(
        read(&mut runtime, constraints),
        (Size::new(122., 124.), Offset::new(7., 13.), None)
    );
}

#[test]
fn fractional_box_uses_the_placed_baseline_after_parent_minimums() {
    let view = FractionallySizedBox::new(marker(true))
        .width_factor(0.5)
        .height_factor(0.5);
    let mut runtime = Runtime::new(view.into()).unwrap();
    assert_eq!(
        read(&mut runtime, Constraints::new(0., 200., 160., 200.)),
        (Size::new(100., 160.), Offset::new(0., 30.), Some(70.))
    );
}
