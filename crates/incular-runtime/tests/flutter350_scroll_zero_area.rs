//! Zero-area viewport cases from Flutter 3.50 widgets tests.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_widgets::{
    Center, CustomScrollView, Directionality, GridView, KeepAlive, ListView, Placeholder,
    ScrollController, ShrinkWrappingViewport, SizedBox, Sliver, SliverToBoxAdapter, Text, Widget,
    internal::Key,
};

const VIEWPORT: Key = Key::Value(100);
const SURFACE: Size = Size {
    width: 800.,
    height: 600.,
};

fn boxed_viewport(viewport: impl Into<Widget>, size: Size) -> Widget {
    Center::new(SizedBox::from_size(size).child(viewport.into().with_key(VIEWPORT))).into()
}

fn mount(scene: impl Into<Widget>) -> Runtime {
    Runtime::new(Directionality::new(TextDirection::Ltr, scene).into()).unwrap()
}

fn viewport_size(runtime: &Runtime) -> Size {
    let tree = runtime.tree();
    let element = tree
        .element_with_key(&VIEWPORT)
        .expect("viewport stays mounted");
    tree.element_bounds(element).unwrap().size
}

#[test]
fn custom_scroll_view_at_zero_area_keeps_its_centered_viewport() {
    let sliver: Box<dyn Sliver> = Box::new(SliverToBoxAdapter::new(Placeholder::new()));
    let mut runtime = mount(boxed_viewport(CustomScrollView::new([sliver]), Size::ZERO));
    runtime.run_frame(Constraints::tight(SURFACE)).unwrap();
    assert_eq!(viewport_size(&runtime), Size::ZERO);
}

#[test]
fn list_view_at_zero_area_keeps_its_centered_viewport() {
    let mut runtime = mount(boxed_viewport(
        ListView::new([Widget::from(Placeholder::new())]),
        Size::ZERO,
    ));
    runtime.run_frame(Constraints::tight(SURFACE)).unwrap();
    assert_eq!(viewport_size(&runtime), Size::ZERO);
}

#[test]
fn zero_sized_grid_has_no_onstage_tiles_and_recovers() {
    let grid = |bounds| {
        let tiles = (0..20u64).map(|i| Widget::from(Text::new(i.to_string())).with_key(i));
        boxed_viewport(GridView::count(4, tiles), bounds)
    };
    let mut runtime = mount(grid(Size::ZERO));
    let constraints = Constraints::tight(SURFACE);
    let (hidden, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(viewport_size(&runtime), Size::ZERO);
    let retained_zero_tile = runtime
        .tree()
        .element_with_key(&Key::Value(0))
        .expect("the layout samples only tile zero");
    assert!(runtime.tree().element_with_key(&Key::Value(1)).is_none());
    assert!(
        !hidden
            .commands()
            .iter()
            .any(|paint| matches!(paint, PaintCommand::GlyphRun { .. }))
    );

    let root = runtime.tree().root().unwrap();
    let recovered = Size::new(300., 300.);
    runtime
        .schedule_update(
            root,
            Directionality::new(TextDirection::Ltr, grid(recovered)).into(),
        )
        .unwrap();
    let (visible, _) = runtime.run_frame(constraints).unwrap();
    assert_eq!(viewport_size(&runtime), recovered);
    assert_eq!(
        runtime.tree().element_with_key(&Key::Value(0)),
        Some(retained_zero_tile)
    );
    assert!(
        visible
            .commands()
            .iter()
            .any(|paint| matches!(paint, PaintCommand::GlyphRun { .. }))
    );
    for tile in [0, 1] {
        let tile = runtime.tree().element_with_key(&Key::Value(tile)).unwrap();
        let bounds = runtime.tree().element_bounds(tile).unwrap();
        assert!(bounds.size.width > 0. && bounds.size.height > 0.);
    }
}

#[test]
fn shrink_wrapping_viewport_accepts_a_zero_area_host_and_a_nonzero_offset() {
    let host = WindowMetrics::new(PhysicalSize::new(0, 0), 1.);
    let controller = ScrollController::new();
    assert!(controller.apply_layout_correction_by(5.));
    assert_eq!(controller.offset(), 5.);
    let viewport =
        Widget::from(ShrinkWrappingViewport::default().controller(controller)).with_key(VIEWPORT);
    let mut runtime = mount(Center::new(viewport));
    runtime.handle_platform_event(PlatformEvent::Metrics(host));
    assert_eq!(runtime.environment().viewport, Size::ZERO);
    runtime
        .run_frame(Constraints::tight(runtime.environment().viewport))
        .unwrap();
    assert_eq!(viewport_size(&runtime), Size::ZERO);
}

#[test]
fn a_zero_stride_grid_keeps_explicitly_retained_siblings_without_materializing_others() {
    let grid = GridView::count(
        4,
        (0..20u64).map(|index| {
            KeepAlive::new(
                index == 1,
                Widget::from(Text::new(index.to_string())).with_key(index),
            )
        }),
    );
    let mut runtime = mount(Widget::from(grid).with_key(VIEWPORT));
    let find = |runtime: &Runtime, key| runtime.tree().element_with_key(&Key::Value(key));
    runtime.run_frame(Constraints::tight(SURFACE)).unwrap();
    let retained = find(&runtime, 1).unwrap();
    assert!(find(&runtime, 2).is_some());

    let (hidden, _) = runtime.run_frame(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(viewport_size(&runtime), Size::ZERO);
    assert_eq!(find(&runtime, 1), Some(retained));
    assert!(find(&runtime, 2).is_none());
    assert!(
        !hidden
            .commands()
            .iter()
            .any(|paint| matches!(paint, PaintCommand::GlyphRun { .. }))
    );

    runtime.run_frame(Constraints::tight(SURFACE)).unwrap();
    assert_eq!(find(&runtime, 1), Some(retained));
    assert!(find(&runtime, 2).is_some());
}

#[test]
fn zero_main_axis_cache_window_samples_one_tile_even_when_stride_is_positive() {
    let grid = GridView::count(
        4,
        (0..20u64).map(|index| Widget::from(Text::new(index.to_string())).with_key(index)),
    )
    .cache_extent(0.);
    let mut runtime = mount(Widget::from(grid).with_key(VIEWPORT));
    let (hidden, _) = runtime
        .run_frame(Constraints::tight(Size::new(800., 0.)))
        .unwrap();
    assert_eq!(viewport_size(&runtime), Size::new(800., 0.));
    assert!(runtime.tree().element_with_key(&Key::Value(0)).is_some());
    assert!(runtime.tree().element_with_key(&Key::Value(1)).is_none());
    assert!(
        !hidden
            .commands()
            .iter()
            .any(|paint| matches!(paint, PaintCommand::GlyphRun { .. }))
    );
}
