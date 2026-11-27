use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size, time::Instant};
use incular_runtime::Runtime;
use incular_scroll::ScrollController;
use incular_widgets::{
    BorderRadiusGeometry, ClipOverlapBehavior, CustomScrollView, Directionality, GestureDetector,
    Sliver, SliverClipRRect, SliverIgnorePointer, SliverPersistentHeader, SliverToBoxAdapter,
    TextDirection, Widget,
};

#[test]
fn preserved_sliver_shape_receives_the_underlap_tap_only_if_the_header_ignores_input() {
    for ignoring in [true, false] {
        let scroll = ScrollController::new();
        let child_taps = Rc::new(Cell::new(0));
        let header_taps = Rc::new(Cell::new(0));
        let sized = |height, color| Widget::box_(Size::new(800., height), color);
        let header = GestureDetector::new(sized(100., Color::BLACK)).on_tap({
            let hits = header_taps.clone();
            move || hits.set(hits.get() + 1)
        });
        let header = SliverPersistentHeader::new(100., header).pinned(true);
        let header = SliverIgnorePointer::new(ignoring, header);
        let child = GestureDetector::new(sized(100., Color::WHITE)).on_tap({
            let hits = child_taps.clone();
            move || hits.set(hits.get() + 1)
        });
        let clipped = SliverClipRRect::new(SliverToBoxAdapter::new(child))
            .clip_overlap(ClipOverlapBehavior::PreserveShape)
            .border_radius(BorderRadiusGeometry::circular(40.));
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(header),
            Box::new(clipped),
            Box::new(SliverToBoxAdapter::new(sized(1000., Color::WHITE))),
        ];
        let widget = Directionality::new(
            TextDirection::Ltr,
            CustomScrollView::new(slivers).controller(scroll.clone()),
        );
        let mut runtime = Runtime::new(widget.into()).unwrap();
        let t0 = Instant::now();
        let bounds = Constraints::tight(Size::new(800., 600.));
        runtime.run_frame_at(bounds, t0).unwrap();
        assert!(scroll.jump_to(50.));
        runtime.run_frame_at(bounds, t0).unwrap();

        for (n, phase) in [PointerPhase::Down, PointerPhase::Up]
            .into_iter()
            .enumerate()
        {
            runtime.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: Offset::new(10., 90.),
                },
                t0 + Duration::from_millis(n as u64 + 1),
            );
        }
        assert_eq!(child_taps.get(), i32::from(ignoring));
        assert_eq!(header_taps.get(), i32::from(!ignoring));
    }
}
