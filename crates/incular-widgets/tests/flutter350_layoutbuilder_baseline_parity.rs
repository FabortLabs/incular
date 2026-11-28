//! Flutter 3.50.0 Baseline asks through LayoutBuilder for the laid-out child.
use incular_config::Constraints;
use incular_core::{Rect, Size};
use incular_widgets::{
    Baseline, Center, LayoutBuilder, SizedBox, Text, TextDirection, TextStyle, Widget,
    internal::{Key, WidgetTree},
};

const LEAF: u64 = 1;
const HOST: u64 = 2;

fn text_with_baseline(target: f32) -> Widget {
    Baseline::new(
        target,
        Text::new("X")
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::default().font_size(4.)),
    )
    .into()
}

fn frame(leaf: Widget, generated: bool) -> (f32, Option<f32>) {
    let leaf = leaf.with_key(Key::Value(LEAF));
    let child = if generated {
        Widget::from(LayoutBuilder::new(move |_, _| leaf.clone()))
    } else {
        leaf
    };
    let mut tree = WidgetTree::new();
    let outer = Widget::from(Baseline::new(180., child)).with_key(Key::Value(HOST));
    tree.mount(Center::new(outer).into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let element = |key| tree.element_with_key(&Key::Value(key)).unwrap();
    let bounds = |key| -> Rect { tree.element_bounds(element(key)).unwrap() };
    let reported = tree.baseline(tree.render_id(element(LEAF)).unwrap());
    (bounds(LEAF).origin.y - bounds(HOST).origin.y, reported)
}

#[test]
fn forwards_the_childs_actual_alphabetic_baseline() {
    let text = Widget::from(
        Text::new("X")
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::default().font_size(100.).height(Some(1.))),
    );
    let (direct, baseline) = frame(text.clone(), false);
    let floating = baseline.expect("text has a real metric");
    assert!((direct - (180. - floating)).abs() < 0.001);
    let (through, retained) = frame(text, true);
    assert_eq!(retained, Some(floating));
    assert!(
        (through - direct).abs() < 0.001,
        "wrapper local top {through}; direct local top {direct}; child baseline {floating}"
    );
}
#[test]
fn a_box_without_baseline_keeps_the_ordinary_bottom_edge_fallback() {
    let leaf = Widget::from(SizedBox::new().width(60.).height(100.));
    assert_eq!(frame(leaf.clone(), false), (80., None));
    assert_eq!(frame(leaf, true), (80., None));
}

#[test]
fn original_twenty_pixel_baseline_places_the_generated_child_at_160() {
    // Like Flutter's detector, this leaf reports an actual baseline at 20.
    assert_eq!(frame(text_with_baseline(20.), true), (160., Some(20.)));
}

#[test]
fn regenerated_child_replaces_or_clears_the_cached_actual_baseline() {
    let mut tree = WidgetTree::new();
    let make = |actual: Option<f32>| {
        let generated = LayoutBuilder::new(move |_, _| {
            let box_ = SizedBox::new().width(10.).height(100.);
            let child = match actual {
                Some(value) => text_with_baseline(value),
                None => Widget::from(box_),
            };
            child.with_key(Key::Value(LEAF))
        });
        Center::new(Widget::from(Baseline::new(180., generated)).with_key(Key::Value(HOST))).into()
    };
    let root = tree.mount(make(Some(20.))).unwrap();
    let viewport = Constraints::tight(Size::new(800., 600.));
    for (step, (actual, expected)) in [(Some(20.), 160.), (None, 80.), (Some(30.), 150.)]
        .into_iter()
        .enumerate()
    {
        if step != 0 {
            tree.update(root, make(actual)).unwrap();
        }
        tree.layout(viewport).unwrap();
        let y = |key| {
            let element = tree.element_with_key(&Key::Value(key)).unwrap();
            tree.element_bounds(element).unwrap().origin.y
        };
        assert_eq!(y(LEAF) - y(HOST), expected);
    }
}

fn measurable_mock() -> Widget {
    SizedBox::new()
        .width(50.)
        .height(50.)
        .child(text_with_baseline(40.))
        .into()
}

fn dry_at(widget: impl Into<Widget>, constraints: Constraints) -> Option<f32> {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget.into()).unwrap();
    tree.dry_alphabetic_baseline(tree.render_id(root).unwrap(), constraints)
}

#[test]
fn shifted_dry_align_baselines_match_the_mock_size_and_alignment() {
    use incular_config::Alignment;
    use incular_widgets::Align;

    let maximum = Constraints::loose(Size::new(200., 200.));
    for (alignment, expected) in [
        (Alignment::CENTER, 115.),
        (Alignment::TOP_LEFT, 40.),
        (Alignment::BOTTOM_RIGHT, 190.),
    ] {
        assert_eq!(
            dry_at(Align::new(alignment, measurable_mock()), maximum),
            Some(expected)
        );
    }
    assert_eq!(
        dry_at(Align::new(Alignment::CENTER, SizedBox::shrink()), maximum),
        None
    );
    assert_eq!(
        dry_at(
            Align::new(Alignment::TOP_LEFT, measurable_mock())
                .width_factor(2.)
                .height_factor(1.5),
            maximum,
        ),
        Some(40.)
    );
}

