//! Flutter 3.50.0 ignores outside callbacks for mounted, non-current
//! routes. Use one surface above a real retained outlet and check push/pop.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size,
};
use incular_navigation::{Navigator, Page};
use incular_runtime::{RouteOutlet, Runtime};
use incular_widgets::{
    Align, Alignment, HitTestBehavior, Listener, SizedBox, TapRegion, TapRegionSurface, Widget,
    internal::Key,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn bounds() -> Constraints {
    Constraints::tight(Size::new(200., 200.))
}

fn page(name: &'static str, key: &Key, count: &Rc<Cell<u32>>) -> Page {
    let count = Rc::clone(count);
    let region = Widget::from(
        TapRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
            .behavior(HitTestBehavior::Opaque)
            .on_tap_outside(move |_| count.set(count.get() + 1)),
    )
    .with_key(key.clone());
    // Make the page canvas hit-testable at the off-region click position.
    let canvas =
        Listener::new(Align::new(Alignment::CENTER, region)).behavior(HitTestBehavior::Opaque);
    Page::new(name, canvas)
}

fn tap_outside(runtime: &mut Runtime, key: &Key) {
    let tree = runtime.tree();
    let id = tree
        .element_with_key(key)
        .expect("current route region mounted");
    let (transform, size) = tree.element_geometry(id).expect("region geometry");
    let rect = transform.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size));
    let position = Offset::new(
        rect.origin.x + rect.size.width + 1.,
        rect.origin.y + rect.size.height + 1.,
    );
    assert!((0. ..200.).contains(&position.x) && (0. ..200.).contains(&position.y));
    for (phase, buttons) in [
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind: PointerDeviceKind::Touch,
            buttons,
            button: Some(PRIMARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        });
    }
}

#[test]
fn tap_region_outside_callbacks_follow_current_route_across_push_and_pop() {
    let navigator = Navigator::new();
    let mut runtime = Runtime::new(TapRegionSurface::new(SizedBox::shrink()).into())
        .expect("mount common tap surface");
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(
        &navigator,
        &runtime.spawner().scope(),
    )));
    let mounted = Rc::clone(&outlet);
    let root = runtime.tree().root().expect("surface root");
    runtime
        .register_builder(root, move || {
            TapRegionSurface::new(SizedBox::from_dimensions(
                Some(200.),
                Some(200.),
                Some(mounted.borrow().widget()),
            ))
            .into()
        })
        .expect("mount navigation outlet on common surface");
    runtime.run_frame(bounds()).expect("initial frame");
    RouteOutlet::attach(&outlet, &mut runtime).expect("attach outlet");
    let present = |runtime: &mut Runtime| {
        RouteOutlet::present_frame(&outlet, runtime, bounds()).expect("present settled route");
    };

    let first = Rc::new(Cell::new(0));
    let second = Rc::new(Cell::new(0));
    let first_key = Key::from("first tap region");
    let second_key = Key::from("second tap region");
    navigator.push_page(page("first", &first_key, &first));
    present(&mut runtime);
    let covered = runtime
        .tree()
        .element_with_key(&first_key)
        .expect("first route mounted");

    navigator.push_page(page("second", &second_key, &second));
    present(&mut runtime);
    assert!(
        runtime.tree().element_exists(covered),
        "inactive route remains mounted"
    );
    tap_outside(&mut runtime, &second_key);
    let on_second = (first.get(), second.get());

    navigator.pop();
    present(&mut runtime);
    assert!(runtime.tree().element_with_key(&second_key).is_none());
    assert_eq!(runtime.tree().element_with_key(&first_key), Some(covered));
    tap_outside(&mut runtime, &first_key);
    let on_first_again = (first.get(), second.get());

    assert_eq!([on_second, on_first_again], [(0, 1), (1, 1)]);
}
