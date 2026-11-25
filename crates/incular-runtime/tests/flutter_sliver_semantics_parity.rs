//! Flutter 3.50.0 `sliver_semantics_test.dart`: "Offscreen sliver are
//! hidden in semantics tree". The contract concerns the attached semantics
//! nodes and their hidden flags, not paint clipping.

use incular_config::{Clip, Constraints};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Center, CustomScrollView, Directionality, ScrollController, SizedBox, Sliver,
    SliverEnsureSemantics, SliverToBoxAdapter, Text, TextDirection, Widget, internal::Key,
};

const ROWS: usize = 30;
const EXTENT: f32 = 200.;
const SCROLL: f32 = 300.;
const VIEWPORT: u64 = 100;

fn surface() -> Constraints {
    Constraints::tight(Size::new(800., 600.))
}

fn sliver_runtime(controller: &ScrollController, ensure: bool, clip: Clip) -> Runtime {
    let slivers: Vec<Box<dyn Sliver>> = (0..ROWS)
        .map(|index| {
            let text = Text::new(format!("Item {index}")).text_direction(TextDirection::Ltr);
            let row =
                Widget::from(SizedBox::new().height(EXTENT).child(text)).with_key(index as u64);
            let row = SliverToBoxAdapter::new(row);
            let sliver: Box<dyn Sliver> = if ensure && index == 4 {
                Box::new(SliverEnsureSemantics::new(row))
            } else {
                Box::new(row)
            };
            sliver
        })
        .collect();
    let view = Widget::from(
        CustomScrollView::new(slivers)
            .controller(controller.clone())
            .clip_behavior(clip),
    )
    .with_key(VIEWPORT);
    let scene = Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::new().height(EXTENT).child(view)),
    );
    Runtime::new(scene.into()).expect("runtime")
}

#[test]
fn sliver_box_adapters_apply_the_literal_initial_scroll_offset() {
    let controller = ScrollController::new().with_initial_scroll_offset(SCROLL);
    let mut runtime = sliver_runtime(&controller, false, Clip::HardEdge);
    runtime
        .run_frame(surface())
        .expect("first frame at initial scroll offset");
    assert_eq!(controller.viewport_extent(), EXTENT);
    assert_eq!(controller.offset(), SCROLL);
    let mut expected = [None; ROWS];
    expected[..4].copy_from_slice(&[Some(true), Some(false), Some(false), Some(true)]);
    assert_eq!(statuses(&runtime), expected);
}

#[test]
fn offscreen_sliver_box_adapters_are_hidden_only_within_the_cache() {
    let controller = ScrollController::new();
    let mut runtime = sliver_runtime(&controller, false, Clip::HardEdge);
    runtime.run_frame(surface()).expect("initial layout");
    // Isolate the semantics contract from initial-offset handling by jumping
    // to the same settled offset as Flutter after measuring the box extents.
    assert!(controller.jump_to(SCROLL));
    runtime.run_frame(surface()).expect("frame after jump");
    assert_eq!(controller.viewport_extent(), EXTENT);
    assert_eq!(controller.offset(), SCROLL);

    let tree = runtime.tree();
    let bounds = |key| {
        let element = tree.element_with_key(&Key::from(key)).expect("fixture key");
        tree.element_bounds(element).expect("fixture bounds")
    };
    let viewport = bounds(VIEWPORT);
    for index in 0..4 {
        let row = bounds(index);
        assert_eq!(row.size.height, EXTENT);
        assert_eq!(
            row.origin.y - viewport.origin.y,
            index as f32 * EXTENT - SCROLL
        );
    }

    let actual = statuses(&runtime);
    let mut expected = [None; ROWS];
    expected[..4].copy_from_slice(&[Some(true), Some(false), Some(false), Some(true)]);
    assert_eq!(
        actual, expected,
        "None is absent; Some(bool) is the hidden flag"
    );
}

fn statuses(runtime: &Runtime) -> [Option<bool>; ROWS] {
    let tree = runtime.tree();
    let mut actual = [None; ROWS];
    for (_, node) in tree.semantics().iter() {
        if let Some(index) = node
            .label
            .as_deref()
            .and_then(|label| label.strip_prefix("Item "))
            .and_then(|index| index.parse::<usize>().ok())
        {
            assert!(
                actual[index].replace(node.state.hidden).is_none(),
                "Item {index} should have only one semantics node",
            );
        }
    }
    actual
}

#[test]
fn ensured_sliver_bypasses_the_cache_clip_until_it_reenters_the_normal_window() {
    let controller = ScrollController::new().with_initial_scroll_offset(SCROLL);
    let mut runtime = sliver_runtime(&controller, true, Clip::HardEdge);
    runtime.run_frame(surface()).unwrap();
    let mut expected = [None; ROWS];
    expected[..5].copy_from_slice(&[
        Some(true),
        Some(false),
        Some(false),
        Some(true),
        Some(false),
    ]);
    assert_eq!(statuses(&runtime), expected);

    assert!(controller.jump_to(450.));
    runtime.run_frame(surface()).unwrap();
    expected[..5].copy_from_slice(&[None, Some(true), Some(false), Some(false), Some(true)]);
    assert_eq!(statuses(&runtime), expected);
}

#[test]
fn an_unclipped_viewport_keeps_cached_rows_unhidden_but_not_off_cache_rows() {
    let controller = ScrollController::new().with_initial_scroll_offset(SCROLL);
    let mut runtime = sliver_runtime(&controller, false, Clip::None);
    runtime.run_frame(surface()).unwrap();
    let mut expected = [None; ROWS];
    expected[..4].fill(Some(false));
    assert_eq!(statuses(&runtime), expected);
}
