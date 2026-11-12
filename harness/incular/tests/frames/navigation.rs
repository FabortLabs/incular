use std::cell::Cell;
use std::rc::Rc;

use incular_core::{Color, Offset};
use incular_navigation::{NavigationEvent, Navigator, Route};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    Align, Alignment, ClipRect, ColoredBox, GestureDetector, NavigationBackButtonDispatcher,
    OverlayPortal, PopScope, Positioned, SizedBox, Stack, Widget,
};

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
            "navigation__pop_scope__blocked_root_route",
            pop_scope_blocked,
        ),
        (
            "navigation__overlay_portal__content_outside_anchor_clip",
            overlay_portal_outside_clip,
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
