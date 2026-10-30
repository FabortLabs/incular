//! Flutter's `Hero` flights (`widgets/heroes.dart`) between the pages of a
//! navigator. Expected rectangles are worked out from Flutter's maths: the
//! flight follows the route animation through `Curves.fastOutSlowIn`
//! (`Cubic(0.4, 0, 0.2, 1)`, 0.025724 at 0.1 and 0.775580 at 0.5) and a
//! linear `RectTween` unless a `createRectTween` is given.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular::prelude::{
    Color, ColoredBox, Directionality, Hero, HeroController, HeroFlightDirection, HeroMode,
    HeroRectTween, KeyedSubtree, Navigator, Offset, Positioned, Rect, Route, RouteTransition, Size,
    SizedBox, Stack, TextDirection, Widget,
};
use incular_config::Constraints;
use incular_core::{Transform, time::Instant};
use incular_rendering::{Brush, DisplayList, PaintCommand};
use incular_widgets::internal::{OpacityController, WidgetTree};

const WIDTH: f32 = 200.;
const HEIGHT: f32 = 300.;
const FROM: Color = Color::rgba(255, 0, 0, 255);
const TO: Color = Color::rgba(0, 0, 255, 255);
const SHUTTLE: Color = Color::rgba(0, 255, 0, 255);
const DURATION: Duration = Duration::from_millis(300);
/// `Curves.fastOutSlowIn` at 0.1 and 0.5.
const CURVE_AT_TENTH: f32 = 0.025_724;
const CURVE_AT_HALF: f32 = 0.775_580;

fn painted(list: &DisplayList, color: Color) -> Vec<Rect> {
    let mut transforms = vec![Transform::IDENTITY];
    let mut rects = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                let current = *transforms.last().unwrap();
                transforms.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, color: c } if *c == color => {
                rects.push(transforms.last().unwrap().transform_rect_bbox(*rect));
            }
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(c),
            } if *c == color => {
                rects.push(transforms.last().unwrap().transform_rect_bbox(rrect.rect));
            }
            _ => {}
        }
    }
    rects
}

fn assert_rect(actual: Rect, expected: Rect) {
    let close = |a: f32, b: f32| (a - b).abs() < 0.1;
    assert!(
        close(actual.origin.x, expected.origin.x)
            && close(actual.origin.y, expected.origin.y)
            && close(actual.size.width, expected.size.width)
            && close(actual.size.height, expected.size.height),
        "expected {expected:?}, got {actual:?}"
    );
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

/// A page with `hero` at `at`.
fn page(at: Rect, hero: Widget) -> Widget {
    SizedBox::from_dimensions(
        Some(WIDTH),
        Some(HEIGHT),
        Some(Stack::new([Widget::from(
            Positioned::new(hero)
                .left(at.origin.x)
                .top(at.origin.y)
                .width(at.size.width)
                .height(at.size.height),
        )])),
    )
    .into()
}

/// A box filling its hero.
fn fill(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::expand()).into()
}

fn hero_box(tag: &str, color: Color) -> Hero {
    Hero::new(tag, fill(color))
}

/// A fading page transition starting at `start`.
fn fade(start: Instant) -> (RouteTransition, OpacityController) {
    let opacity = OpacityController::new();
    opacity.set_opacity(0.);
    opacity.animate_to(1., DURATION, start);
    (RouteTransition::Fade(opacity.clone()), opacity)
}

struct Harness {
    tree: WidgetTree,
    navigator: Navigator,
}

impl Harness {
    /// The navigator's routes in a stack with the controller's overlay
    /// above them.
    fn new(navigator: Navigator, controller: &HeroController) -> Self {
        controller.attach(&navigator);
        let revision = Rc::new(Cell::new(0_u64));
        let observer = {
            let revision = revision.clone();
            navigator.observe(move |_| revision.set(revision.get() + 1))
        };
        let routes = navigator.clone();
        let overlay = controller.overlay();
        let root = Widget::stateful_builder(revision, move |_| {
            let _observer = &observer;
            let mut children: Vec<Widget> = routes
                .routes()
                .iter()
                .map(|route| KeyedSubtree::new(route.name.as_str(), route.presented_child()).into())
                .collect();
            children.push(KeyedSubtree::new("heroes", overlay.clone()).into());
            Stack::new(children).into()
        });
        let mut tree = WidgetTree::new();
        tree.mount(Directionality::new(TextDirection::Ltr, root).into())
            .expect("mount");
        let mut harness = Self { tree, navigator };
        harness.frame();
        harness
    }

