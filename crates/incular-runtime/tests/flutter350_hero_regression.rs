//! Native regression for the matched shuttle in the Flutter 3.50 Hero
//! push/pop frame scenarios. The outgoing route must fly back while it fades.

use std::{cell::RefCell, rc::Rc, time::Duration};

use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_navigation::{HeroController, Navigator, Route, RouteTransition};
use incular_rendering::{Brush, DisplayList, PaintCommand};
use incular_runtime::{RouteOutlet, Runtime};
use incular_widgets::{
    ColoredBox, Hero, HeroRectTween, HeroTag, Positioned, SizedBox, Stack, Widget, internal::Key,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const SHUTTLE: Color = Color::rgba(0, 0x80, 0, 0xFF);
const FRAME_MS: u64 = 16;

fn shuttle_key() -> Key {
    "flutter350-hero-shuttle".into()
}

fn hero_page(x: f32, y: f32, width: f32, height: f32, color: Color) -> Widget {
    let endpoint = SizedBox::new()
        .width(width)
        .height(height)
        .child(ColoredBox::new(color, SizedBox::shrink()));
    let hero = Hero::new("photo", endpoint).flight_shuttle_builder(|_, _| {
        Widget::from(ColoredBox::new(SHUTTLE, SizedBox::shrink())).with_key(shuttle_key())
    });
    Stack::new([Widget::from(Positioned::new(hero).left(x).top(y))]).into()
}

fn paints_shuttle(list: &DisplayList) -> bool {
    list.commands().iter().any(|command| match command {
        PaintCommand::Rect { color, .. } => *color == SHUTTLE,
        PaintCommand::RRect {
            brush: Brush::Solid(color),
            ..
        } => *color == SHUTTLE,
        _ => false,
    })
}

#[test]
fn flutter_350_hero_paints_a_return_shuttle_during_route_pop() {
    let navigator = Navigator::new();
    let controller = HeroController::with_create_rect_tween(HeroRectTween::linear);
    controller.attach(&navigator);
    navigator.push(Route::new("/home", hero_page(10., 20., 40., 40., BLUE)));

    let mut runtime = Runtime::new(Stack::new(Vec::<Widget>::new()).into()).unwrap();
    let outlet = Rc::new(RefCell::new(RouteOutlet::new(
        &navigator,
        &runtime.spawner().scope(),
    )));
    let root = runtime.tree().root().unwrap();
    runtime
        .register_builder(root, {
            let outlet = Rc::clone(&outlet);
            move || Stack::new([outlet.borrow().widget(), controller.overlay()]).into()
        })
        .unwrap();
    let constraints = Constraints::tight(Size::new(800., 600.));
    let start = Instant::now();
    runtime.run_frame_at(constraints, start).unwrap();
    RouteOutlet::attach(&outlet, &mut runtime).unwrap();
    let present = |runtime: &mut Runtime, frame| {
        let time = start + Duration::from_millis(frame * FRAME_MS);
        RouteOutlet::present_frame_at(&outlet, runtime, constraints, time)
            .unwrap()
            .0
    };

    for frame in 0..3 {
        present(&mut runtime, frame);
    }
    let transition = RouteTransition::fade_in(
        Duration::from_millis(300),
        start + Duration::from_millis(3 * FRAME_MS),
    );
    let animation = transition.animation().unwrap();
    navigator.push(
        Route::new("/details", hero_page(100., 150., 80., 60., ORANGE)).transition(transition),
    );
    for frame in 3..10 {
        present(&mut runtime, frame);
    }
    let push = present(&mut runtime, 10);
    assert!(runtime.tree().element_with_key(&shuttle_key()).is_some());
    assert!(paints_shuttle(&push), "push must paint the same shuttle");
    for frame in 11..25 {
        present(&mut runtime, frame);
    }
    assert!(animation.is_completed());
    assert!(runtime.tree().element_with_key(&shuttle_key()).is_none());

    let popped = navigator.pop().expect("details pop succeeds");
    for frame in 25..32 {
        present(&mut runtime, frame);
    }
    let previous_value = animation.value();
    let pop = present(&mut runtime, 32);
    assert!(
        (0.0..previous_value).contains(&animation.value()) && animation.value() > 0.,
        "the outgoing route must still be reversing: {previous_value} to {}",
        animation.value()
    );
    let present = runtime.tree().element_with_key(&shuttle_key()).is_some();
    let painted = paints_shuttle(&pop);
    let tag = HeroTag::from("photo");
    assert!(
        present && painted,
        "return shuttle must be mounted and painted (mounted={present}, painted={painted}, from={:?}, to={:?})",
        popped.heroes().location(&tag),
        navigator.current().unwrap().heroes().location(&tag),
    );
}

/// Checks the quarter-step geometry of the actual painted shuttle. The
/// Flutter default curve tests use centered squares sized 100 and 200.
fn assert_curve_flight(
    pop: bool,
    curve: incular_widgets::Curve,
    reverse_curve: Option<incular_widgets::Curve>,
    expected: impl Fn(f32) -> f32,
) {
    use incular_core::Offset;
    use incular_widgets::Center;

    let page = |size, curve, reverse_curve, endpoint: Key| {
        let child: Widget = SizedBox::new()
            .width(size)
            .height(size)
            .child(ColoredBox::new(BLUE, SizedBox::shrink()))
            .into();
        Center::new(
            Hero::new("curve", child.with_key(endpoint))
                .curve(curve)
                .reverse_curve(reverse_curve)
                .create_rect_tween(HeroRectTween::linear)
                .flight_shuttle_builder(|_, _| {
                    Widget::from(ColoredBox::new(SHUTTLE, SizedBox::shrink()))
                        .with_key(shuttle_key())
                }),
        )
    };

    let navigator = Navigator::new();
    let controller = HeroController::new();
    controller.attach(&navigator);
    let home_key: Key = "curve-home-endpoint".into();
    let details_key: Key = "curve-details-endpoint".into();
    let details_size = if pop { 200. } else { 150. };
    navigator.push(Route::new(
        "/home",
        page(100., incular_widgets::Curve::Linear, None, home_key.clone()),
    ));

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
    let now = Instant::now();
    let at = |ms| now + Duration::from_millis(ms);
    runtime.run_frame_at(constraints, at(0)).unwrap();
    RouteOutlet::attach(&outlet, &mut runtime).unwrap();
    let present = |runtime: &mut Runtime, ms| {
        RouteOutlet::present_frame_at(&outlet, runtime, constraints, at(ms))
            .unwrap()
            .0
    };
    present(&mut runtime, 0);
    let geometry = |runtime: &Runtime, key: &Key| {
        let tree = runtime.tree();
        tree.element_geometry(tree.element_with_key(key).unwrap())
            .unwrap()
    };
    assert_eq!(geometry(&runtime, &home_key).1, Size::new(100., 100.));

    navigator.push(
        Route::new(
            "/details",
            page(details_size, curve, reverse_curve, details_key.clone()),
        )
        .transition(RouteTransition::fade_in(Duration::from_millis(300), at(16))),
    );
    present(&mut runtime, 16);
    assert_eq!(
        geometry(&runtime, &details_key).1,
        Size::new(details_size, details_size)
    );
    let flight_start = if pop {
        present(&mut runtime, 32);
        present(&mut runtime, 317);
        assert!(
            runtime.tree().element_with_key(&shuttle_key()).is_none(),
            "the settled push must not leave an overlay shuttle",
        );
        assert_eq!(
            geometry(&runtime, &details_key).1,
            Size::new(details_size, details_size)
        );
        assert!(navigator.pop().is_some());
        present(&mut runtime, 332);
        332
    } else {
        16
    };

    for (time, quarter) in [(75, 0.25), (150, 0.50), (225, 0.75)] {
        let list = present(&mut runtime, flight_start + time);
        assert!(paints_shuttle(&list), "missing shuttle at {quarter}");
        let (transform, size) = geometry(&runtime, &shuttle_key());
        let span = details_size - 100.;
        let target = if pop {
            details_size - span * expected(quarter)
        } else {
            100. + span * expected(quarter)
        };
        assert!(
            (size.width - target).abs() < 0.001,
            "{quarter}: {size:?} != {target}"
        );
        assert!(
            (size.height - target).abs() < 0.001,
            "{quarter}: {size:?} != {target}"
        );
        let origin = transform.transform_point(Offset::ZERO);
        assert!((origin.x - (800. - target) * 0.5).abs() < 0.001);
        assert!((origin.y - (600. - target) * 0.5).abs() < 0.001);
    }
    let (end, key) = if pop {
        (100., &home_key)
    } else {
        (details_size, &details_key)
    };
    // Flutter's interpolation simulation reaches the exact end value on
    // this frame; it becomes completed only after the duration.
    present(&mut runtime, flight_start + 300);
    let final_key = if runtime.tree().element_with_key(&shuttle_key()).is_some() {
        &shuttle_key()
    } else {
        key
    };
    assert_eq!(geometry(&runtime, final_key).1, Size::new(end, end));
    let list = present(&mut runtime, flight_start + 301);
    assert!(
        !paints_shuttle(&list),
        "completed flight still paints its shuttle"
    );
    assert!(runtime.tree().element_with_key(&shuttle_key()).is_none());
    assert_eq!(geometry(&runtime, key).1, Size::new(end, end));
}

#[test]
fn flutter_350_hero_push_uses_the_destination_curve() {
    use incular_widgets::Curve;
    assert_curve_flight(false, Curve::FastOutSlowIn, None, |t| {
        Curve::FastOutSlowIn.apply(t)
    });
    assert_curve_flight(false, Curve::Linear, None, |t| t);
}

#[test]
fn flutter_350_hero_pop_uses_flipped_default_or_explicit_reverse_curve() {
    use incular_widgets::Curve;
    assert_curve_flight(true, Curve::FastOutSlowIn, None, |t| {
        Curve::FastOutSlowIn.apply(t)
    });
    assert_curve_flight(true, Curve::EaseIn, None, |t| Curve::EaseIn.apply(t));
    assert_curve_flight(true, Curve::EaseIn, Some(Curve::EaseOut), |t| {
        1. - Curve::EaseOut.apply(1. - t)
    });
}
