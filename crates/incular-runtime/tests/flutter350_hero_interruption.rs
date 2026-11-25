//! Flutter 3.50 Hero pop interrupting an in-progress push.
use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Lerp, Offset, Rect, Size, time::Instant};
use incular_navigation::{HeroController, Navigator, Route, RouteTransition};
use incular_runtime::{RouteOutlet, Runtime};
use incular_widgets::{
    Curve, Hero, HeroRectTween, Positioned, SizedBox, Stack, Widget, internal::Key,
};

fn page(x: f32, shuttle: &'static str) -> Widget {
    let child = Hero::new(
        "photo",
        SizedBox::from_dimensions(Some(40.), Some(40.), None::<Widget>),
    )
    .curve(Curve::Linear)
    .create_rect_tween(|begin, end| {
        HeroRectTween::custom(begin, end, |begin, end, t| {
            let k = t * t * t;
            let mut rect: Rect = begin.lerp(&end, t);
            rect.origin.x = begin.origin.x + (end.origin.x - begin.origin.x) * k;
            rect
        })
    })
    .flight_shuttle_builder(move |_, _| {
        Widget::from(SizedBox::shrink()).with_key(Key::from(shuttle))
    });
    Stack::new([Widget::from(Positioned::new(child).left(x).top(20.))]).into()
}

#[test]
fn interrupted_push_retraces_its_original_tween_with_the_same_shuttle() {
    let navigator = Navigator::new();
    let controller = HeroController::new();
    controller.attach(&navigator);
    navigator.push(Route::new("/home", page(10., "home-shuttle")));

    let mut runtime = Runtime::new(Stack::new(Vec::<Widget>::new()).into()).unwrap();
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(
        &navigator,
        &runtime.spawner().scope(),
    )));
    runtime
        .register_builder(runtime.tree().root().unwrap(), {
            let outlet = Rc::clone(&outlet);
            move || Stack::new([outlet.borrow().widget(), controller.overlay()]).into()
        })
        .unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    let start = Instant::now();
    let at = |ms| start + Duration::from_millis(ms);
    runtime.run_frame_at(constraints, at(0)).unwrap();
    RouteOutlet::attach(&outlet, &mut runtime).unwrap();
    let frame = |runtime: &mut Runtime, ms| {
        RouteOutlet::present_frame_at(&outlet, runtime, constraints, at(ms)).unwrap();
    };
    frame(&mut runtime, 0);
    navigator.push(
        Route::new("/details", page(110., "detail-shuttle"))
            .transition(RouteTransition::fade_in(Duration::from_millis(300), at(16))),
    );
    let shuttle_key = Key::from("detail-shuttle");
    let home_shuttle_key = Key::from("home-shuttle");
    let shuttle = |runtime: &Runtime| {
        let tree = runtime.tree();
        let element = tree
            .element_with_key(&shuttle_key)
            .expect("the original shuttle remains mounted");
        let (transform, _) = tree.element_geometry(element).unwrap();
        assert!(tree.element_with_key(&home_shuttle_key).is_none());
        (element, transform.transform_point(Offset::ZERO).x)
    };
    // Pump the route into view, then the newly inserted shuttle.
    frame(&mut runtime, 16);
    frame(&mut runtime, 16);
    let identity = shuttle(&runtime).0;
    let mut positions = [10.; 5];
    for t in 1..=4 {
        frame(&mut runtime, 16 + t as u64 * 30);
        let progress = t as f32 / 10.;
        positions[t] = shuttle(&runtime).1;
        assert!((positions[t] - (10. + 100. * progress.powi(3))).abs() < 0.001);
    }
    assert!(navigator.pop().is_some());
    frame(&mut runtime, 136);
    for (i, expected) in positions.into_iter().rev().enumerate() {
        frame(&mut runtime, 136 + i as u64 * 30);
        let (id, x) = shuttle(&runtime);
        assert_eq!(id, identity);
        assert!(
            (x - expected).abs() < 0.001,
            "reverse sample {i}: {x} != {expected}"
        );
    }
    frame(&mut runtime, 257);
    assert!(runtime.tree().element_with_key(&shuttle_key).is_none());
}
