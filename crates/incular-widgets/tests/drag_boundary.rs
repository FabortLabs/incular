//! Flutter's `DragBoundary` and its rectangle delegate.

use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, Offset, Rect, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Builder, DragBoundary, DragBoundaryDelegate, Padding, RectDragBoundary, Widget,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

/// Mounts a 100 x 80 boundary inset by 10 and returns the delegates a
/// descendant obtained, global and local, with the tree that publishes the
/// boundary.
fn delegates() -> (WidgetTree, RectDragBoundary, RectDragBoundary) {
    let found = Rc::new(RefCell::new(None));
    let builder = {
        let found = found.clone();
        Builder::new(move |context| {
            *found.borrow_mut() = Some((
                DragBoundary::for_rect_of(context, true),
                DragBoundary::for_rect_of(context, false),
            ));
            Widget::box_(Size::new(100., 80.), Color::WHITE)
        })
    };
    let mut tree = WidgetTree::new();
    tree.mount(Padding::new(EdgeInsets::all(10.), DragBoundary::new(builder)).into())
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(500., 500.)))
        .expect("layout");
    let _ = tree.paint();
    tree.update_compositor(Instant::now()).expect("compositor");
    let (global, local) = found.borrow_mut().take().expect("built");
    (tree, global, local)
}

#[test]
fn the_boundary_is_the_widgets_box() {
    let (_tree, global, local) = delegates();
    assert_eq!(global.boundary(), Some(rect(10., 10., 100., 80.)));
    assert_eq!(local.boundary(), Some(rect(0., 0., 100., 80.)));
}

#[test]
fn dragged_rects_are_kept_inside() {
    let (_tree, global, _) = delegates();
    assert!(global.is_within_boundary(&rect(20., 20., 10., 10.)));
    assert!(!global.is_within_boundary(&rect(5., 20., 10., 10.)));
    // Clamped to left 10 and to bottom 90 - 10.
    assert_eq!(
        global.nearest_position_within_boundary(rect(-5., 85., 10., 10.)),
        rect(10., 80., 10., 10.)
    );
}

#[test]
fn without_a_boundary_everything_is_allowed() {
    let found = Rc::new(RefCell::new(None));
    let builder = {
        let found = found.clone();
        Builder::new(move |context| {
            *found.borrow_mut() = Some(DragBoundary::for_rect_of(context, true));
            Widget::box_(Size::new(10., 10.), Color::WHITE)
        })
    };
    let mut tree = WidgetTree::new();
    tree.mount(builder.into()).expect("mount");
    tree.layout(Constraints::loose(Size::new(50., 50.)))
        .expect("layout");
    let delegate = found.borrow_mut().take().expect("built");
    let dragged = rect(-100., -100., 10., 10.);
    assert!(delegate.is_within_boundary(&dragged));
    assert_eq!(delegate.nearest_position_within_boundary(dragged), dragged);
}
