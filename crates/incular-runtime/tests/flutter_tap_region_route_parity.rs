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
    Align, Alignment, GestureDetector, HitTestBehavior, Listener, Positioned, SizedBox, Stack,
    TapRegion, TapRegionSurface, Widget, internal::Key,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const VIEWPORT_WIDTH: f32 = 800.;
const VIEWPORT_HEIGHT: f32 = 600.;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
}

fn page(name: &'static str, key: &Key, count: &Rc<Cell<(u32, u32)>>) -> Page {
    let count = Rc::clone(count);
    let up_count = Rc::clone(&count);
    let region = Widget::from(
        TapRegion::new(Widget::box_(Size::new(100., 100.), Color::WHITE))
            .behavior(HitTestBehavior::Opaque)
            .on_tap_outside(move |_| {
                let (down, up) = count.get();
                count.set((down + 1, up));
            })
            .on_tap_up_outside(move |_| {
                let (down, up) = up_count.get();
                up_count.set((down, up + 1));
            }),
    )
    .with_key(key.clone());
    // Make the page canvas hit-testable at the off-region click position.
    let canvas =
        Listener::new(Align::new(Alignment::CENTER, region)).behavior(HitTestBehavior::Opaque);
    Page::new(name, canvas)
}

fn tap(runtime: &mut Runtime, key: &Key, inside: bool) {
    let tree = runtime.tree();
    let id = tree
        .element_with_key(key)
        .expect("current route region mounted");
    let (transform, size) = tree.element_geometry(id).expect("region geometry");
    let rect = transform.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size));
    let position = if inside {
        Offset::new(
            rect.origin.x + rect.size.width / 2.,
            rect.origin.y + rect.size.height / 2.,
        )
    } else {
        Offset::new(
            rect.origin.x + rect.size.width + 1.,
            rect.origin.y + rect.size.height + 1.,
        )
    };
    assert!((0. ..VIEWPORT_WIDTH).contains(&position.x));
    assert!((0. ..VIEWPORT_HEIGHT).contains(&position.y));
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

fn host() -> (Navigator, Runtime, Rc<RefCell<RouteOutlet>>) {
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
                Some(VIEWPORT_WIDTH),
                Some(VIEWPORT_HEIGHT),
                Some(mounted.borrow().widget()),
            ))
            .into()
        })
        .expect("mount navigation outlet on common surface");
    runtime.run_frame(bounds()).expect("initial frame");
    RouteOutlet::attach(&outlet, &mut runtime).expect("attach outlet");
    (navigator, runtime, outlet)
}

#[test]
fn tap_region_outside_callbacks_follow_current_route_across_push_and_pop() {
    let (navigator, mut runtime, outlet) = host();
    let present = |runtime: &mut Runtime| {
        RouteOutlet::present_frame(&outlet, runtime, bounds()).expect("present settled route");
    };

    let first = Rc::new(Cell::new((0, 0)));
    let second = Rc::new(Cell::new((0, 0)));
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
    tap(&mut runtime, &second_key, false);
    let on_second = (first.get(), second.get());

    navigator.pop();
    present(&mut runtime);
    assert!(runtime.tree().element_with_key(&second_key).is_none());
    assert_eq!(runtime.tree().element_with_key(&first_key), Some(covered));
    tap(&mut runtime, &first_key, false);
    let on_first_again = (first.get(), second.get());

    assert_eq!(
        [on_second, on_first_again],
        [((0, 0), (1, 1)), ((1, 1), (1, 1))]
    );
}

#[test]
fn navigation_button_tap_is_outside_only_for_the_route_it_leaves() {
    let (navigator, mut runtime, outlet) = host();
    let present = |runtime: &mut Runtime| {
        RouteOutlet::present_frame(&outlet, runtime, bounds()).expect("present settled route");
    };
    let first_key = Key::from("first region with navigation");
    let second_key = Key::from("second region after navigation");
    let button_key = Key::from("outside navigation button");
    let first_count = Rc::new(Cell::new((0, 0)));
    let second_count = Rc::new(Cell::new((0, 0)));
    let second_page = page("second", &second_key, &second_count);
    let button = Widget::from(
        GestureDetector::new(Widget::box_(Size::new(56., 56.), Color::WHITE))
            .behavior(HitTestBehavior::Opaque)
            .on_tap({
                let navigator = navigator.clone();
                move || {
                    navigator.push_page(second_page.clone());
                }
            }),
    )
    .with_key(button_key.clone());
    let mut first_page = page("first", &first_key, &first_count);
    first_page.child = Stack::new([
        first_page.child,
        Widget::from(Positioned::new(button).right(16.).bottom(16.)),
    ])
    .into();
    navigator.push_page(first_page);
    present(&mut runtime);
    let down = || (first_count.get().0, second_count.get().0);

    tap(&mut runtime, &first_key, false);
    assert_eq!(down(), (1, 0));
    tap(&mut runtime, &button_key, true);
    present(&mut runtime);
    tap(&mut runtime, &second_key, false);
    assert_eq!(down(), (2, 1));
    navigator.pop();
    present(&mut runtime);
    tap(&mut runtime, &first_key, false);
    assert_eq!(down(), (3, 1));
}

#[test]
fn tap_region_consumption_does_not_block_a_new_route_button() {
    let (navigator, mut runtime, outlet) = host();
    let first_key = Key::from("navigation tap region");
    let button_key = Key::from("next route test button");
    let tapped = Rc::new(Cell::new(false));
    let button = Widget::from(
        GestureDetector::new(Widget::box_(Size::new(120., 48.), Color::WHITE))
            .behavior(HitTestBehavior::Opaque)
            .on_tap({
                let tapped = Rc::clone(&tapped);
                move || tapped.set(true)
            }),
    )
    .with_key(button_key.clone());
    let next = Widget::from(Align::new(Alignment::CENTER, button));
    let link = GestureDetector::new(Widget::box_(Size::new(150., 150.), Color::WHITE))
        .behavior(HitTestBehavior::Opaque)
        .on_tap({
            let navigator = navigator.clone();
            move || {
                navigator.push_page(Page::new("next", next.clone()));
            }
        });
    let first = Widget::from(
        TapRegion::new(link)
            .consume_outside_taps(true)
            .on_tap_outside(|_| {})
            .behavior(HitTestBehavior::Opaque),
    )
    .with_key(first_key.clone());
    navigator.push_page(Page::new("first", Align::new(Alignment::CENTER, first)));
    RouteOutlet::present_frame(&outlet, &mut runtime, bounds()).expect("present first route");
    tap(&mut runtime, &first_key, true);
    RouteOutlet::present_frame(&outlet, &mut runtime, bounds()).expect("present next route");
    tap(&mut runtime, &button_key, true);
    assert!(
        tapped.get(),
        "covered route must not consume the button tap"
    );
}
