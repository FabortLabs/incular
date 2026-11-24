//! A Flutter 3.50.0 kept-alive row preserves identity without painting
//! after it leaves the sliver cache, even in an unclipped viewport.

use std::time::{Duration, Instant};

use incular_config::{Clip, Constraints};
use incular_core::{Color, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, KeepAlive, Positioned, ScrollController, SizedBox, Sliver, SliverList, Stack,
    Widget, internal::Key,
};

#[test]
fn off_cache_kept_alive_row_does_not_paint_outside_an_unclipped_viewport() {
    const MARKER: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
    let marker_key = Key::from(2_u64);
    let sliver = SliverList::builder(40, {
        let marker_key = marker_key.clone();
        move |index| {
            let child = if index == 2 {
                Widget::box_(Size::new(100., 50.), MARKER).with_key(marker_key.clone())
            } else {
                SizedBox::new().height(50.).into()
            };
            KeepAlive::new(index == 2, child)
        }
    });
    let controller = ScrollController::new();
    let view = CustomScrollView::new([Box::new(sliver) as Box<dyn Sliver>])
        .controller(controller.clone())
        .clip_behavior(Clip::None);
    // When parked, row 2's extrapolated position is still inside the outer
    // scene (400 + 100 - 450 = 50). Neither viewport nor scene clips it away.
    let root = Stack::new([Positioned::new(view)
        .left(0.)
        .top(400.)
        .width(100.)
        .height(180.)])
    .clip_behavior(Clip::None);
    let mut runtime = Runtime::new(root.into()).expect("runtime");
    let bounds = Constraints::tight(Size::new(800., 600.));
    let start = Instant::now();
    let mut original_id = None;
    let painted = std::array::from_fn(|frame| {
        let offset = [0., 450., 0.][frame];
        if frame != 0 {
            controller.jump_to(offset);
        }
        let (list, _) = runtime
            .run_frame_at(bounds, start + Duration::from_millis(16 * frame as u64))
            .expect("frame");
        assert_eq!(controller.offset(), offset);
        let id = runtime
            .tree()
            .element_with_key(&marker_key)
            .expect("kept-alive row remains mounted");
        assert_eq!(id, *original_id.get_or_insert(id), "same retained row");

        list.commands().iter().any(|command| match command {
            PaintCommand::Rect { color, .. }
            | PaintCommand::RRect {
                brush: Brush::Solid(color),
                ..
            } => *color == MARKER,
            _ => false,
        })
    });
    assert_eq!(painted, [true, false, true], "paint before, away, and back");
}
