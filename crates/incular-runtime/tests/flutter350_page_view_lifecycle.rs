//! At the default zero cache extent, a PageView must evict an ordinary page
//! after it leaves the viewport, even if both pages were mounted mid-scroll.
//! The pinned Flutter test also checks disposal and fresh state on return.

use std::time::Duration;

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{PageController, PageView, Widget, internal::Key};

#[test]
fn flutter_350_page_view_evicts_plain_page_after_partial_scroll_settles() {
    let page0 = Key::from("page0");
    let page1 = Key::from("page1");
    let extent = Size::new(800., 600.);
    let constraints = Constraints::tight(extent);
    for partial in [false, true] {
        let controller = PageController::new();
        let mut runtime = Runtime::new(
            PageView::new([
                Widget::box_(extent, Color::rgba(0x21, 0x96, 0xf3, 0xff)).with_key(page0.clone()),
                Widget::box_(extent, Color::rgba(0xff, 0x57, 0x22, 0xff)).with_key(page1.clone()),
                Widget::box_(extent, Color::rgba(0x4c, 0xaf, 0x50, 0xff)),
            ])
            .controller(controller.clone())
            .into(),
        )
        .unwrap();
        let start = Instant::now();
        let pump = |runtime: &mut Runtime, frame: u64| {
            runtime
                .run_frame_at(constraints, start + Duration::from_millis(frame * 16))
                .unwrap();
        };
        pump(&mut runtime, 0);
        let first = runtime.tree().element_with_key(&page0).unwrap();
        if partial {
            assert!(controller.jump_to(400.));
            pump(&mut runtime, 1);
            assert_eq!(controller.page(), Some(0.5));
            assert!(runtime.tree().element_exists(first));
            assert!(runtime.tree().element_with_key(&page1).is_some());
        }
        assert!(controller.jump_to_page(1));
        pump(&mut runtime, 2);
        assert_eq!(controller.page(), Some(1.0));
        assert!(runtime.tree().element_with_key(&page1).is_some());
        let still_mounted = runtime.tree().element_exists(first);
        assert!(controller.jump_to_page(0));
        pump(&mut runtime, 3);
        let returned = runtime.tree().element_with_key(&page0).unwrap();
        assert!(
            !still_mounted && returned != first,
            "page0 must unmount away and remount on return (partial={partial}, still_mounted={still_mounted}, same_element={})",
            returned == first
        );
    }
}

#[test]
fn page_cache_boundary_evicts_only_after_the_last_cached_pixel() {
    use incular_widgets::ScrollCacheExtent;

    let extent = Size::new(800., 600.);
    let constraints = Constraints::tight(extent);
    let key = Key::from("first");
    for reverse in [false, true] {
        let controller = PageController::new();
        let mut runtime = Runtime::new(
            PageView::new([
                Widget::box_(extent, Color::WHITE).with_key(key.clone()),
                Widget::box_(extent, Color::BLACK),
                Widget::box_(extent, Color::WHITE),
            ])
            .controller(controller.clone())
            .reverse(reverse)
            .page_snapping(false)
            .allow_implicit_scrolling(true)
            .scroll_cache_extent(ScrollCacheExtent::pixels(40.))
            .into(),
        )
        .unwrap();
        let t = Instant::now();
        runtime.run_frame_at(constraints, t).unwrap();
        let first = runtime.tree().element_with_key(&key).unwrap();
        assert!(controller.jump_to(800.));
        runtime
            .run_frame_at(constraints, t + Duration::from_millis(16))
            .unwrap();
        assert_eq!(runtime.tree().element_with_key(&key), Some(first));
        assert!(controller.jump_to(839.));
        let (_, stats) = runtime
            .run_frame_at(constraints, t + Duration::from_millis(32))
            .unwrap();
        assert_eq!(runtime.tree().element_with_key(&key), Some(first));
        assert_eq!(
            stats.laid_out_render_objects, 0,
            "scroll stays compositor-only before crossing"
        );

        assert!(controller.jump_to(840.));
        runtime
            .run_frame_at(constraints, t + Duration::from_millis(48))
            .unwrap();
        assert!(!runtime.tree().element_exists(first));
        assert!(runtime.tree().element_with_key(&key).is_none());
        assert!(controller.jump_to(839.));
        runtime
            .run_frame_at(constraints, t + Duration::from_millis(64))
            .unwrap();
        assert_ne!(runtime.tree().element_with_key(&key).unwrap(), first);
    }
}
