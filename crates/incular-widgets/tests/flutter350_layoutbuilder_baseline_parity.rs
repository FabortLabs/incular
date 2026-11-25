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
