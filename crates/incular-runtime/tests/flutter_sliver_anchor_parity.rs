//! Flutter 3.50.0 center/anchor scroll metrics regression. The variable-list
//! oracle is the paired `slivers__viewport_center__anchor` capture documented
//! in audits/flutter-3.50.0-parity/frame-rechecks-batch9.md.

use std::time::{Duration, Instant};

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, ScrollController, SizedBox, Sliver, SliverFixedExtentList,
    SliverList, SliverToBoxAdapter, TextDirection, Widget, internal::Key,
};

const CENTER: u64 = 1;
const VIEWPORT: Size = Size {
    width: 800.,
    height: 600.,
};
const FRAME: Duration = Duration::from_millis(16);

fn centered_scroll_view(before: Box<dyn Sliver>) -> (Runtime, ScrollController) {
    let controller = ScrollController::new();
    let slivers: Vec<Box<dyn Sliver>> = vec![
        before,
        Box::new(SliverToBoxAdapter::new(
            Widget::from(SizedBox::new().height(80.)).with_key(CENTER),
        )),
        Box::new(SliverFixedExtentList::new(10, 50., |_| {
            SizedBox::new().height(50.)
        })),
    ];
    let view = CustomScrollView::new(slivers)
        .controller(controller.clone())
        .center(1)
        .anchor(0.25);
    let runtime =
        Runtime::new(Directionality::new(TextDirection::Ltr, view).into()).expect("runtime");
    (runtime, controller)
}

#[test]
fn viewport_center_anchor_estimates_reverse_lazy_list_from_current_layout() {
    let before = SliverList::builder(12, |index| {
        SizedBox::new().height(if index % 2 == 0 { 30. } else { 45. })
    });
    let (mut runtime, controller) = centered_scroll_view(Box::new(before));
    let start = Instant::now();
    let jumps = [None, Some(-200.), None, Some(240.), None];
    let actual: [_; 5] = std::array::from_fn(|frame| {
        if let Some(offset) = jumps[frame] {
            controller.jump_to(offset);
        }
        runtime
            .run_frame_at(Constraints::tight(VIEWPORT), start + FRAME * frame as u32)
            .expect("scroll frame");
        (controller.min_offset(), controller.max_offset())
    });

    // At mount Flutter sees 11 rows totaling 405 and estimates one more at
    // their mean. At -200 it sees all twelve (450). After the positive jump
    // only five totaling 180 remain, and Flutter estimates seven more at 36.
    // The center/anchor shift adds 150 to each negated reverse extent.
    let expected_min = [150. - (405. + 405. / 11.), -300., -300., -282., -282.];
    assert!(
        actual
            .iter()
            .zip(expected_min)
            .all(|(&(min, max), expected)| { (min - expected).abs() < 0.001 && max == 130. }),
        "observed (min, max) per frame: {actual:?}; expected min {expected_min:?} and max 130",
    );
}

#[test]
fn viewport_center_anchor_fixed_extent_control_has_stable_range() {
    let before = SliverFixedExtentList::new(12, 37.5, |_| SizedBox::new().height(37.5));
    let (mut runtime, controller) = centered_scroll_view(Box::new(before));
    let start = Instant::now();
    for (frame, offset) in [0., -200., 100., -300., 130.].into_iter().enumerate() {
        if frame != 0 {
            controller.jump_to(offset);
        }
        runtime
            .run_frame_at(Constraints::tight(VIEWPORT), start + FRAME * frame as u32)
            .expect("scroll frame");
        assert_eq!(
            (
                controller.min_offset(),
                controller.offset(),
                controller.max_offset()
            ),
            (-300., offset, 130.),
        );
        let tree = runtime.tree();
        let center = tree
            .element_with_key(&Key::from(CENTER))
            .and_then(|element| tree.element_bounds(element))
            .expect("center sliver geometry");
        assert_eq!(center.origin.y, 150. - offset);
    }
}
