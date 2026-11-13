use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::host;
use incular_core::{Color, Offset, time::Instant};
use incular_navigation::{HeroController, NavigationEvent, Navigator, Route, RouteTransition};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::Hero;
use incular_widgets::{
    Align, Alignment, BasicRouterDelegate, ClipRect, ColoredBox, GestureDetector,
    MemoryRouteInformationProvider, NavigationBackButtonDispatcher, OverlayPortal, PopScope,
    Positioned, RouteInformationProvider, Router, SizedBox, Stack, StringRouteInformationParser,
    Widget,
};
use std::time::Duration;

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);

fn solid(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "navigation__navigator__push_pop_can_pop",
            navigator_push_pop,
        ),
        (
            "navigation__router__provider_route_change",
            router_provider_route_change,
        ),
        (
            "navigation__pop_scope__blocked_root_route",
            pop_scope_blocked,
        ),
        (
            "navigation__overlay_portal__content_outside_anchor_clip",
            overlay_portal_outside_clip,
        ),
        (
            "navigation__overlay_portal__show_hide",
            overlay_portal_show_hide,
        ),
        (
            "navigation__overlay_portal__direct_anchor",
            overlay_portal_direct_anchor,
        ),
        ("navigation__navigator__fade_push_timeline", |h| {
            fade_timeline(h, false)
        }),
        ("navigation__navigator__fade_pop_timeline", |h| {
            fade_timeline(h, true)
        }),
        ("navigation__hero__push_flight_navigator", |h| {
            hero_flight(h, false)
        }),
        ("navigation__hero__pop_flight_navigator", |h| {
            hero_flight(h, true)
        }),
        (
            "navigation__navigator__observer_order",
            navigator_observer_order,
        ),
        (
            "navigation__pop_scope__route_back_blocked",
            pop_scope_route_back_blocked,
        ),
    ]
}

fn navigator_push_pop(h: &mut FrameHarness) {
    let navigator = Navigator::new();
    // The root Navigator installs its initial route before observer events.
    let pushes = Rc::new(Cell::new(1.0));
    let pops = Rc::new(Cell::new(0.0));
    navigator.push(Route::new("/home", solid(BLUE)));
    let _observer = navigator.observe({
        let pushes = Rc::clone(&pushes);
        let pops = Rc::clone(&pops);
        move |event| match event {
            NavigationEvent::Pushed { .. } => pushes.set(pushes.get() + 1.),
            NavigationEvent::Popped { .. } => pops.set(pops.get() + 1.),
            _ => {}
        }
    });
    h.value("can_pop", {
        let navigator = navigator.clone();
        move || f64::from(navigator.can_pop())
    });
    h.value("pushes", {
        let pushes = Rc::clone(&pushes);
        move || pushes.get()
    });
    h.value("pops", {
        let pops = Rc::clone(&pops);
        move || pops.get()
    });
    h.mount(|_| solid(BLUE));
    h.frames(1);
    navigator.push(Route::new("/details", solid(ORANGE)));
    h.frames(2);
    navigator.pop();
    h.frames(2);
}

fn router_provider_route_change(h: &mut FrameHarness) {
    let path = Rc::new(RefCell::new("/home".to_owned()));
    let build_path = Rc::clone(&path);
    let delegate = BasicRouterDelegate::new(move || {
        let color = if build_path.borrow().as_str() == "/details" {
            ORANGE
        } else {
            BLUE
        };
        Align::new(
            Alignment::TOP_LEFT,
            probe(
                "route",
                SizedBox::new().width(120.).height(80.).child(solid(color)),
            ),
        )
        .into()
    });
    delegate.on_set_new_route_path({
        let path = Rc::clone(&path);
        move |route: String| {
            *path.borrow_mut() = route;
            Ok(())
        }
    });
    let provider = MemoryRouteInformationProvider::new("/home");
    let router =
        Router::with_provider_parser(delegate, provider.clone(), StringRouteInformationParser);
    h.value("route_is_details", {
        let path = Rc::clone(&path);
        move || f64::from(path.borrow().as_str() == "/details")
    });
    h.value("provider_is_details", {
        let provider = provider.clone();
        move || f64::from(provider.value().location() == "/details")
    });
    h.mount(move |_| router.clone().into_widget());
    h.frames(2);
    provider.set_value("/details");
    h.frames(2);
}