    fn frame(&mut self) -> DisplayList {
        self.tree
            .layout(Constraints::tight(Size::new(WIDTH, HEIGHT)))
            .expect("layout");
        self.tree.paint()
    }
}

/// Pushes a page with a fading transition, and lays it out once at the
/// start of its animation, as Flutter's offstage frame does.
fn push(harness: &mut Harness, child: Widget) -> (Instant, OpacityController) {
    let start = Instant::now();
    let (transition, opacity) = fade(start);
    harness
        .navigator
        .push(Route::new("next", child).transition(transition));
    harness.frame();
    (start, opacity)
}

fn tick(harness: &mut Harness, opacity: &OpacityController, at: Instant) -> DisplayList {
    opacity.animation().tick(at);
    harness.frame()
}

#[test]
fn a_hero_flies_along_the_curved_route_animation_to_its_new_place() {
    let navigator = Navigator::new();
    let from = rect(10., 20., 40., 40.);
    let to = rect(100., 150., 80., 60.);
    navigator.push(Route::new(
        "home",
        page(from, hero_box("photo", FROM).into()),
    ));
    let controller = HeroController::new();
    let mut harness = Harness::new(navigator, &controller);
    assert_rect(painted(&harness.frame(), FROM)[0], from);

    let (start, opacity) = push(&mut harness, page(to, hero_box("photo", TO).into()));
    let list = tick(&mut harness, &opacity, start + DURATION / 10);
    // The shuttle is the destination's child; both heroes hold their place
    // hidden.
    assert!(painted(&list, FROM).is_empty());
    let shuttles = painted(&list, TO);
    assert_eq!(shuttles.len(), 1);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let at = |t: f32| {
        rect(
            lerp(10., 100., t),
            lerp(20., 150., t),
            lerp(40., 80., t),
            lerp(40., 60., t),
        )
    };
    assert_rect(shuttles[0], at(CURVE_AT_TENTH));

    let list = tick(&mut harness, &opacity, start + DURATION / 2);
    assert_rect(painted(&list, TO)[0], at(CURVE_AT_HALF));

    // At the end the destination hero shows itself; the departing hero
    // keeps its placeholder under the new page.
    let list = tick(&mut harness, &opacity, start + DURATION);
    assert_eq!(painted(&list, TO), [to]);
    assert!(painted(&list, FROM).is_empty());
    let routes = harness.navigator.routes();
    let tag = "photo".into();
    assert!(routes[0].heroes().is_in_flight(&tag));
    assert!(!routes[1].heroes().is_in_flight(&tag));
}

#[test]
fn the_destination_hero_create_rect_tween_wins_over_the_controller() {
    // The controller's tween would stay at the start; the destination's
    // jumps to the end.
    let navigator = Navigator::new();
    let from = rect(0., 0., 20., 20.);
    let to = rect(120., 200., 60., 40.);
    navigator.push(Route::new(
        "home",
        page(from, hero_box("card", FROM).into()),
    ));
    let controller = HeroController::with_create_rect_tween(|begin, end| {
        HeroRectTween::custom(begin, end, |begin, _, _| begin)
    });
    let mut harness = Harness::new(navigator, &controller);
    let hero = hero_box("card", TO)
        .create_rect_tween(|begin, end| HeroRectTween::custom(begin, end, |_, end, _| end));
    let (start, opacity) = push(&mut harness, page(to, hero.into()));
    let list = tick(&mut harness, &opacity, start + DURATION / 2);
    assert_rect(painted(&list, TO)[0], to);
}

#[test]
fn the_controller_create_rect_tween_applies_without_a_hero_tween() {
    let navigator = Navigator::new();
    let from = rect(0., 0., 20., 20.);
    let to = rect(120., 200., 60., 40.);
    navigator.push(Route::new(
        "home",
        page(from, hero_box("card", FROM).into()),
    ));
    // An arc-like tween: the width follows the progress, the origin waits
    // for the end.
    let controller = HeroController::with_create_rect_tween(|begin, end| {
        HeroRectTween::custom(begin, end, |begin, end, t| {
            Rect::from_origin_size(
                begin.origin,
                Size::new(
                    begin.size.width + (end.size.width - begin.size.width) * t,
                    begin.size.height,
                ),
            )
        })
    });
    let mut harness = Harness::new(navigator, &controller);
    let (start, opacity) = push(&mut harness, page(to, hero_box("card", TO).into()));
    let list = tick(&mut harness, &opacity, start + DURATION / 2);
    assert_rect(
        painted(&list, TO)[0],
        rect(0., 0., 20. + 40. * CURVE_AT_HALF, 20.),
    );
}

