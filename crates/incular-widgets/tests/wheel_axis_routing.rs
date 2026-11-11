//! Mouse wheel routing across scroll axes, and the scrollbar of a horizontal
//! scroll view.
//!
//! Each viewport reads the wheel on its own axis (Flutter
//! `ScrollableState._pointerSignalEventDelta`): a vertical wheel over a
//! horizontal list scrolls the vertical page around it, Shift turns the
//! wheel horizontal, and a horizontal-only strip with nothing vertical
//! around it scrolls with a plain wheel.

mod common;

use common::*;
use incular_config::{Axis, Constraints};
use incular_core::{Color, Offset, PointerPhase, Size};
use incular_rendering::PaintCommand;
use incular_scroll::ScrollController;
use incular_widgets::{Column, Row, SingleChildScrollView};

fn strip(controller: &ScrollController) -> Widget {
    SingleChildScrollView::new(Row::new(
        (0..10).map(|index| Widget::box_(Size::new(100., 80.), Color::WHITE).with_key(index)),
    ))
    .scroll_direction(Axis::Horizontal)
    .controller(controller.clone())
    .into()
}

/// A vertical page whose first child is a horizontal strip at y 0..80.
fn page(page: &ScrollController, row: &ScrollController) -> WidgetTree {
    let mut tree = WidgetTree::new();
    tree.mount(
        incular_widgets::Directionality::new(
            incular_widgets::TextDirection::Ltr,
            SingleChildScrollView::new(Column::new([
                strip(row),
                Widget::box_(Size::new(300., 2_000.), Color::BLACK),
            ]))
            .controller(page.clone()),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(300., 400.)))
        .expect("layout");
    tree
}

#[test]
fn vertical_wheel_over_a_horizontal_strip_scrolls_the_page() {
    let (vertical, horizontal) = (ScrollController::new(), ScrollController::new());
    let mut tree = page(&vertical, &horizontal);
    assert!(tree.scroll_at(Offset::new(150., 40.), Offset::new(0., 30.)));
    assert_eq!(vertical.offset(), 30.);
    assert_eq!(horizontal.offset(), 0.);
}

#[test]
fn horizontal_wheel_over_the_strip_scrolls_the_strip() {
    let (vertical, horizontal) = (ScrollController::new(), ScrollController::new());
    let mut tree = page(&vertical, &horizontal);
    assert!(tree.scroll_at(Offset::new(150., 40.), Offset::new(25., 0.)));
    assert_eq!(horizontal.offset(), 25.);
    assert_eq!(vertical.offset(), 0.);
}

#[test]
fn shift_wheel_over_the_strip_scrolls_it_horizontally() {
    let (vertical, horizontal) = (ScrollController::new(), ScrollController::new());
    let mut tree = page(&vertical, &horizontal);
    assert!(tree.scroll_at_with_axis_flip(Offset::new(150., 40.), Offset::new(0., 30.), true));
    assert_eq!(horizontal.offset(), 30.);
    assert_eq!(vertical.offset(), 0.);
}

#[test]
fn plain_wheel_scrolls_a_horizontal_strip_with_nothing_vertical_around_it() {
    let horizontal = ScrollController::new();
    let mut tree = WidgetTree::new();
    tree.mount(
        incular_widgets::Directionality::new(
            incular_widgets::TextDirection::Ltr,
            strip(&horizontal),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(300., 80.)))
        .expect("layout");
    assert!(tree.scroll_at(Offset::new(150., 40.), Offset::new(0., 30.)));
    assert_eq!(horizontal.offset(), 30.);
}

#[test]
fn a_horizontal_scroll_view_has_a_horizontal_thumb_along_its_bottom() {
    let horizontal = ScrollController::new();
    horizontal.set_scrollbar_thumb_visibility(true);
    let mut tree = WidgetTree::new();
    tree.mount(
        incular_widgets::Directionality::new(
            incular_widgets::TextDirection::Ltr,
            strip(&horizontal),
        )
        .into(),
    )
    .unwrap();
    tree.layout(Constraints::tight(Size::new(300., 80.)))
        .expect("layout");
    settle_scrollbar_fade(&mut tree);
    let geometry = tree.scrollbar_diagnostics().pop().expect("scrollbar");
    assert!(geometry.visible);
    assert_eq!(geometry.axis, Axis::Horizontal);
    assert!(geometry.thumb.size.width > geometry.thumb.size.height);
    assert_eq!(
        geometry.thumb.origin.y + geometry.thumb.size.height,
        80.,
        "the thumb runs along the bottom edge"
    );
    let list = tree.paint();
    let thumbs: Vec<_> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::RRect { rrect, .. } => Some(rrect.rect),
            _ => None,
        })
        .collect();
    assert!(
        thumbs
            .iter()
            .any(|rect| rect.size.width > rect.size.height && rect.size.height <= 6.),
        "a horizontal thumb is painted: {thumbs:?}"
    );
    assert!(
        !thumbs
            .iter()
            .any(|rect| rect.size.height > rect.size.width && rect.size.width <= 6.),
        "no vertical thumb is painted: {thumbs:?}"
    );

    // Dragging the thumb sideways scrolls the strip.
    let start = Offset::new(
        geometry.thumb.origin.x + geometry.thumb.size.width / 2.,
        geometry.thumb.origin.y + geometry.thumb.size.height / 2.,
    );
    assert!(tree.scrollbar_pointer(PointerPhase::Down, start));
    assert!(tree.scrollbar_pointer(PointerPhase::Move, start + Offset::new(40., 0.)));
    assert!(tree.scrollbar_pointer(PointerPhase::Up, start + Offset::new(40., 0.)));
    assert!(horizontal.offset() > 0., "offset {}", horizontal.offset());
}