fn pop_scope_blocked(h: &mut FrameHarness) {
    let dispatcher = NavigationBackButtonDispatcher::new();
    let callbacks = Rc::new(Cell::new(0.0));
    let did_pop = Rc::new(Cell::new(-1.0));
    h.value("callbacks", {
        let callbacks = Rc::clone(&callbacks);
        move || callbacks.get()
    });
    h.value("did_pop", {
        let did_pop = Rc::clone(&did_pop);
        move || did_pop.get()
    });
    let pop_scope = PopScope::new(SizedBox::new().width(100.).height(80.).child(solid(BLUE)))
        .can_pop(false)
        .dispatcher(dispatcher.clone())
        .on_pop_invoked({
            let callbacks = Rc::clone(&callbacks);
            let did_pop = Rc::clone(&did_pop);
            move |popped| {
                callbacks.set(callbacks.get() + 1.);
                did_pop.set(if popped { 1. } else { 0. });
            }
        });
    h.mount(move |_| Widget::from(pop_scope.clone()));
    h.frames(1);
    let _ = dispatcher.dispatch_back();
    h.frames(1);
}

fn overlay_portal_outside_clip(h: &mut FrameHarness) {
    let taps = Rc::new(Cell::new(0.0));
    h.value("popup_taps", {
        let taps = Rc::clone(&taps);
        move || taps.get()
    });
    let tap = Rc::clone(&taps);
    h.mount(move |phase| {
        let popup = GestureDetector::new(probe(
            "popup",
            SizedBox::new().width(80.).height(60.).child(solid(ORANGE)),
        ))
        .on_tap({
            let tap = Rc::clone(&tap);
            move || tap.set(tap.get() + 1.)
        });
        let portal: Widget = OverlayPortal::new(Align::new(
            Alignment::CENTER,
            probe(
                "anchor",
                SizedBox::new().width(60.).height(50.).child(solid(BLUE)),
            ),
        ))
        .overlay_child_layout_builder(move |_, _| {
            Stack::new([Widget::from(
                Positioned::new(popup.clone()).left(200.).top(200.),
            )])
            .into()
        })
        .show(phase == 1)
        .into();
        Widget::from(Align::new(
            Alignment::TOP_LEFT,
            ClipRect::new(SizedBox::new().width(100.).height(100.).child(portal)),
        ))
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(1);
    h.down(Offset::new(225., 225.));
    h.frames(1);
    h.up();
    h.frames(1);
}

fn overlay_portal_show_hide(h: &mut FrameHarness) {
    h.mount(|phase| {
        let popup = probe(
            "popup",
            SizedBox::new().width(80.).height(60.).child(solid(ORANGE)),
        );
        let portal: Widget = OverlayPortal::new(Align::new(
            Alignment::CENTER,
            probe(
                "anchor",
                SizedBox::new().width(60.).height(50.).child(solid(BLUE)),
            ),
        ))
        .overlay_child_layout_builder(move |_, _| {
            Stack::new([Widget::from(
                Positioned::new(popup.clone()).left(200.).top(200.),
            )])
            .into()
        })
        .show(phase == 1)
        .into();
        Widget::from(Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(100.).height(100.).child(portal),
        ))
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(2);
    h.set_phase(2);
    h.frames(2);
}

fn overlay_portal_direct_anchor(h: &mut FrameHarness) {
    h.mount(|_| {
        let portal: Widget = OverlayPortal::new(probe(
            "anchor",
            SizedBox::new().width(60.).height(50.).child(solid(BLUE)),
        ))
        .overlay_child_layout_builder(|_, _| SizedBox::shrink().into())
        .show(false)
        .into();
        Widget::from(Align::new(
            Alignment::TOP_LEFT,
            SizedBox::new().width(100.).height(100.).child(portal),
        ))
    });
    h.frames(2);
}

const FADE: Duration = Duration::from_millis(300);

fn probe_box(name: &str, color: Color) -> Widget {
    Align::new(
        Alignment::TOP_LEFT,
        probe(
            name,
            SizedBox::new().width(120.).height(80.).child(solid(color)),
        ),
    )
    .into()
}

fn fade_timeline(h: &mut FrameHarness, pop: bool) {
    let navigator = Navigator::new();
    navigator.push(Route::new("/home", probe_box("below", BLUE)));
    incular_parity_harness::load_fonts();
    let base = Instant::now();
    host::mount_navigator(h, &navigator, |_, outlet| outlet);
    h.frames(2);
    navigator.push(Route::new("/details", probe_box("top", ORANGE)).transition(
        RouteTransition::fade_in(FADE, base + h.elapsed() + Duration::from_millis(16)),
    ));
    h.frames(22);
    if pop {
        navigator.pop();
        h.frames(22);
    }
}

fn hero_stack(name: &str, x: f32, y: f32, width: f32, height: f32, color: Color) -> Widget {
    Stack::new([Widget::from(
        Positioned::new(Widget::from(Hero::new(
            "photo",
            probe(
                name,
                SizedBox::new()
                    .width(width)
                    .height(height)
                    .child(solid(color)),
            ),
        )))
        .left(x)
        .top(y),
    )])
    .into()
}

fn hero_flight(h: &mut FrameHarness, pop: bool) {
    let navigator = Navigator::new();
    let controller = HeroController::new();
    controller.attach(&navigator);
    navigator.push(Route::new(
        "/home",
        hero_stack("hero_from", 10., 20., 40., 40., BLUE),
    ));
    incular_parity_harness::load_fonts();
    let base = Instant::now();
    let overlay = controller.clone();
    host::mount_navigator(h, &navigator, move |_, outlet| {
        Stack::new([outlet, overlay.overlay()]).into()
    });
    h.frames(2);
    navigator.push(
        Route::new(
            "/details",
            hero_stack("hero_to", 100., 150., 80., 60., ORANGE),
        )
        .transition(RouteTransition::fade_in(
            FADE,
            base + h.elapsed() + Duration::from_millis(16),
        )),
    );
    h.frames(22);
    if pop {
        navigator.pop();
        h.frames(22);
    }
}

fn navigator_observer_order(h: &mut FrameHarness) {
    let navigator = Navigator::new();
    let log = Rc::new(Cell::new(0.0_f64));
    let observer_log = Rc::clone(&log);
    let _observer = navigator.observe(move |event| {
        let code = match event {
            NavigationEvent::Pushed { .. } => 1.,
            NavigationEvent::Replaced { .. } => 2.,
            NavigationEvent::Popped { .. } => 3.,
            _ => return,
        };
        observer_log.set(observer_log.get() * 10. + code);
    });
    navigator.push(Route::new("/home", probe_box("root", BLUE)));
    h.value("log", {
        let log = Rc::clone(&log);
        move || log.get()
    });
    host::mount_navigator(h, &navigator, |_, outlet| outlet);
    h.frames(2);
    navigator.push(Route::new("/a", probe_box("a", ORANGE)));
    h.frames(2);
    let _ = navigator.replace(Route::new("/b", probe_box("b", BLUE)));
    h.frames(2);
    let _ = navigator.pop();
    h.frames(2);
}

fn pop_scope_route_back_blocked(h: &mut FrameHarness) {
    let navigator = Navigator::new();
    navigator.push(Route::new("/home", probe_box("root", BLUE)));
    let dispatcher = NavigationBackButtonDispatcher::new();
    let callbacks = Rc::new(Cell::new(0.0));
    let did_pop = Rc::new(Cell::new(-1.0));
    h.value("callbacks", {
        let callbacks = Rc::clone(&callbacks);
        move || callbacks.get()
    });
    h.value("did_pop", {
        let did_pop = Rc::clone(&did_pop);
        move || did_pop.get()
    });
    let pop_scope = PopScope::new(probe_box("top", ORANGE))
        .can_pop(false)
        .dispatcher(dispatcher.clone())
        .on_pop_invoked({
            let callbacks = Rc::clone(&callbacks);
            let did_pop = Rc::clone(&did_pop);
            move |popped| {
                callbacks.set(callbacks.get() + 1.);
                did_pop.set(if popped { 1. } else { 0. });
            }
        });
    host::mount_navigator(h, &navigator, |_, outlet| outlet);
    h.frames(2);
    navigator.push(Route::new("/details", Widget::from(pop_scope)));
    h.frames(2);
    let _ = dispatcher.dispatch_back();
    h.frames(2);
}
