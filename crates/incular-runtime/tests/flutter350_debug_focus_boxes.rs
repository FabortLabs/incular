#![cfg(debug_assertions)]

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Size};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{Column, Directionality, Focus, FocusNode, SizedBox, Widget};

const PRIMARY: Color = Color::from_argb32(0xF000FF00);
const ANCESTOR: Color = Color::from_argb32(0xF00000FF);
const FOCUSABLE: Color = Color::from_argb32(0xF000FFFF);
const SKIPPED: Color = Color::from_argb32(0xF0FFFF00);
const NOT_FOCUSABLE: Color = Color::from_argb32(0xF0FF0000);

fn boxed(node: &FocusNode, extent: f32) -> Focus {
    Focus::new(SizedBox::square(extent)).node(node.clone())
}

fn scene(child: impl Into<Widget>) -> Runtime {
    Runtime::new(Directionality::new(TextDirection::Ltr, child).into()).unwrap()
}

fn draw(runtime: &mut Runtime) -> DisplayList {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap()
        .0
}

fn colors(list: &DisplayList) -> Vec<Color> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Border { border, .. } => {
                assert_eq!(border.width, 3.);
                Some(border.color)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn enabled_focus_boxes_draw_the_five_foreground_colors_at_three_pixels() {
    let (primary, parent, focusable, skip, unavailable) = (
        FocusNode::new(),
        FocusNode::new(),
        FocusNode::new(),
        FocusNode::new(),
        FocusNode::new(),
    );
    skip.set_skip_traversal(true);
    unavailable.set_can_request_focus(false);
    let mut runtime = scene(Column::new([
        Widget::from(Focus::new(boxed(&primary, 10.)).node(parent)),
        boxed(&focusable, 10.).into(),
        boxed(&skip, 10.).into(),
        boxed(&unavailable, 10.).into(),
    ]));
    runtime.set_debug_paint_focus_boxes(true);
    let _ = draw(&mut runtime);
    primary.request_focus();
    let borders = colors(&draw(&mut runtime));
    assert_eq!(
        borders,
        [PRIMARY, ANCESTOR, FOCUSABLE, SKIPPED, NOT_FOCUSABLE]
    );
}

#[test]
fn ordinary_focus_never_paints_a_border_and_disabling_drops_retained_debug_ink() {
    let node = FocusNode::new();
    let mut runtime = scene(boxed(&node, 100.));
    assert!(colors(&draw(&mut runtime)).is_empty());

    runtime.set_debug_paint_focus_boxes(true);
    assert_eq!(colors(&draw(&mut runtime)), [FOCUSABLE]);
    runtime.set_debug_paint_focus_boxes(false);
    assert!(colors(&draw(&mut runtime)).is_empty());
    node.request_focus();
    assert!(colors(&draw(&mut runtime)).is_empty());
    assert!(node.has_primary_focus());
}

#[test]
fn focus_boxes_update_as_focus_and_traversal_permissions_change() {
    let node = FocusNode::new();
    let mut runtime = scene(boxed(&node, 100.));
    runtime.set_debug_paint_focus_boxes(true);
    assert_eq!(colors(&draw(&mut runtime)), [FOCUSABLE]);
    node.request_focus();
    assert_eq!(colors(&draw(&mut runtime)), [PRIMARY]);
    node.set_skip_traversal(true);
    assert_eq!(colors(&draw(&mut runtime)), [PRIMARY]);
    node.unfocus();
    assert_eq!(colors(&draw(&mut runtime)), [SKIPPED]);
    node.set_can_request_focus(false);
    assert_eq!(colors(&draw(&mut runtime)), [NOT_FOCUSABLE]);
}

#[test]
fn the_mixed_focus_state_scene_still_paints_and_lays_out() {
    let primary = FocusNode::new();
    let unavailable = FocusNode::new();
    let skip = FocusNode::new();
    unavailable.set_can_request_focus(false);
    skip.set_skip_traversal(true);
    let mut runtime = scene(Column::new([
        Widget::from(boxed(&primary, 10.)),
        boxed(&unavailable, 10.).into(),
        boxed(&skip, 10.).into(),
    ]));
    runtime.set_debug_paint_focus_boxes(true);
    let _ = draw(&mut runtime);
    primary.request_focus();
    assert_eq!(
        colors(&draw(&mut runtime)),
        [PRIMARY, NOT_FOCUSABLE, SKIPPED]
    );
}
