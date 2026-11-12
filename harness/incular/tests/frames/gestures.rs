//! Gesture frame scenarios. Each one has a twin with the same name in
//! harness/flutter/test/frames/gestures_test.dart; keep them identical.

use std::rc::Rc;
use std::{cell::Cell, time::Duration};

use incular_core::{Color, DragStartBehavior, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    Center, ColoredBox, DismissDirection, Dismissible, DragDropContext, DragTarget, Draggable,
    GestureDetector, InteractiveViewer, LongPressDraggable, Positioned,
    ReorderableDelayedDragStartListener, ReorderableList, SizedBox, Stack,
    TransformationController, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("gestures__tap", tap),
        ("gestures__double_tap", double_tap),
        ("gestures__long_press", long_press),
        ("gestures__vertical_drag_down", vertical_drag_down),
        (
            "gestures__scale_single_pointer_pan",
            scale_single_pointer_pan,
        ),
        (
            "gestures__scale__two_pointer_pinch",
            scale_two_pointer_pinch,
        ),
        ("gestures__draggable_drop_target", draggable_drop_target),
        (
            "gestures__long_press_draggable_quick_move",
            long_press_draggable_quick_move,
        ),
        ("gestures__dismissible_threshold", dismissible_threshold),
        (
            "gestures__interactive_viewer_mouse_wheel_zoom",
            interactive_viewer_mouse_wheel_zoom,
        ),
        (
            "gestures__reorderable_list_long_press",
            reorderable_list_long_press,
        ),
    ]
}

fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn tap(h: &mut FrameHarness) {
    let taps = Rc::new(Cell::new(0.0));
    let down_y = Rc::new(Cell::new(0.0));
    let read_taps = taps.clone();
    let read_down_y = down_y.clone();
    h.value("tap.count", move || read_taps.get());
    h.value("tap.down_y", move || read_down_y.get());
    let tap = taps.clone();
    let down = down_y.clone();
    h.mount(move |_| {
        let tap = tap.clone();
        let down = down.clone();
        Center::new(probe(
            "target",
            Widget::from(
                GestureDetector::new(SizedBox::new().width(120.).height(80.).child(colored(BLUE)))
                    .on_tap(move || tap.set(tap.get() + 1.))
                    .on_tap_down(move |details| down.set(f64::from(details.global_position.y))),
            ),
        ))
        .into()
    });
    h.frames(1);
    h.down(Offset::new(400., 300.));
    h.up();
    h.frames(1);
}

fn double_tap(h: &mut FrameHarness) {
    let double_taps = Rc::new(Cell::new(0.0));
    let read = double_taps.clone();
    h.value("double_tap.count", move || read.get());
    let count = double_taps.clone();
    h.mount(move |_| {
        let count = count.clone();
        Center::new(probe(
            "target",
            Widget::from(
                GestureDetector::new(
                    SizedBox::new()
                        .width(120.)
                        .height(80.)
                        .child(colored(GREEN)),
                )
                .on_double_tap(move || count.set(count.get() + 1.)),
            ),
        ))
        .into()
    });
    h.frames(1);
    h.down(Offset::new(400., 300.));
    h.up();
    h.frames(3);
    h.down(Offset::new(400., 300.));
    h.up();
    h.frames(4);
}

fn long_press(h: &mut FrameHarness) {
    let count = Rc::new(Cell::new(0.0));
    let start_y = Rc::new(Cell::new(0.0));
    let read_count = count.clone();
    let read_start_y = start_y.clone();
    h.value("long_press.count", move || read_count.get());
    h.value("long_press.start_y", move || read_start_y.get());
    let pressed = count.clone();
    let start = start_y.clone();
    h.mount(move |_| {
        let pressed = pressed.clone();
        let start = start.clone();
        Center::new(probe(
            "target",
            Widget::from(
                GestureDetector::new(SizedBox::new().width(120.).height(80.).child(colored(BLUE)))
                    .on_long_press(move || pressed.set(pressed.get() + 1.))
                    .on_long_press_start(move |details| {
                        start.set(f64::from(details.global_position.y));
                    }),
            ),
        ))
        .into()
    });
    h.frames(1);
    h.down(Offset::new(400., 300.));
    h.frames(32);
    h.up();
    h.frames(1);
}

