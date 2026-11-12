use std::{cell::Cell, rc::Rc, time::Duration};

use incular_animation::Curves;
use incular_core::Color;
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    ColoredBox, ListView, PageController, PageView, ScrollController, ScrollNotificationType,
    SizedBox, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("scroll__controller__jump_clamp", controller_jump_clamp),
        (
            "scroll__controller__jump_activity_notifications",
            controller_jump_activity_notifications,
        ),
        (
            "scroll__controller__animate_retarget",
            controller_animate_retarget,
        ),
        (
            "scroll__page_view__fraction_initial_page",
            page_view_fraction_initial_page,
        ),
    ]
}

fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn controller_jump_clamp(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let read = controller.clone();
    h.value("offset", move || f64::from(read.offset()));
    let read = controller.clone();
    h.value("max", move || f64::from(read.max_offset()));
    let view_controller = controller.clone();
    h.mount(move |_| {
        ListView::builder(40, |index| {
            let row = colored(if index % 2 == 0 { BLUE } else { GREEN });
            if index == 0 {
                probe("row0", row)
            } else if index == 5 {
                probe("row5", row)
            } else {
                row
            }
        })
        .item_extent(50.)
        .controller(view_controller.clone())
        .into()
    });
    h.frames(1);
    controller.jump_to(125.);
    h.frames(1);
    controller.jump_to(100_000.);
    h.frames(3);
}

fn controller_jump_activity_notifications(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    let starts = Rc::new(Cell::new(0_u32));
    let updates = Rc::new(Cell::new(0_u32));
    let ends = Rc::new(Cell::new(0_u32));
    h.value("notifications.start", {
        let starts = starts.clone();
        move || f64::from(starts.get())
    });
    h.value("notifications.update", {
        let updates = updates.clone();
        move || f64::from(updates.get())
    });
    h.value("notifications.end", {
        let ends = ends.clone();
        move || f64::from(ends.get())
    });
    let _subscription = controller.add_notification_listener(move |notification| {
        match notification.kind {
            ScrollNotificationType::Start => starts.set(starts.get() + 1),
            ScrollNotificationType::Update => updates.set(updates.get() + 1),
            ScrollNotificationType::End => ends.set(ends.get() + 1),
            _ => {}
        }
        false
    });
    let view_controller = controller.clone();
    h.mount(move |_| {
        Widget::from(
            ListView::builder(40, |index| {
                ColoredBox::new(
                    if index % 2 == 0 { BLUE } else { GREEN },
                    SizedBox::shrink(),
                )
            })
            .item_extent(50.)
            .controller(view_controller.clone()),
        )
    });
    h.frames(1);
    controller.jump_to(125.);
    h.frames(1);
}

fn controller_animate_retarget(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let read = controller.clone();
    h.value("offset", move || f64::from(read.offset()));
    let view_controller = controller.clone();
    h.mount(move |_| {
        ListView::builder(80, |index| {
            let row = colored(if index % 2 == 0 { BLUE } else { ORANGE });
            if index == 0 {
                probe("row0", row)
            } else if index == 8 {
                probe("row8", row)
            } else {
                row
            }
        })
        .item_extent(50.)
        .controller(view_controller.clone())
        .into()
    });
    h.frames(1);
    controller.animate_to(600., Duration::from_millis(320), Curves::FAST_OUT_SLOW_IN);
    h.frames(10);
    controller.animate_to(250., Duration::from_millis(160), Curves::LINEAR);
    h.frames(12);
}

fn page_view_fraction_initial_page(h: &mut FrameHarness) {
    let controller = PageController::new();
    controller.deferred_jump_to(400.);
    let read = controller.clone();
    h.value("page", move || f64::from(read.offset() / 400.));
    h.mount(move |_| {
        PageView::new([
            probe("page0", colored(BLUE)),
            probe("page1", colored(ORANGE)),
            probe("page2", colored(GREEN)),
        ])
        .controller(controller.clone())
        .viewport_fraction(0.5)
        .into()
    });
    h.frames(3);
}
