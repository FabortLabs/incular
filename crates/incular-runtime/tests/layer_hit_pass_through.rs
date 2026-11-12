//! A transparent upper layer passes clicks outside its painted content to the
//! layers below, and an ancestor `GestureDetector` does not fire for a tap a
//! descendant recognizer wins.
//!
//! Flutter's layout and proxy render objects (`RenderPositionedBox`,
//! `RenderPadding`, `RenderFlex`, `RenderClipRRect`, `RenderBackdropFilter`,
//! `RenderOpacity`, ...) only hit through their children, and competing tap
//! recognizers enter one arena where the innermost wins.

use incular_config::{Alignment, Constraints, EdgeInsets};
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size,
};
use incular_platform::{PlatformEvent, WindowEvent, WindowOptions};
use incular_runtime::Application;
use incular_widgets::{
    Align, BackdropFilter, Center, ClipRRect, Color, Column, ConstrainedBox, Container,
    GestureDetector, HitTestBehavior, MainAxisSize, Opacity, Padding, Row, SizedBox, Stack, Widget,
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
            ..WindowOptions::new("layer hit test")
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

fn counter() -> (Rc<Cell<u32>>, impl Fn() + 'static) {
    let count = Rc::new(Cell::new(0));
    let bump = count.clone();
    (count, move || bump.set(bump.get() + 1))
}

/// A 40x40 panel that counts its taps.
fn panel(on_tap: impl Fn() + 'static) -> Widget {
    GestureDetector::new(Widget::box_(Size::new(40., 40.), Color::WHITE))
        .on_tap(on_tap)
        .into()
}

/// A click-away barrier under `layer`: clicking empty space at (10, 10)
/// must reach the barrier, clicking the panel in the bottom-right corner
/// must reach only the panel.
fn assert_layer_passes_through(label: &str, layer: impl Fn(Widget) -> Widget) {
    assert_layer_passes_through_at(label, Offset::new(190., 190.), layer);
}

fn assert_layer_passes_through_at(
    label: &str,
    on_panel_at: Offset,
    layer: impl Fn(Widget) -> Widget,
) {
    let (barrier_taps, on_barrier) = counter();
    let (panel_taps, on_panel) = counter();
    let barrier = GestureDetector::new(SizedBox::new().width(200.).height(200.))
        .behavior(HitTestBehavior::Opaque)
        .on_tap(on_barrier);
    let upper = Align::new(Alignment::BOTTOM_RIGHT, layer(panel(on_panel)));
    let mut application = app(incular_widgets::Directionality::new(
        incular_widgets::TextDirection::Ltr,
        Stack::new([Widget::from(barrier), upper.into()]),
    )
    .into());
    click(&mut application, Offset::new(10., 10.));
    assert_eq!(
        barrier_taps.get(),
        1,
        "{label}: empty space reaches the barrier"
    );
    assert_eq!(panel_taps.get(), 0, "{label}: empty space misses the panel");
    click(&mut application, on_panel_at);
    assert_eq!(panel_taps.get(), 1, "{label}: the panel gets its tap");
    assert_eq!(
        barrier_taps.get(),
        1,
        "{label}: the barrier misses the panel tap"
    );
}

#[test]
fn align_alone() {
    assert_layer_passes_through("Align", |panel| panel);
}

#[test]
fn padding() {
    assert_layer_passes_through("Padding", |panel| {
        Padding::new(EdgeInsets::all(4.), panel).into()
    });
}

#[test]
fn container_with_padding_and_margin() {
    assert_layer_passes_through("Container", |panel| {
        Container::new()
            .padding(EdgeInsets::all(4.))
            .margin(EdgeInsets::all(4.))
            .child(panel)
            .into()
    });
}

#[test]
fn full_height_column() {
    assert_layer_passes_through_at("Column", Offset::new(180., 40.), |panel| {
        Column::new([SizedBox::new().height(20.).into(), panel]).into()
    });
}

#[test]
fn row_with_min_size() {
    assert_layer_passes_through("Row", |panel| {
        Row::new([SizedBox::new().width(20.).into(), panel])
            .main_axis_size(MainAxisSize::Min)
            .into()
    });
}

#[test]
fn constrained_box() {
    assert_layer_passes_through("ConstrainedBox", |panel| {
        ConstrainedBox::new(
            Constraints::new(100., 200., 100., 200.),
            Align::new(Alignment::BOTTOM_RIGHT, panel),
        )
        .into()
    });
}

#[test]
fn center_in_a_sized_box() {
    assert_layer_passes_through("SizedBox(Center)", |panel| {
        SizedBox::new()
            .width(120.)
            .height(120.)
            .child(Align::new(Alignment::BOTTOM_RIGHT, panel))
            .into()
    });
}

#[test]
fn clip_and_blur() {
    assert_layer_passes_through("ClipRRect(BackdropFilter)", |panel| {
        SizedBox::new()
            .width(120.)
            .height(120.)
            .child(ClipRRect::new(
                8.,
                BackdropFilter::blur(4., Align::new(Alignment::BOTTOM_RIGHT, panel)),
            ))
            .into()
    });
}

#[test]
fn opacity() {
    assert_layer_passes_through("Opacity", |panel| {
        SizedBox::new()
            .width(120.)
            .height(120.)
            .child(Opacity::new(
                0.9,
                Align::new(Alignment::BOTTOM_RIGHT, panel),
            ))
            .into()
    });
}

#[test]
fn deferring_gesture_detector() {
    assert_layer_passes_through("GestureDetector(deferToChild)", |panel| {
        GestureDetector::new(
            SizedBox::new()
                .width(120.)
                .height(120.)
                .child(Align::new(Alignment::BOTTOM_RIGHT, panel)),
        )
        .on_tap(|| {})
        .into()
    });
}

#[test]
fn center_and_padding_nested() {
    assert_layer_passes_through_at(
        "SizedBox(Center(Padding))",
        Offset::new(140., 140.),
        |panel| {
            SizedBox::new()
                .width(120.)
                .height(120.)
                .child(Align::new(
                    Alignment::BOTTOM_RIGHT,
                    Padding::new(EdgeInsets::all(0.), Center::new(panel)),
                ))
                .into()
        },
    );
}

#[test]
fn inner_gesture_detector_wins_over_its_ancestor() {
    let (outer_taps, on_outer) = counter();
    let (inner_taps, on_inner) = counter();
    let tree = GestureDetector::new(
        Container::new()
            .color(Color::BLACK)
            .child(Center::new(panel(on_inner))),
    )
    .on_tap(on_outer);
    let mut application = app(tree.into());
    click(&mut application, Offset::new(100., 100.));
    assert_eq!(inner_taps.get(), 1, "the inner detector gets the tap");
    assert_eq!(outer_taps.get(), 0, "the outer detector loses the arena");
    click(&mut application, Offset::new(10., 10.));
    assert_eq!(
        outer_taps.get(),
        1,
        "outside the inner child the outer wins"
    );
    assert_eq!(inner_taps.get(), 1);
}

#[test]
fn button_inside_a_gesture_detector_wins() {
    let (outer_taps, on_outer) = counter();
    let (button_taps, on_button) = counter();
    let tree = GestureDetector::new(
        Container::new().color(Color::BLACK).child(Center::new(
            incular_widgets::internal::ActionSurface::new("go")
                .size(Size::new(40., 40.))
                .on_press(on_button),
        )),
    )
    .on_tap(on_outer);
    let mut application =
        app(incular_widgets::Directionality::new(incular_widgets::TextDirection::Ltr, tree).into());
    click(&mut application, Offset::new(100., 100.));
    assert_eq!(
        (button_taps.get(), outer_taps.get()),
        (1, 0),
        "the button gets the tap"
    );
    assert_eq!(
        outer_taps.get(),
        0,
        "the outer detector loses to the button"
    );
}

fn pointer(application: &mut Application, phase: PointerPhase, position: Offset) {
    let id = application.primary_window();
    application.handle_window_event(WindowEvent::platform(
        id,
        PlatformEvent::Input(InputEvent::PointerWithMetadata {
            pointer: 0,
            device: 1,
            kind: PointerDeviceKind::Mouse,
            buttons: if phase == PointerPhase::Up {
                0
            } else {
                PRIMARY_POINTER_BUTTON
            },
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

#[test]
fn a_drag_from_a_button_goes_to_the_outer_pan_and_cancels_the_button() {
    let (outer_taps, on_outer) = counter();
    let (pans, on_pan) = counter();
    let (button_taps, on_button) = counter();
    let tree = GestureDetector::new(
        Container::new().color(Color::BLACK).child(Center::new(
            incular_widgets::internal::ActionSurface::new("go")
                .size(Size::new(40., 40.))
                .on_press(on_button),
        )),
    )
    .on_tap(on_outer)
    .on_pan_update(move |_| on_pan());
    let mut application =
        app(incular_widgets::Directionality::new(incular_widgets::TextDirection::Ltr, tree).into());
    pointer(
        &mut application,
        PointerPhase::Down,
        Offset::new(100., 100.),
    );
    for step in 1..=4 {
        pointer(
            &mut application,
            PointerPhase::Move,
            Offset::new(100. + 10. * step as f32, 100.),
        );
    }
    pointer(&mut application, PointerPhase::Up, Offset::new(140., 100.));
    assert!(pans.get() > 0, "the outer pan wins the drag");
    assert_eq!(button_taps.get(), 0, "the button is cancelled");
    assert_eq!(outer_taps.get(), 0);
    click(&mut application, Offset::new(100., 100.));
    assert_eq!(
        (button_taps.get(), outer_taps.get()),
        (1, 0),
        "a later tap still reaches the button"
    );
}