fn vertical_drag_down(h: &mut FrameHarness) {
    let start_y = Rc::new(Cell::new(0.0));
    let delta_y = Rc::new(Cell::new(0.0));
    let end_velocity_y = Rc::new(Cell::new(0.0));
    let read_start_y = start_y.clone();
    let read_delta_y = delta_y.clone();
    let read_velocity_y = end_velocity_y.clone();
    h.value("drag.start_y", move || read_start_y.get());
    h.value("drag.delta_y", move || read_delta_y.get());
    h.value("drag.end_velocity_y", move || read_velocity_y.get());
    let start = start_y.clone();
    let delta = delta_y.clone();
    let velocity = end_velocity_y.clone();
    h.mount(move |_| {
        let start = start.clone();
        let delta = delta.clone();
        let velocity = velocity.clone();
        Center::new(probe(
            "target",
            Widget::from(
                GestureDetector::new(
                    SizedBox::new()
                        .width(160.)
                        .height(120.)
                        .child(colored(GREEN)),
                )
                .drag_start_behavior(DragStartBehavior::Down)
                .on_tap(|| {})
                .on_vertical_drag_start(move |details| {
                    start.set(f64::from(details.global_position.y));
                })
                .on_vertical_drag_update(move |details| {
                    delta.set(delta.get() + f64::from(details.delta.y));
                })
                .on_vertical_drag_end(move |details| {
                    velocity.set(f64::from(details.velocity.pixels_per_second.y));
                }),
            ),
        ))
        .into()
    });
    h.frames(1);
    h.down(Offset::new(400., 300.));
    h.frames(1);
    h.move_to(Offset::new(400., 290.));
    h.frames(1);
    h.move_to(Offset::new(400., 280.));
    h.frames(1);
    h.move_to(Offset::new(400., 250.));
    h.frames(1);
    h.up();
    h.frames(1);
}

fn scale_single_pointer_pan(h: &mut FrameHarness) {
    let starts = Rc::new(Cell::new(0.0));
    let updates = Rc::new(Cell::new(0.0));
    let focal_y = Rc::new(Cell::new(0.0));
    let read_starts = starts.clone();
    let read_updates = updates.clone();
    let read_focal_y = focal_y.clone();
    h.value("scale.starts", move || read_starts.get());
    h.value("scale.updates", move || read_updates.get());
    h.value("scale.focal_y", move || read_focal_y.get());
    let start = starts.clone();
    let update = updates.clone();
    let focal = focal_y.clone();
    h.mount(move |_| {
        let start = start.clone();
        let update = update.clone();
        let focal = focal.clone();
        Center::new(probe(
            "target",
            Widget::from(
                GestureDetector::new(
                    SizedBox::new()
                        .width(200.)
                        .height(160.)
                        .child(colored(BLUE)),
                )
                .on_scale_start(move |_| start.set(start.get() + 1.))
                .on_scale_update(move |details| {
                    update.set(update.get() + 1.);
                    focal.set(f64::from(details.focal_point.y));
                }),
            ),
        ))
        .into()
    });
    h.frames(1);
    h.down(Offset::new(400., 300.));
    h.frames(1);
    h.move_to(Offset::new(400., 250.));
    h.frames(1);
    h.move_to(Offset::new(400., 220.));
    h.frames(1);
    h.up();
    h.frames(1);
}

