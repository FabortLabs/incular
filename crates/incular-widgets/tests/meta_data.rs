//! Flutter's `MetaData` and reading it from a hit test.

use incular_config::Constraints;
use incular_core::{Color, Offset, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{Center, HitTestBehavior, MetaData, Positioned, Stack, Widget};

fn labels(tree: &WidgetTree, point: Offset) -> Vec<&'static str> {
    tree.hit_test_meta_data(point)
        .iter()
        .map(|data| *data.downcast_ref::<&'static str>().expect("a label"))
        .collect()
}

fn mount(widget: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(widget).expect("mount");
    tree.layout(Constraints::tight(Size::new(100., 100.)))
        .expect("layout");
    tree
}

fn square(side: f32) -> Widget {
    Widget::box_(Size::new(side, side), Color::WHITE)
}

#[test]
fn nested_meta_data_reports_deepest_first() {
    let tree = mount(
        MetaData::new("outer")
            .behavior(HitTestBehavior::Opaque)
            .child(Center::new(MetaData::new("inner").child(square(20.))))
            .into(),
    );
    assert_eq!(labels(&tree, Offset::new(50., 50.)), ["inner", "outer"]);
    // Outside the inner square only the opaque outer data is hit.
    assert_eq!(labels(&tree, Offset::new(5., 5.)), ["outer"]);
}

#[test]
fn defer_to_child_needs_a_hit_child() {
    // A childless data widget fills its tight constraints but has no child
    // to hit, so deferring data is not reported and translucent data is.
    let tree = mount(MetaData::new("deferring").into());
    assert!(labels(&tree, Offset::new(50., 50.)).is_empty());
    let tree = mount(
        MetaData::new("translucent")
            .behavior(HitTestBehavior::Translucent)
            .into(),
    );
    assert_eq!(labels(&tree, Offset::new(50., 50.)), ["translucent"]);
}

#[test]
fn opaque_data_hides_what_is_behind_it_and_translucent_does_not() {
    let layered = |behavior| {
        mount(
            Stack::new([
                Widget::from(MetaData::new("behind").child(square(100.))),
                Positioned::new(MetaData::new("front").behavior(behavior).child(square(0.)))
                    .left(0.)
                    .top(0.)
                    .width(50.)
                    .height(50.)
                    .into(),
            ])
            .into(),
        )
    };
    let tree = layered(HitTestBehavior::Opaque);
    assert_eq!(labels(&tree, Offset::new(10., 10.)), ["front"]);
    assert_eq!(labels(&tree, Offset::new(80., 80.)), ["behind"]);
    let tree = layered(HitTestBehavior::Translucent);
    assert_eq!(labels(&tree, Offset::new(10., 10.)), ["front", "behind"]);
}
