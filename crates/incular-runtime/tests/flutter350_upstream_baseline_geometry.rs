use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Size};
use incular_runtime::Runtime;
use incular_widgets::{Baseline, Text, TextStyle, Widget, internal::Key};

const CHILD: Key = Key::Value(1);

fn snapshot(runtime: &mut Runtime) -> (Size, Offset, Size, Option<f32>) {
    runtime.run_frame(Constraints::at_most(800., 600.)).unwrap();
    let tree = runtime.tree();
    let baseline = tree.root().unwrap();
    let child = tree.element_with_key(&CHILD).unwrap();
    let outer = tree.element_bounds(baseline).unwrap();
    let inner = tree.element_bounds(child).unwrap();
    (
        outer.size,
        inner.origin - outer.origin,
        inner.size,
        tree.baseline(tree.render_id(baseline).unwrap()),
    )
}

#[test]
fn baseline_repositions_a_baselineless_child_using_its_bottom_edge() {
    let build = |value| {
        Widget::from(Baseline::new(
            value,
            Widget::box_(Size::new(100., 100.), Color::WHITE).with_key(CHILD),
        ))
    };
    let mut runtime = Runtime::new(build(0.)).unwrap();
    let root = runtime.tree().root().unwrap();
    let states = [
        (0., -100.),
        (25., -75.),
        (90., -10.),
        (100., 0.),
        (110., 10.),
    ];
    for (value, y) in states {
        runtime.schedule_update(root, build(value)).unwrap();
        assert_eq!(
            snapshot(&mut runtime),
            (
                Size::new(100., value),
                Offset::new(0., y),
                Size::new(100., 100.),
                None
            ),
            "baseline {value}",
        );
    }
}

#[test]
fn a_negative_target_moves_a_real_baseline_above_the_parent() {
    let child = Widget::from(
        Text::new("A")
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::default().font_size(16.)),
    )
    .with_key(CHILD);
    let mut runtime = Runtime::new(Baseline::new(40., child.clone()).into()).unwrap();
    let root = runtime.tree().root().unwrap();
    let (_, _, original_size, _) = snapshot(&mut runtime);
    let tree = runtime.tree();
    let text = tree.element_with_key(&CHILD).unwrap();
    let real = tree.baseline(tree.render_id(text).unwrap()).unwrap();
    runtime
        .schedule_update(root, Baseline::new(-5., child).into())
        .unwrap();
    let (size, offset, child_size, reported) = snapshot(&mut runtime);
    assert_eq!(size.width, original_size.width);
    assert_eq!(child_size, original_size);
    assert_eq!(offset, Offset::new(0., -5. - real));
    assert_eq!(size.height, (child_size.height + offset.y).max(0.));
    assert!((reported.unwrap() + 5.).abs() < 1e-5);

    runtime
        .schedule_update(
            root,
            Baseline::new(
                -5.,
                Widget::box_(Size::new(100., 100.), Color::WHITE).with_key(CHILD),
            )
            .into(),
        )
        .unwrap();
    assert_eq!(
        snapshot(&mut runtime),
        (
            Size::new(100., 0.),
            Offset::new(0., -105.),
            Size::new(100., 100.),
            None
        )
    );
}
