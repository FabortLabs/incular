use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    HitTestBehavior, MetaData, PlatformViewController, PlatformViewHitTestBehavior,
    PlatformViewSurface, SizedBox, Stack, Widget,
};

const POINT: Offset = Offset::new(1., 1.);

fn runtime(root: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(root.into()).unwrap();
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .unwrap();
    runtime
}

fn native(behavior: PlatformViewHitTestBehavior) -> Widget {
    PlatformViewSurface::new(PlatformViewController::new(
        "view",
        vec![],
        TextDirection::Ltr,
    ))
    .hit_test_behavior(behavior)
    .into()
}

fn ordinary() -> Widget {
    MetaData::new("ordinary")
        .behavior(HitTestBehavior::Translucent)
        .child(SizedBox::expand())
        .into()
}

#[test]
fn an_empty_hit_path_contains_no_platform_view() {
    let runtime = runtime(native(PlatformViewHitTestBehavior::Opaque));
    assert!(!runtime.hit_test_contains_platform_view(Offset::new(101., 1.)));
}

#[test]
fn an_ordinary_hit_path_contains_no_platform_view() {
    let runtime = runtime(ordinary());
    assert_eq!(runtime.tree().hit_test_meta_data(POINT).len(), 1);
    assert!(!runtime.hit_test_contains_platform_view(POINT));
}

fn both_targets(front_is_native: bool) {
    let embedded = native(PlatformViewHitTestBehavior::Translucent);
    let paint_order = if front_is_native {
        [ordinary(), embedded]
    } else {
        [embedded, ordinary()]
    };
    let mut runtime = runtime(Stack::new(paint_order));
    let _ = runtime.take_platform_view_commands();
    let path = runtime.tree().hit_test_meta_data(POINT);
    assert_eq!(path.len(), 2);
    assert_eq!(path[0].is::<&str>(), !front_is_native);
    assert_eq!(path[1].is::<&str>(), front_is_native);
    assert!(runtime.hit_test_contains_platform_view(POINT));
    assert!(runtime.take_platform_view_commands().is_empty());
}

#[test]
fn a_platform_surface_behind_an_ordinary_translucent_hit_is_found() {
    both_targets(false);
}

#[test]
fn a_platform_surface_in_front_of_an_ordinary_hit_is_found() {
    both_targets(true);
}

#[test]
fn a_transparent_platform_surface_does_not_accept_the_gesture() {
    let runtime = runtime(Stack::new([
        ordinary(),
        native(PlatformViewHitTestBehavior::Transparent),
    ]));
    assert!(!runtime.hit_test_contains_platform_view(POINT));
}
