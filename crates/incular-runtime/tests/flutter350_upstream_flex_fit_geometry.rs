use incular_config::{Constraints, FlexFit, MainAxisAlignment, MainAxisSize, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Flex, Flexible, SizedBox, Widget, internal::Key};

fn build(main_size: MainAxisSize, flex: u32, fit: FlexFit, desired_width: f32) -> Widget {
    let child = |index, width| {
        Widget::from(SizedBox::from_size(Size::new(width, 100.))).with_key(Key::Value(index))
    };
    Flex::horizontal([
        Flexible::flex_factor(flex, child(0, desired_width))
            .fit(fit)
            .into(),
        child(1, 100.),
        child(2, 100.),
    ])
    .main_axis_size(main_size)
    .main_axis_alignment(MainAxisAlignment::SpaceBetween)
    .text_direction(TextDirection::Ltr)
    .into()
}

fn frame(runtime: &mut Runtime) -> (f32, [(f32, f32); 3]) {
    runtime.run_frame(Constraints::at_most(500., 400.)).unwrap();
    let tree = runtime.tree();
    let size = tree.element_bounds(tree.root().unwrap()).unwrap().size;
    let children = [0, 1, 2].map(|i| {
        let bounds = tree
            .element_bounds(tree.element_with_key(&Key::Value(i)).unwrap())
            .unwrap();
        assert_eq!(bounds.size.height, 100.);
        (bounds.origin.x, bounds.size.width)
    });
    (size.width, children)
}

#[test]
fn min_flex_uses_bounded_free_space_before_shrinking_to_actual_children() {
    let mut runtime = Runtime::new(build(MainAxisSize::Min, 0, FlexFit::Loose, 100.)).unwrap();
    assert_eq!(
        frame(&mut runtime),
        (300., [(0., 100.), (100., 100.), (200., 100.)])
    );
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, build(MainAxisSize::Min, 1, FlexFit::Tight, 100.))
        .unwrap();
    let tight = frame(&mut runtime);
    runtime
        .schedule_update(root, build(MainAxisSize::Min, 1, FlexFit::Loose, 100.))
        .unwrap();
    let loose = frame(&mut runtime);
    assert_eq!(
        [tight, loose],
        [
            (500., [(0., 300.), (300., 100.), (400., 100.)]),
            (300., [(0., 100.), (100., 100.), (200., 100.)]),
        ],
    );
}

#[test]
fn loose_fit_keeps_natural_size_until_the_child_reaches_its_allocation() {
    let mut runtime = Runtime::new(build(MainAxisSize::Max, 0, FlexFit::Loose, 100.)).unwrap();
    let initial = (500., [(0., 100.), (200., 100.), (400., 100.)]);
    assert_eq!(frame(&mut runtime), initial);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, build(MainAxisSize::Max, 1, FlexFit::Loose, 100.))
        .unwrap();
    assert_eq!(frame(&mut runtime), initial);
    runtime
        .schedule_update(root, build(MainAxisSize::Max, 1, FlexFit::Loose, 1000.))
        .unwrap();
    assert_eq!(
        frame(&mut runtime),
        (500., [(0., 300.), (300., 100.), (400., 100.)])
    );
}

fn in_overflow_box(max_width: f32, main_size: MainAxisSize, flex: u32, fit: FlexFit) -> Widget {
    let child = || Widget::from(SizedBox::from_size(Size::new(100., 100.)));
    let layout = Widget::from(
        Flex::horizontal([
            child(),
            Flexible::flex_factor(flex, child()).fit(fit).into(),
            child(),
        ])
        .main_axis_size(main_size)
        .text_direction(TextDirection::Ltr),
    )
    .with_key(Key::Value(20));
    incular_widgets::OverflowBox::new(layout)
        .min_width(0.)
        .max_width(max_width)
        .min_height(0.)
        .max_height(400.)
        .into()
}

fn measure_nested_flex(runtime: &mut Runtime) -> Size {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let tree = runtime.tree();
    let flex = tree.element_with_key(&Key::Value(20)).unwrap();
    tree.element_bounds(flex).unwrap().size
}

#[test]
fn min_loose_flex_survives_unbounded_to_bounded_layout() {
    let states = [
        (f32::INFINITY, MainAxisSize::Min, 0, FlexFit::Loose, 300.),
        (f32::INFINITY, MainAxisSize::Min, 1, FlexFit::Loose, 300.),
        (500., MainAxisSize::Min, 1, FlexFit::Loose, 300.),
        (500., MainAxisSize::Max, 1, FlexFit::Loose, 500.),
        (500., MainAxisSize::Min, 1, FlexFit::Tight, 500.),
        (505., MainAxisSize::Min, 1, FlexFit::Tight, 505.),
    ];
    let (width, main, flex, fit, expected) = states[0];
    let mut runtime = Runtime::new(in_overflow_box(width, main, flex, fit)).unwrap();
    assert_eq!(measure_nested_flex(&mut runtime), Size::new(expected, 100.));
    let root = runtime.tree().root().unwrap();
    let flex_id = runtime.tree().element_with_key(&Key::Value(20)).unwrap();
    for (width, main, factor, fit, expected) in states.into_iter().skip(1) {
        runtime
            .schedule_update(root, in_overflow_box(width, main, factor, fit))
            .unwrap();
        assert_eq!(measure_nested_flex(&mut runtime), Size::new(expected, 100.));
        assert_eq!(
            runtime.tree().element_with_key(&Key::Value(20)),
            Some(flex_id)
        );
    }
}

#[test]
fn unsupported_unbounded_flex_modes_use_documented_intrinsic_fallback() {
    // The equivalent Flutter debug configurations emit a layout error.
    // Incular keeps a finite intrinsic layout rather than failing the frame.
    for (main, fit) in [
        (MainAxisSize::Min, FlexFit::Tight),
        (MainAxisSize::Max, FlexFit::Loose),
    ] {
        let mut runtime = Runtime::new(in_overflow_box(f32::INFINITY, main, 1, fit)).unwrap();
        assert_eq!(measure_nested_flex(&mut runtime), Size::new(300., 100.));
    }
}
