use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{
    Center, Directionality, IgnoreBaseline, IntrinsicHeight, Placeholder, PositionedDirectional,
    Stack, Text, Widget,
};

const WRAPPER: Key = Key::Value(320);
const TEXT: Key = Key::Value(321);

fn keyed(widget: impl Into<Widget>, key: Key) -> Widget {
    widget.into().with_key(key)
}

fn metrics(tree: &WidgetTree, key: &Key) -> (Size, Option<f32>) {
    let element = tree.element_with_key(key).unwrap();
    let size = tree.element_bounds(element).unwrap().size;
    let baseline = tree.baseline(tree.render_id(element).unwrap());
    (size, baseline)
}

#[test]
fn intrinsic_height_passes_along_real_baselines_and_preserves_absence() {
    let frame = |suppress: bool| {
        let text = keyed(Text::new("Baseline"), TEXT);
        let child = if suppress {
            IgnoreBaseline::new(text).into()
        } else {
            text
        };
        Directionality::new(
            TextDirection::Ltr,
            Center::new(keyed(IntrinsicHeight::new(child), WRAPPER)),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(frame(false)).unwrap();
    let viewport = Constraints::tight(Size::new(800., 600.));
    tree.layout(viewport).unwrap();
    let (size, baseline) = metrics(&tree, &TEXT);
    assert!(baseline.is_some());
    assert_eq!(metrics(&tree, &WRAPPER), (size, baseline));

    tree.update(root, frame(true)).unwrap();
    tree.layout(viewport).unwrap();
    assert_eq!(metrics(&tree, &WRAPPER), (size, None));
    assert_eq!(metrics(&tree, &TEXT), (size, baseline));

    tree.update(root, frame(false)).unwrap();
    tree.layout(viewport).unwrap();
    assert_eq!(metrics(&tree, &WRAPPER), (size, baseline));
}

#[test]
fn intrinsic_height_placeholder_tolerates_a_zero_area_host() {
    let root = Center::new(keyed(IntrinsicHeight::new(Placeholder::new()), WRAPPER));
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(metrics(&tree, &WRAPPER), (Size::ZERO, None));
}

#[test]
fn directional_positioned_placeholder_tolerates_a_zero_area_host() {
    let child = PositionedDirectional::new(Placeholder::new())
        .top(0.)
        .start(0.);
    let root = Directionality::new(
        TextDirection::Ltr,
        Center::new(keyed(Stack::new([child]), WRAPPER)),
    );
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(metrics(&tree, &WRAPPER), (Size::ZERO, None));
}