#[test]
fn shifted_dry_align_baselines_survive_infinite_small_and_zero_bounds() {
    use incular_config::Alignment;
    use incular_widgets::Align;

    assert_eq!(
        dry_at(
            Align::new(Alignment::TOP_LEFT, measurable_mock()),
            Constraints::unbounded(),
        ),
        Some(40.)
    );
    for constraints in [
        Constraints::loose(Size::new(1., 1.)),
        Constraints::tight(Size::ZERO),
    ] {
        assert_eq!(
            dry_at(
                Align::new(Alignment::CENTER, measurable_mock()),
                constraints
            ),
            Some(40.)
        );
    }
}

#[test]
fn shifted_dry_baselines_use_the_current_child_constraints_not_the_wet_offset() {
    use incular_config::Alignment;
    use incular_widgets::{FractionallySizedBox, OverflowBox, Padding, SizedOverflowBox};

    let small = Constraints::loose(Size::new(100., 100.));
    let large = Constraints::loose(Size::new(200., 200.));
    let left = Alignment::TOP_LEFT;
    assert_eq!(
        dry_at(OverflowBox::new(measurable_mock()).alignment(left), small),
        Some(40.)
    );
    assert_eq!(
        dry_at(
            SizedOverflowBox::new(Size::new(100., 100.), measurable_mock()).alignment(left),
            large
        ),
        Some(40.)
    );
    assert_eq!(
        dry_at(
            FractionallySizedBox::new(measurable_mock())
                .width_factor(0.5)
                .height_factor(0.5)
                .alignment(left),
            large,
        ),
        Some(40.)
    );
    assert_eq!(
        dry_at(Padding::all(10., measurable_mock()), small),
        Some(50.)
    );
    assert_eq!(
        dry_at(Baseline::new(30., measurable_mock()), small),
        Some(30.)
    );
    assert_eq!(
        dry_at(Baseline::new(-10., measurable_mock()), small),
        Some(-10.)
    );
    assert_eq!(dry_at(Baseline::new(30., SizedBox::shrink()), small), None);
}

#[test]
fn querying_a_baseline_at_different_bounds_keeps_the_previous_laid_out_geometry() {
    use incular_config::Alignment;
    use incular_widgets::Align;

    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Align::new(Alignment::BOTTOM_RIGHT, measurable_mock()).into())
        .unwrap();
    let render = tree.render_id(root).unwrap();
    let first = Constraints::tight(Size::new(100., 100.));
    tree.layout(first).unwrap();
    let original = tree.element_bounds(root).unwrap();
    let wet_baseline = tree.baseline(render);
    let layouts = tree.diagnostics().layouts;
    let second = Constraints::tight(Size::new(200., 200.));
    assert_eq!(tree.dry_alphabetic_baseline(render, second), Some(190.));
    assert_eq!(tree.element_bounds(root).unwrap(), original);
    assert_eq!(tree.baseline(render), wet_baseline);
    assert_eq!(tree.diagnostics().layouts, layouts);
}

#[test]
fn a_childless_align_answers_the_dry_query_and_expands_only_unfactored_bounded_axes() {
    use incular_config::Alignment;
    use incular_widgets::Align;

    let size = Constraints::loose(Size::new(100., 100.));
    let mut tree = WidgetTree::new();
    let empty = |alignment| Widget::from(Align::empty(alignment));
    let root = tree.mount(empty(Alignment::CENTER)).unwrap();
    for alignment in [Alignment::CENTER, Alignment::TOP_LEFT] {
        tree.update(root, empty(alignment)).unwrap();
        let render = tree.render_id(root).unwrap();
        assert!(tree.children(root).unwrap().is_empty());
        assert_eq!(tree.dry_alphabetic_baseline(render, size), None);
        tree.layout(size).unwrap();
        assert_eq!(tree.render_size(render), Some(Size::new(100., 100.)));
    }

    tree.update(
        root,
        Align::empty(Alignment::CENTER).height_factor(1.5).into(),
    )
    .unwrap();
    tree.layout(size).unwrap();
    assert_eq!(
        tree.render_size(tree.render_id(root).unwrap()),
        Some(Size::new(100., 0.))
    );
}

#[test]
fn the_current_paragraph_based_child_answers_both_dry_baseline_policies() {
    use incular_config::Alignment;
    use incular_text::TextBaseline;
    use incular_widgets::Align;

    let mut tree = WidgetTree::new();
    let widget = Align::new(Alignment::TOP_LEFT, measurable_mock());
    let root = tree.mount(widget.into()).unwrap();
    let render = tree.render_id(root).unwrap();
    let constraints = Constraints::loose(Size::new(100., 100.));
    assert_eq!(
        tree.dry_baseline(render, constraints, TextBaseline::Alphabetic),
        Some(40.)
    );
    assert_eq!(
        tree.dry_baseline(render, constraints, TextBaseline::Ideographic),
        Some(40.)
    );
}