#[test]
fn the_shuttle_builder_gets_the_flight_and_both_children() {
    let navigator = Navigator::new();
    let from = rect(10., 10., 30., 30.);
    let to = rect(110., 210., 50., 50.);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let departing = hero_box("avatar", FROM).flight_shuttle_builder(|_, _| fill(Color::BLACK));
    navigator.push(Route::new("home", page(from, departing.into())));
    let controller = HeroController::new();
    let mut harness = Harness::new(navigator, &controller);
    // The destination's builder wins over the departing hero's.
    let destination = hero_box("avatar", TO).flight_shuttle_builder({
        let seen = seen.clone();
        move |_, flight| {
            seen.borrow_mut().push((
                flight.direction,
                flight.animation,
                flight.from_child.clone(),
                flight.to_child.clone(),
            ));
            fill(SHUTTLE)
        }
    });
    let (start, opacity) = push(&mut harness, page(to, destination.into()));
    let list = tick(&mut harness, &opacity, start + DURATION / 2);
    assert!(painted(&list, Color::BLACK).is_empty());
    let shuttle = painted(&list, SHUTTLE);
    let lerp = |a: f32, b: f32| a + (b - a) * CURVE_AT_HALF;
    assert_rect(
        shuttle[0],
        rect(
            lerp(10., 110.),
            lerp(10., 210.),
            lerp(30., 50.),
            lerp(30., 50.),
        ),
    );
    let seen = seen.borrow();
    let (direction, animation, from_child, to_child) = seen.last().expect("built");
    assert_eq!(*direction, HeroFlightDirection::Push);
    assert!((animation - CURVE_AT_HALF).abs() < 1e-3);
    assert!(*from_child == fill(FROM));
    assert!(*to_child == fill(TO));
}

#[test]
fn placeholders_and_hero_mode() {
    let navigator = Navigator::new();
    let from = rect(10., 10., 30., 30.);
    let to = rect(110., 210., 50., 50.);
    let placeholder = Color::rgba(9, 9, 9, 255);
    let departing =
        hero_box("a", FROM).placeholder_builder(move |_, size, _| Widget::box_(size, placeholder));
    let page_one = Stack::new([
        page(from, departing.into()),
        // A disabled hero does not fly.
        page(
            rect(10., 100., 30., 30.),
            HeroMode::new(false, hero_box("b", FROM)).into(),
        ),
    ]);
    navigator.push(Route::new("home", page_one));
    let controller = HeroController::new();
    let mut harness = Harness::new(navigator, &controller);
    let page_two = Stack::new([
        page(to, hero_box("a", TO).into()),
        page(rect(110., 100., 30., 30.), hero_box("b", TO).into()),
    ]);
    let (start, opacity) = push(&mut harness, page_two.into());
    let list = tick(&mut harness, &opacity, start + DURATION / 2);
    // The departing hero shows its placeholder at its own size.
    assert_eq!(painted(&list, placeholder), [from]);
    // "b" stays where it is on both pages; only "a" has a shuttle.
    let mut blue = painted(&list, TO);
    blue.sort_by(|a, b| a.origin.y.total_cmp(&b.origin.y));
    assert_eq!(blue.len(), 2);
    assert_rect(blue[0], rect(110., 100., 30., 30.));
    assert_eq!(painted(&list, FROM), [rect(10., 100., 30., 30.)]);
}

#[test]
fn heroes_do_not_fly_without_a_route_animation_or_on_pop() {
    let navigator = Navigator::new();
    let from = rect(10., 10., 30., 30.);
    let to = rect(110., 210., 50., 50.);
    navigator.push(Route::new("home", page(from, hero_box("x", FROM).into())));
    let controller = HeroController::new();
    let mut harness = Harness::new(navigator.clone(), &controller);
    // No transition: the route animation is complete, so nothing flies.
    navigator.push(Route::new("plain", page(to, hero_box("x", TO).into())));
    let list = harness.frame();
    assert_eq!(painted(&list, TO), [to]);
    assert_eq!(painted(&list, FROM), [from]);
    navigator.pop();

    // After a flight the departing hero keeps its placeholder; popping
    // back has no reverse animation in Incular, so it shows again at once.
    let (start, opacity) = push(&mut harness, page(to, hero_box("x", TO).into()));
    tick(&mut harness, &opacity, start + DURATION / 2);
    let list = tick(&mut harness, &opacity, start + DURATION);
    assert!(painted(&list, FROM).is_empty());
    navigator.pop();
    let list = harness.frame();
    assert_eq!(painted(&list, FROM), [from]);
    assert!(painted(&list, TO).is_empty());
}
