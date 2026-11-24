//! Pinned Flutter Stack and IndexedStack actual alphabetic baselines.
use incular_config::{Alignment, Constraints};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Baseline, Center, IndexedStack, Positioned, SizedBox, Stack, Text, TextDirection, TextStyle,
    Widget, internal::Key,
};

const OUTER: u64 = 1;
const STACK: u64 = 2;
const TEXT: u64 = 3;
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn large_box() -> Widget {
    SizedBox::new().width(200.).height(200.).into()
}

fn text(key: u64) -> Widget {
    Widget::from(Text::new("X").text_direction(TextDirection::Ltr)).with_key(Key::Value(key))
}

fn geometry(runtime: &Runtime, key: u64) -> (Size, Option<f32>) {
    let tree = runtime.tree();
    let element = tree.element_with_key(&Key::Value(key)).unwrap();
    let size = tree.element_bounds(element).unwrap().size;
    let baseline = tree.baseline(tree.render_id(element).unwrap());
    (size, baseline)
}

#[test]
fn indexed_stack_updates_only_the_selected_actual_baseline() {
    let leaf = Widget::from(
        Text::new("X")
            .text_direction(TextDirection::Ltr)
            .style(TextStyle::default().font_size(100.).height(Some(1.))),
    )
    .with_key(Key::Value(TEXT));
    let sibling = large_box();
    let make = |index| {
        let stack = Widget::from(
            IndexedStack::new([leaf.clone(), sibling.clone()])
                .alignment(Alignment::CENTER)
                .index(index),
        )
        .with_key(Key::Value(STACK));
        Center::new(Widget::from(Baseline::new(300., stack)).with_key(Key::Value(OUTER))).into()
    };
    let mut r = Runtime::new(make(Some(0))).unwrap();
    let root = r.tree().root().unwrap();
    let mut geometry_by_index = Vec::new();
    for (step, index) in [Some(0), None, Some(1), Some(0)].into_iter().enumerate() {
        if step != 0 {
            r.schedule_update(root, make(index)).unwrap();
        }
        r.run_frame(Constraints::tight(VIEW)).unwrap();
        let (leaf_size, leaf_baseline) = geometry(&r, TEXT);
        let (stack_size, stack_baseline) = geometry(&r, STACK);
        let wanted = (index == Some(0))
            .then(|| (200. - leaf_size.height) / 2. + leaf_baseline.expect("text actual baseline"));
        assert_eq!(stack_size, Size::new(200., 200.));
        geometry_by_index.push((index, stack_baseline, wanted, geometry(&r, OUTER).0.height));
    }
    for (index, baseline, wanted, outer_height) in geometry_by_index {
        assert_eq!(baseline, wanted, "selection {index:?}");
        assert_eq!(
            outer_height,
            500. - wanted.unwrap_or(200.),
            "selection {index:?}"
        );
    }
}

#[test]
fn plain_stack_chooses_the_highest_actual_baseline_after_positioning() {
    const LOWER: u64 = 4;
    let child = Widget::from(Stack::new([
        large_box(),
        Positioned::new(text(LOWER)).top(29.).into(),
        Positioned::new(text(TEXT)).top(3.).into(),
    ]))
    .with_key(Key::Value(STACK));
    let mut r = Runtime::new(Center::new(child).into()).unwrap();
    r.run_frame(Constraints::tight(VIEW)).unwrap();
    let higher = geometry(&r, TEXT).1.unwrap() + 3.;
    let farther = geometry(&r, LOWER).1.unwrap() + 29.;
    assert!(farther > higher);
    assert_eq!(geometry(&r, STACK).1, Some(higher));
}