fn scale_two_pointer_pinch(h: &mut FrameHarness) {
    let scale = Rc::new(Cell::new(1.0));
    let pointers = Rc::new(Cell::new(0.0));
    h.value("scale", {
        let scale = Rc::clone(&scale);
        move || scale.get()
    });
    h.value("pointers", {
        let pointers = Rc::clone(&pointers);
        move || pointers.get()
    });
    h.mount(move |_| {
        let scale = Rc::clone(&scale);
        let pointers = Rc::clone(&pointers);
        Center::new(probe(
            "target",
            Widget::from(
                GestureDetector::new(
                    SizedBox::new()
                        .width(200.)
                        .height(160.)
                        .child(colored(BLUE)),
                )
                .on_scale_update(move |details| {
                    scale.set(f64::from(details.scale));
                    pointers.set(details.pointer_count as f64);
                }),
            ),
        ))
        .into()
    });
    h.down(Offset::new(370., 300.));
    h.frames(1);
    h.second_down(Offset::new(430., 300.));
    h.frames(1);
    h.second_move_to(Offset::new(460., 300.));
    h.frames(1);
    h.move_to(Offset::new(350., 300.));
    h.frames(1);
    h.second_up();
    h.frames(1);
    h.up();
    h.frames(1);
}

fn draggable_drop_target(h: &mut FrameHarness) {
    let context = DragDropContext::new();
    let drag_starts = Rc::new(Cell::new(0.0));
    let accepted_data = Rc::new(Cell::new(0.0));
    let starts_read = drag_starts.clone();
    let accepted_read = accepted_data.clone();
    h.value("drag.started", move || starts_read.get());
    h.value("target.accepted_data", move || accepted_read.get());

    let target_context = context.clone();
    let accepted = accepted_data.clone();
    let drag_starts = drag_starts.clone();
    h.mount(move |_| {
        let accepted = accepted.clone();
        let starts = drag_starts.clone();
        let target = DragTarget::new(
            target_context.clone(),
            probe(
                "drop_target",
                SizedBox::new()
                    .width(120.)
                    .height(80.)
                    .child(colored(GREEN)),
            ),
        )
        .on_accept_with_details(move |payload, _| accepted.set(f64::from(payload)));
        let source: Widget = Draggable::new(
            context.clone(),
            7_i32,
            probe(
                "drag_source",
                SizedBox::new().width(80.).height(60.).child(colored(BLUE)),
            ),
        )
        .feedback(|_| {
            SizedBox::new()
                .width(40.)
                .height(40.)
                .child(colored(BLUE))
                .into()
        })
        .on_start(move |_| starts.set(starts.get() + 1.))
        .into();
        Stack::new(vec![
            Positioned::new(target).left(500.).top(250.),
            Positioned::new(source).left(100.).top(250.),
        ])
        .into()
    });
    h.frames(1);
    let start = h.center("drag_source");
    h.down(start);
    h.frames(1);
    h.move_to(Offset::new(160., 280.));
    h.frames(1);
    h.move_to(Offset::new(560., 290.));
    h.frames(1);
    h.up();
    h.frames(1);
}

fn long_press_draggable_quick_move(h: &mut FrameHarness) {
    let context = DragDropContext::new();
    let context_read = context.clone();
    h.value("drag.started", move || {
        if context_read.is_dragging() { 1.0 } else { 0.0 }
    });
    h.mount(move |_| {
        Center::new(probe(
            "drag_source",
            Widget::from(LongPressDraggable::new(
                context.clone(),
                7_i32,
                SizedBox::new().width(120.).height(80.).child(colored(BLUE)),
            )),
        ))
        .into()
    });
    h.frames(1);
    let start = h.center("drag_source");
    h.down(start);
    h.pump(Duration::from_millis(100));
    h.move_to(start + Offset::new(40., 0.));
    h.frames(1);
    h.up();
    h.frames(1);
}

