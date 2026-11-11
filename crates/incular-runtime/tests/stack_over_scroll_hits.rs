//! A layer stacked above a scroll view must not swallow taps that miss its
//! own content: an `IgnorePointer`, an empty box, or a small `Positioned`
//! child leave the list below hittable (Flutter's `RenderStack` defers to
//! its children's hit tests).

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size,
};
use incular_platform::{PlatformEvent, WindowEvent, WindowOptions};
use incular_runtime::Application;
use incular_widgets::{
    Color, Column, GestureDetector, IgnorePointer, Positioned, SingleChildScrollView, SizedBox,
    Stack, Widget,
};
use std::{cell::Cell, rc::Rc, time::Instant};

fn window() -> Size {
    Size::new(200., 200.)
}

fn app(root: Widget) -> Application {
    let retained = root.clone();
    let mut application = Application::new_with_options(
        WindowOptions {
            initial_logical_size: window(),
            ..WindowOptions::new("stack hit test")
        },
        move |_| retained.clone(),
    )
    .expect("application");
    let id = application.primary_window();
    application
        .run_window_frame_at(id, Constraints::tight(window()), Instant::now())
        .expect("initial frame");
    application
}

fn click(application: &mut Application, position: Offset) {
    let id = application.primary_window();
    for (phase, buttons) in [
        (PointerPhase::Move, 0),
        (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        application.handle_window_event(WindowEvent::platform(
            id,
            PlatformEvent::Input(InputEvent::PointerWithMetadata {
                pointer: 0,
                device: 1,
                kind: PointerDeviceKind::Mouse,
                buttons,
                button: Some(PRIMARY_POINTER_BUTTON),
                sample: incular_core::PointerSampleMetadata::default(),
                phase,
                position,
            }),
        ));
        application
            .run_window_frame_at(id, Constraints::tight(window()), Instant::now())
            .expect("frame");
    }
}

fn list(taps: &Rc<Cell<u32>>) -> Widget {
    let taps = taps.clone();
    SingleChildScrollView::new(Column::new([
        GestureDetector::new(Widget::box_(Size::new(200., 60.), Color::WHITE))
            .on_tap(move || taps.set(taps.get() + 1))
            .into(),
        Widget::box_(Size::new(200., 600.), Color::BLACK),
    ]))
    .into()
}

fn assert_list_receives_tap(overlay: Widget, label: &str) {
    let taps = Rc::new(Cell::new(0));
    let mut application = app(incular_widgets::Directionality::new(
        incular_widgets::TextDirection::Ltr,
        Stack::new([list(&taps), overlay]),
    )
    .into());
    click(&mut application, Offset::new(100., 30.));
    assert_eq!(
        taps.get(),
        1,
        "{label}: tap below the overlay must reach the list"
    );
}

#[test]
fn ignore_pointer_layer_does_not_swallow_scroll_view_taps() {
    assert_list_receives_tap(
        IgnorePointer::new(SizedBox::shrink()).into(),
        "IgnorePointer(SizedBox::shrink())",
    );
}

#[test]
fn empty_layer_does_not_swallow_scroll_view_taps() {
    assert_list_receives_tap(SizedBox::shrink().into(), "SizedBox::shrink()");
}

#[test]
fn small_positioned_layer_does_not_swallow_scroll_view_taps() {
    assert_list_receives_tap(
        Positioned::new(Widget::box_(Size::new(20., 20.), Color::WHITE))
            .right(0.)
            .bottom(0.)
            .into(),
        "small Positioned child",
    );
}

#[test]
fn scrolled_list_under_an_ignore_pointer_layer_keeps_receiving_taps() {
    use incular_widgets::{ScrollController, internal::ActionSurface};
    let taps = Rc::new(Cell::new(0));
    let controller = ScrollController::new();
    let items: Vec<Widget> = (0..20)
        .map(|index| {
            let taps = taps.clone();
            ActionSurface::new(format!("item {index}"))
                .size(Size::new(200., 40.))
                .on_press(move || taps.set(taps.get() + 1))
                .into()
        })
        .collect();
    let scroll = SingleChildScrollView::new(Column::new(items)).controller(controller.clone());
    let mut application = app(incular_widgets::Directionality::new(
        incular_widgets::TextDirection::Ltr,
        Stack::new([
            Widget::from(Positioned::fill(scroll)),
            IgnorePointer::new(SizedBox::shrink()).into(),
        ]),
    )
    .into());
    click(&mut application, Offset::new(100., 30.));
    assert_eq!(taps.get(), 1, "unscrolled");
    let _ = controller.jump_to(100.);
    click(&mut application, Offset::new(100., 30.));
    assert_eq!(taps.get(), 2, "scrolled");
}

#[test]
fn nested_stack_with_small_positioned_child_does_not_swallow_taps() {
    assert_list_receives_tap(
        Stack::new([Widget::from(
            Positioned::new(Widget::box_(Size::new(20., 20.), Color::WHITE))
                .right(0.)
                .bottom(0.),
        )])
        .into(),
        "nested Stack holding a small Positioned child",
    );
}

#[test]
fn align_wrapper_does_not_swallow_taps() {
    assert_list_receives_tap(
        incular_widgets::Align::new(
            incular_config::Alignment::BOTTOM_RIGHT,
            Widget::box_(Size::new(20., 20.), Color::WHITE),
        )
        .into(),
        "Align wrapping a small child",
    );
}
