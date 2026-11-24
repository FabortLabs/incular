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