fn dismissible_threshold(h: &mut FrameHarness) {
    let progress = Rc::new(Cell::new(0.0));
    let reached = Rc::new(Cell::new(0.0));
    let dismissed = Rc::new(Cell::new(0.0));
    h.value("dismiss.progress", {
        let progress = progress.clone();
        move || progress.get()
    });
    h.value("dismiss.reached", {
        let reached = reached.clone();
        move || reached.get()
    });
    h.value("dismiss.count", {
        let dismissed = dismissed.clone();
        move || dismissed.get()
    });
    h.mount(move |_| {
        let progress = progress.clone();
        let reached = reached.clone();
        let dismissed = dismissed.clone();
        Center::new(
            Dismissible::new(
                DismissDirection::Horizontal,
                probe(
                    "dismissible",
                    SizedBox::new()
                        .width(160.)
                        .height(80.)
                        .child(colored(GREEN)),
                ),
                move |_| dismissed.set(dismissed.get() + 1.),
            )
            .on_update(move |details| {
                progress.set(f64::from(details.progress));
                reached.set(if details.reached { 1.0 } else { 0.0 });
            }),
        )
        .into()
    });
    h.frames(1);
    let start = h.center("dismissible");
    h.down(start);
    h.frames(1);
    h.move_to(start + Offset::new(100., 0.));
    h.frames(1);
    h.up();
    h.pump(Duration::from_millis(600));
}

fn interactive_viewer_mouse_wheel_zoom(h: &mut FrameHarness) {
    let controller = TransformationController::new();
    let controller_read = controller.clone();
    h.value("viewer.scale", move || {
        controller_read.value().coefficients()[0]
    });
    let controller_read = controller.clone();
    h.value("viewer.translate_x", move || {
        controller_read.value().coefficients()[4]
    });
    let controller_read = controller.clone();
    h.value("viewer.translate_y", move || {
        controller_read.value().coefficients()[5]
    });
    h.mount(move |_| {
        Center::new(probe(
            "viewer",
            SizedBox::new().width(200.).height(160.).child(
                InteractiveViewer::new(
                    SizedBox::new()
                        .width(400.)
                        .height(320.)
                        .child(colored(BLUE)),
                )
                .constrained(false)
                .transformation_controller(controller.clone()),
            ),
        ))
        .into()
    });
    h.frames(1);
    h.wheel(Offset::new(400., 300.), Offset::new(0., -40.));
    h.frames(1);
}

fn reorderable_list_long_press(h: &mut FrameHarness) {
    let started_index = Rc::new(Cell::new(-1_i32));
    let ended_index = Rc::new(Cell::new(-1_i32));
    let reorder_from = Rc::new(Cell::new(-1_i32));
    let reorder_to = Rc::new(Cell::new(-1_i32));
    h.value("reorder.started_index", {
        let value = started_index.clone();
        move || f64::from(value.get())
    });
    h.value("reorder.ended_index", {
        let value = ended_index.clone();
        move || f64::from(value.get())
    });
    h.value("reorder.from", {
        let value = reorder_from.clone();
        move || f64::from(value.get())
    });
    h.value("reorder.to", {
        let value = reorder_to.clone();
        move || f64::from(value.get())
    });
    h.mount(move |_| {
        let started_index = started_index.clone();
        let ended_index = ended_index.clone();
        let reorder_from = reorder_from.clone();
        let reorder_to = reorder_to.clone();
        SizedBox::new()
            .width(180.)
            .height(240.)
            .child(
                ReorderableList::new(3, |index| {
                    let name = match index {
                        0 => "reorder_row_0",
                        1 => "reorder_row_1",
                        _ => "reorder_row_2",
                    };
                    ReorderableDelayedDragStartListener::new(
                        index,
                        probe(
                            name,
                            SizedBox::new()
                                .width(180.)
                                .height(60.)
                                .child(colored(if index % 2 == 0 { BLUE } else { GREEN })),
                        ),
                    )
                })
                .item_extent(60.)
                .on_reorder({
                    let from = reorder_from.clone();
                    let to = reorder_to.clone();
                    move |old, new| {
                        from.set(old as i32);
                        to.set(new as i32);
                    }
                })
                .on_reorder_start(move |index| started_index.set(index as i32))
                .on_reorder_end(move |index| ended_index.set(index as i32)),
            )
            .into()
    });
    h.frames(1);
    let start = h.center("reorder_row_0");
    h.down(start);
    h.pump(Duration::from_millis(600));
    h.move_to(start + Offset::new(0., 150.));
    h.frames(2);
    h.up();
    h.frames(1);
}
