use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, Placeholder, SizedOverflowBox, Widget, internal::Key,
};

#[test]
fn sized_overflow_box_tracks_a_zero_area_host_view_and_recovers_without_remount() {
    const OVERFLOW: Key = Key::Value(1);
    let overflow = Widget::from(SizedOverflowBox::new(
        Size::new(100., 100.),
        Placeholder::new(),
    ))
    .with_key(OVERFLOW);
    let scene = Directionality::new(TextDirection::Ltr, Center::new(overflow));
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let mut retained = None;
    for (width, height, expected) in [
        (
            800,
            600,
            Rect::from_origin_size(Offset::new(350., 250.), Size::new(100., 100.)),
        ),
        (0, 0, Rect::from_origin_size(Offset::ZERO, Size::ZERO)),
        (
            800,
            600,
            Rect::from_origin_size(Offset::new(350., 250.), Size::new(100., 100.)),
        ),
    ] {
        let metrics = WindowMetrics::new(PhysicalSize::new(width, height), 1.);
        runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
        let view = runtime.environment();
        assert_eq!((view.physical_width, view.physical_height), (width, height));
        runtime
            .run_frame(Constraints::tight(view.viewport))
            .unwrap();
        let tree = runtime.tree();
        let overflow = tree.element_with_key(&OVERFLOW).unwrap();
        let identity = (overflow, tree.render_id(overflow).unwrap());
        assert_eq!(identity, *retained.get_or_insert(identity));
        assert_eq!(tree.element_bounds(overflow), Some(expected));
    }
}
