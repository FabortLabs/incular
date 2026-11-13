use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::Curves;
use incular_core::{Color, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_scroll::ScrollPhysics;
use incular_widgets::{
    ChangeReportingBehavior, ColoredBox, Column, DraggableScrollableController,
    DraggableScrollableSheet, ListView, ListWheelScrollView, NestedScrollView, PageController,
    PageStorage, PageStorageBucket, PageStorageKey, PageView, ScrollController,
    ScrollNotificationType, SingleChildScrollView, SizedBox, Sliver, SliverToBoxAdapter,
    WheelChildDelegate, Widget,
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
            "scroll__controller__animate_activity_notifications",
            controller_animate_activity_notifications,
        ),
        (
            "scroll__page_view__fraction_initial_page",
            page_view_fraction_initial_page,
        ),
        ("scroll__drag__notification_order", drag_notification_order),
        (
            "scroll__fling__notification_order",
            fling_notification_order,
        ),
        (
            "scroll__drag_overscroll__clamping",
            drag_overscroll_clamping,
        ),
        (
            "scroll__drag_overscroll__bouncing",
            drag_overscroll_bouncing,
        ),
        (
            "scroll__page_view__fling_page_changed",
            page_view_fling_page_changed,
        ),
        (
            "scroll__list_wheel__selected_fling",
            list_wheel_selected_fling,
        ),
        (
            "scroll__draggable_sheet__drag_extent",
            draggable_sheet_drag_extent,
        ),
        (
            "scroll__nested_scroll_view__drag_offsets",
            nested_scroll_view_drag_offsets,
        ),
        (
            "scroll__single_child__keep_offset_page_storage",
            single_child_keep_offset_page_storage,
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

fn controller_animate_activity_notifications(h: &mut FrameHarness) {
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
    controller.animate_to(300., Duration::from_millis(160), Curves::LINEAR);
    h.frames(8);
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

fn drag(h: &mut FrameHarness, from: Offset, to: Offset, steps: u32, frames_per_step: usize) {
    h.down(from);
    for step in 1..=steps {
        h.frames(frames_per_step);
        let t = step as f32 / steps as f32;
        h.move_to(Offset::new(
            from.x + (to.x - from.x) * t,
            from.y + (to.y - from.y) * t,
        ));
    }
    h.frames(1);
    h.up();
}

const NOTIFICATION_NAMES: [&str; 5] = [
    "notifications.start",
    "notifications.update",
    "notifications.overscroll",
    "notifications.end",
    "notifications.user_scroll",
];

/// Counts start, update, overscroll, end and user-scroll notifications. The
/// returned subscription must stay alive for the scenario.
fn watch_notifications(h: &mut FrameHarness, controller: &ScrollController) -> impl std::any::Any {
    let counts = Rc::new(RefCell::new([0_u32; 5]));
    for (index, name) in NOTIFICATION_NAMES.into_iter().enumerate() {
        let counts = counts.clone();
        h.value(name, move || f64::from(counts.borrow()[index]));
    }
    controller.add_notification_listener(move |notification| {
        let index = match notification.kind {
            ScrollNotificationType::Start => 0,
            ScrollNotificationType::Update => 1,
            ScrollNotificationType::Overscroll => 2,
            ScrollNotificationType::End => 3,
            ScrollNotificationType::UserScroll => 4,
            ScrollNotificationType::Metrics => return false,
        };
        counts.borrow_mut()[index] += 1;
        false
    })
}

fn counting_list(controller: ScrollController, physics: ScrollPhysics) -> Widget {
    ListView::builder(40, |index| {
        ColoredBox::new(
            if index % 2 == 0 { BLUE } else { GREEN },
            SizedBox::shrink(),
        )
    })
    .item_extent(50.)
    .controller(controller)
    .physics(physics)
    .into()
}

fn drag_notification_order(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let read = controller.clone();
    h.value("offset", move || f64::from(read.offset()));
    let _subscription = watch_notifications(h, &controller);
    let view = controller.clone();
    h.mount(move |_| counting_list(view.clone(), ScrollPhysics::clamping()));
    h.frames(1);
    drag(h, Offset::new(400., 500.), Offset::new(400., 200.), 5, 8);
    h.frames(20);
}

fn fling_notification_order(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let read = controller.clone();
    h.value("offset", move || f64::from(read.offset()));
    let _subscription = watch_notifications(h, &controller);
    let view = controller.clone();
    h.mount(move |_| counting_list(view.clone(), ScrollPhysics::clamping()));
    h.frames(1);
    drag(h, Offset::new(400., 500.), Offset::new(400., 200.), 3, 1);
    h.frames(90);
}

fn drag_overscroll(h: &mut FrameHarness, physics: ScrollPhysics) {
    let controller = ScrollController::new();
    let read = controller.clone();
    h.value("offset", move || f64::from(read.offset()));
    let _subscription = watch_notifications(h, &controller);
    let view = controller.clone();
    h.mount(move |_| counting_list(view.clone(), physics));
    h.frames(1);
    drag(h, Offset::new(400., 200.), Offset::new(400., 400.), 4, 1);
    h.frames(30);
}

fn drag_overscroll_clamping(h: &mut FrameHarness) {
    drag_overscroll(h, ScrollPhysics::clamping());
}

fn drag_overscroll_bouncing(h: &mut FrameHarness) {
    drag_overscroll(h, ScrollPhysics::BOUNCING);
}

fn page_view_fling_page_changed(h: &mut FrameHarness) {
    let controller = PageController::new();
    let read = controller.clone();
    h.value("page", move || f64::from(read.offset() / 800.));
    let changes = Rc::new(Cell::new(0_u32));
    let last_page = Rc::new(Cell::new(0_usize));
    let read_changes = changes.clone();
    h.value("page_changes", move || f64::from(read_changes.get()));
    let read_last = last_page.clone();
    h.value("last_page", move || read_last.get() as f64);
    // Workaround: a controller-level notification listener that reports each
    // rounded page change. This is the nearest Incular equivalent of Flutter's
    // onPageChanged.
    let _subscription = controller.add_notification_listener({
        let changes = changes.clone();
        let last_page = last_page.clone();
        move |notification| {
            if matches!(notification.kind, ScrollNotificationType::Update) {
                let page = (notification.metrics.pixels / 800.).round().max(0.) as usize;
                if page != last_page.get() {
                    last_page.set(page);
                    changes.set(changes.get() + 1);
                }
            }
            false
        }
    });
    let view = controller.clone();
    h.mount(move |_| {
        PageView::new([
            probe("page0", colored(BLUE)),
            probe("page1", colored(ORANGE)),
            probe("page2", colored(GREEN)),
        ])
        .controller(view.clone())
        .into()
    });
    h.frames(1);
    drag(h, Offset::new(600., 300.), Offset::new(150., 300.), 5, 1);
    h.frames(60);
}

fn list_wheel_selected_fling(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let read = controller.clone();
    h.value("offset", move || f64::from(read.offset()));
    let changes = Rc::new(Cell::new(0_u32));
    let selected = Rc::new(Cell::new(0_usize));
    let read_changes = changes.clone();
    h.value("selected_changes", move || f64::from(read_changes.get()));
    let read_selected = selected.clone();
    h.value("selected_index", move || read_selected.get() as f64);
    let view_controller = controller.clone();
    h.mount(move |_| {
        let children: Vec<Widget> = (0..40)
            .map(|index| {
                let row: Widget = SizedBox::new()
                    .width(300.)
                    .child(colored(if index % 2 == 0 { BLUE } else { GREEN }))
                    .into();
                if index == 0 { probe("item0", row) } else { row }
            })
            .collect();
        let mut view = ListWheelScrollView::new(
            view_controller.clone(),
            50.,
            WheelChildDelegate::children(children),
        );
        let changes = changes.clone();
        let selected = selected.clone();
        view.viewport_mut().set_selection_callback(
            ChangeReportingBehavior::OnScrollUpdate,
            Some(move |index: usize| {
                changes.set(changes.get() + 1);
                selected.set(index);
            }),
        );
        view.into()
    });
    h.frames(1);
    drag(h, Offset::new(400., 450.), Offset::new(400., 250.), 3, 1);
    h.frames(60);
}

fn draggable_sheet_drag_extent(h: &mut FrameHarness) {
    let slot: Rc<RefCell<Option<DraggableScrollableController>>> = Rc::new(RefCell::new(None));
    let read = slot.clone();
    h.value("size", move || {
        read.borrow()
            .as_ref()
            .and_then(DraggableScrollableController::size)
            .map_or(0.0, f64::from)
    });
    h.mount(move |_| {
        let sheet = DraggableScrollableSheet::new(|inner: &ScrollController| -> Widget {
            ListView::builder(40, |index| {
                let row: Widget = ColoredBox::new(
                    if index % 2 == 0 { BLUE } else { GREEN },
                    SizedBox::shrink(),
                )
                .into();
                if index == 0 { probe("item0", row) } else { row }
            })
            .item_extent(50.)
            .controller(inner.clone())
            .into()
        })
        .extents(0.25, 1.0, 0.5);
        *slot.borrow_mut() = Some(sheet.controller());
        sheet.into()
    });
    h.frames(1);
    drag(h, Offset::new(400., 500.), Offset::new(400., 300.), 4, 1);
    h.frames(40);
}

fn nested_scroll_view_drag_offsets(h: &mut FrameHarness) {
    let outer = ScrollController::new();
    let inner = ScrollController::new();
    let read = outer.clone();
    h.value("outer", move || f64::from(read.offset()));
    let read = inner.clone();
    h.value("inner", move || f64::from(read.offset()));
    let view_outer = outer.clone();
    let view_inner = inner.clone();
    h.mount(move |_| {
        let header: Vec<Box<dyn Sliver>> = vec![Box::new(SliverToBoxAdapter::new(
            SizedBox::new()
                .height(200.)
                .child(probe("header", colored(ORANGE))),
        ))];
        let body: Widget = ListView::builder(40, |index| {
            let row: Widget = ColoredBox::new(
                if index % 2 == 0 { BLUE } else { GREEN },
                SizedBox::shrink(),
            )
            .into();
            if index == 0 { probe("item0", row) } else { row }
        })
        .item_extent(50.)
        .controller(view_inner.clone())
        .into();
        NestedScrollView::new(header, body)
            .controller(view_outer.clone())
            .into()
    });
    h.frames(1);
    drag(h, Offset::new(400., 500.), Offset::new(400., 300.), 4, 1);
    h.frames(40);
}

fn single_child_keep_offset_page_storage(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    let bucket = PageStorageBucket::default();
    let read = controller.clone();
    h.value("attached", move || {
        f64::from(u8::from(read.attachment_id().is_some()))
    });
    let read = controller.clone();
    h.value("offset", move || {
        if read.attachment_id().is_some() {
            f64::from(read.offset())
        } else {
            0.0
        }
    });
    let view_controller = controller.clone();
    h.mount(move |phase| {
        if phase == 0 {
            let rows: Vec<Widget> = (0..40)
                .map(|index| {
                    SizedBox::new()
                        .width(800.)
                        .height(50.)
                        .child(colored(if index % 2 == 0 { BLUE } else { GREEN }))
                        .into()
                })
                .collect();
            PageStorage::with_bucket(
                bucket.clone(),
                SingleChildScrollView::new(Column::new(rows))
                    .controller(view_controller.clone())
                    .page_storage_key(PageStorageKey::new("scroll")),
            )
            .into()
        } else {
            SizedBox::shrink().into()
        }
    });
    h.frames(1);
    controller.jump_to(300.);
    h.frames(2);
    h.set_phase(1);
    h.frames(2);
    h.set_phase(0);
    h.frames(3);
}
