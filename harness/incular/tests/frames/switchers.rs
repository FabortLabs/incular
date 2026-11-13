use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_core::{Color, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    Alignment, AnimatedCrossFade, AnimatedGrid, AnimatedGridController, AnimatedList,
    AnimatedListController, AnimatedModalBarrier, AnimatedSize, AnimatedSwitcher, Animation,
    Center, Clip, CrossFadeState, Curve, CustomScrollView, SizeTransition, SizedBox, Sliver,
    SliverAnimatedGrid, SliverAnimatedList, SliverAnimatedListController, SliverGridDelegate,
    Stack, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);
const DURATION: Duration = Duration::from_millis(300);

fn colored(color: Color) -> Widget {
    incular_widgets::ColoredBox::new(color, SizedBox::shrink()).into()
}

fn box_(color: Color, width: f32, height: f32) -> Widget {
    SizedBox::new()
        .width(width)
        .height(height)
        .child(colored(color))
        .into()
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "switchers__switcher_overlapping_children",
            switcher_overlapping_children,
        ),
        (
            "switchers__cross_fade_reversed_different_sizes",
            cross_fade_reversed_different_sizes,
        ),
        ("switchers__animated_size_retarget", animated_size_retarget),
        (
            "switchers__animated_list_insert_size_transition",
            animated_list_insert_size_transition,
        ),
        (
            "switchers__animated_grid_insert_size_transition",
            animated_grid_insert_size_transition,
        ),
        (
            "switchers__sliver_animated_list_initial_layout",
            sliver_animated_list_initial_layout,
        ),
        (
            "switchers__sliver_animated_grid_insert_size_transition",
            sliver_animated_grid_insert_size_transition,
        ),
        (
            "switchers__animated_modal_barrier_color_and_dismiss",
            animated_modal_barrier_color_and_dismiss,
        ),
        (
            "switchers__animated_list_remove_shifts_items",
            animated_list_remove_shifts_items,
        ),
        (
            "switchers__switcher_same_key_no_transition",
            switcher_same_key_no_transition,
        ),
        (
            "switchers__switcher_reverse_duration",
            switcher_reverse_duration,
        ),
        (
            "switchers__switcher_layout_builder_top_left",
            switcher_layout_builder_top_left,
        ),
        (
            "switchers__animated_size_top_left_alignment",
            animated_size_top_left_alignment,
        ),
    ]
}

fn switcher_overlapping_children(h: &mut FrameHarness) {
    h.mount(|phase| {
        let child = match phase {
            0 => probe("switch_a", box_(BLUE, 80., 48.)),
            1 => probe("switch_b", box_(ORANGE, 140., 64.)),
            _ => probe("switch_c", box_(GREEN, 100., 80.)),
        };
        Center::new(AnimatedSwitcher::new(DURATION, Some(child))).into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(5);
    h.set_phase(2);
    h.frames(22);
}

fn cross_fade_reversed_different_sizes(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedCrossFade::new(
                probe("cross_first", box_(BLUE, 90., 40.)),
                probe("cross_second", box_(ORANGE, 150., 76.)),
                if phase == 0 || phase == 2 {
                    CrossFadeState::ShowFirst
                } else {
                    CrossFadeState::ShowSecond
                },
                DURATION,
            )
            .first_curve(Curve::Linear)
            .second_curve(Curve::Linear)
            .size_curve(Curve::Linear)
            .alignment(Alignment::TOP_CENTER),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(8);
    h.set_phase(2);
    h.frames(22);
}

fn animated_size_retarget(h: &mut FrameHarness) {
    h.mount(|phase| {
        let (width, height) = match phase {
            0 => (80., 40.),
            1 => (180., 100.),
            _ => (120., 64.),
        };
        Center::new(probe(
            "animated_size",
            AnimatedSize::new(DURATION, box_(GREEN, width, height))
                .curve(Curve::Linear)
                .alignment(Alignment::CENTER)
                .clip_behavior(Clip::HardEdge),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(8);
    h.set_phase(2);
    h.frames(22);
}

type Items = Rc<RefCell<Vec<(&'static str, Color)>>>;

fn animated_list_insert_size_transition(h: &mut FrameHarness) {
    let controller = AnimatedListController::with_duration(2, DURATION);
    let items: Items = Rc::new(RefCell::new(vec![("list_a", BLUE), ("list_b", ORANGE)]));
    let inserted_id = Rc::new(Cell::new(None::<u64>));
    let did_insert = Rc::new(Cell::new(false));
    {
        let controller = controller.clone();
        h.value("item_count", move || controller.item_count() as f64);
    }
    {
        let controller = controller.clone();
        let inserted_id = inserted_id.clone();
        h.value("incoming_value", move || {
            inserted_id
                .get()
                .and_then(|id| controller.animation_for_id(id))
                .map_or(0., |animation| f64::from(animation.value()))
        });
    }
    h.mount({
        let controller = controller.clone();
        let items = items.clone();
        let did_insert = did_insert.clone();
        let inserted_id = inserted_id.clone();
        move |phase| {
            if phase == 1 && !did_insert.replace(true) {
                items.borrow_mut().insert(1, ("list_new", GREEN));
                let id = controller.insert_item(1);
                inserted_id.set(Some(id));
                if let Some(animation) = controller.animation_for_id(id) {
                    animation.set_value(0.);
                    animation.forward_on_next_tick();
                }
            }
            let builder_items = items.clone();
            let builder_controller = controller.clone();
            AnimatedList::animated(2, move |index, item| {
                let (name, color) = builder_items.borrow()[index];
                let animation = builder_controller
                    .animation_for_id(item.id)
                    .expect("animated item controller");
                probe(
                    name,
                    SizeTransition::new(
                        animation,
                        SizedBox::new().height(48.).child(colored(color)),
                    ),
                )
            })
            .controller(controller.clone())
            .into()
        }
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(22);
}

fn animated_grid_insert_size_transition(h: &mut FrameHarness) {
    let controller = AnimatedGridController::with_duration(2, DURATION);
    let items: Items = Rc::new(RefCell::new(vec![("grid_a", BLUE), ("grid_b", ORANGE)]));
    let inserted_id = Rc::new(Cell::new(None::<u64>));
    let did_insert = Rc::new(Cell::new(false));
    {
        let controller = controller.clone();
        h.value("item_count", move || controller.item_count() as f64);
    }
    {
        let controller = controller.clone();
        let inserted_id = inserted_id.clone();
        h.value("incoming_value", move || {
            inserted_id
                .get()
                .and_then(|id| controller.animation_for_id(id))
                .map_or(0., |animation| f64::from(animation.value()))
        });
    }
    h.mount({
        let controller = controller.clone();
        let items = items.clone();
        let did_insert = did_insert.clone();
        let inserted_id = inserted_id.clone();
        move |phase| {
            if phase == 1 && !did_insert.replace(true) {
                items.borrow_mut().insert(1, ("grid_new", GREEN));
                let id = controller.insert_item(1);
                inserted_id.set(Some(id));
                if let Some(animation) = controller.animation_for_id(id) {
                    animation.set_value(0.);
                    animation.forward_on_next_tick();
                }
            }
            let builder_items = items.clone();
            let builder_controller = controller.clone();
            AnimatedGrid::animated(
                2,
                SliverGridDelegate::fixed_cross_axis_count(2).main_axis_extent(60.),
                move |index, item| {
                    let (name, color) = builder_items.borrow()[index];
                    let animation = builder_controller
                        .animation_for_id(item.id)
                        .expect("animated grid item controller");
                    probe(
                        name,
                        SizeTransition::new(
                            animation,
                            SizedBox::new().height(60.).child(colored(color)),
                        ),
                    )
                },
            )
            .controller(controller.clone())
            .into()
        }
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(22);
}

fn sliver_animated_list_initial_layout(h: &mut FrameHarness) {
    let controller = SliverAnimatedListController::with_duration(2, DURATION);
    h.mount({
        let controller = controller.clone();
        move |_| {
            let list = SliverAnimatedList::new(2, |index| {
                let (name, color) = if index == 0 {
                    ("sliver_list_a", BLUE)
                } else {
                    ("sliver_list_b", ORANGE)
                };
                probe(name, SizedBox::new().height(48.).child(colored(color)))
            })
            .controller(controller.clone());
            CustomScrollView::new(vec![Box::new(list) as Box<dyn Sliver>]).into()
        }
    });
    h.frames(6);
}

fn sliver_animated_grid_insert_size_transition(h: &mut FrameHarness) {
    let controller = incular_widgets::SliverAnimatedGridController::with_duration(2, DURATION);
    let items: Items = Rc::new(RefCell::new(vec![
        ("sliver_grid_a", BLUE),
        ("sliver_grid_b", ORANGE),
    ]));
    let inserted_id = Rc::new(Cell::new(None::<u64>));
    let did_insert = Rc::new(Cell::new(false));
    {
        let controller = controller.clone();
        h.value("item_count", move || controller.item_count() as f64);
    }
    {
        let controller = controller.clone();
        let inserted_id = inserted_id.clone();
        h.value("incoming_value", move || {
            inserted_id
                .get()
                .and_then(|id| controller.animation_for_id(id))
                .map_or(0., |animation| f64::from(animation.value()))
        });
    }
    h.mount({
        let controller = controller.clone();
        let items = items.clone();
        let did_insert = did_insert.clone();
        let inserted_id = inserted_id.clone();
        move |phase| {
            if phase == 1 && !did_insert.replace(true) {
                items.borrow_mut().insert(1, ("sliver_grid_new", GREEN));
                let id = controller.insert_item(1);
                inserted_id.set(Some(id));
                if let Some(animation) = controller.animation_for_id(id) {
                    animation.set_value(0.);
                    animation.forward_on_next_tick();
                }
            }
            let builder_items = items.clone();
            let builder_controller = controller.clone();
            let grid = SliverAnimatedGrid::animated(
                2,
                SliverGridDelegate::fixed_cross_axis_count(2).main_axis_extent(60.),
                move |index, item| {
                    let (name, color) = builder_items.borrow()[index];
                    let animation = builder_controller
                        .animation_for_id(item.id)
                        .expect("animated sliver grid item controller");
                    probe(
                        name,
                        SizeTransition::new(
                            animation,
                            SizedBox::new().height(60.).child(colored(color)),
                        ),
                    )
                },
            )
            .controller(controller.clone());
            CustomScrollView::new(vec![Box::new(grid) as Box<dyn Sliver>]).into()
        }
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(22);
}

fn animated_modal_barrier_color_and_dismiss(h: &mut FrameHarness) {
    let color = Animation::from_to(
        Color::rgba(0, 0, 0, 128),
        Color::rgba(0x4C, 0x77, 0xA0, 128),
        DURATION,
        Curve::Linear,
    );
    let controller = color.controller();
    let dismissals = Rc::new(Cell::new(0_u32));
    {
        let dismissals = dismissals.clone();
        h.value("dismissals", move || f64::from(dismissals.get()));
    }
    h.mount({
        let dismissals = dismissals.clone();
        move |_| {
            let dismissals = dismissals.clone();
            probe(
                "barrier",
                AnimatedModalBarrier::with_color(color.clone())
                    .dismissible(true)
                    .semantics_label("Dismiss")
                    .on_dismiss(move || dismissals.set(dismissals.get() + 1)),
            )
        }
    });
    h.frames(2);
    controller.forward_on_next_tick();
    h.frames(8);
    h.down(Offset::new(400., 300.));
    h.up();
    h.frames(1);
    h.frames(15);
}

fn animated_list_remove_shifts_items(h: &mut FrameHarness) {
    let controller = AnimatedListController::with_duration(3, DURATION);
    let items: Items = Rc::new(RefCell::new(vec![
        ("remove_a", BLUE),
        ("remove_b", ORANGE),
        ("remove_c", GREEN),
    ]));
    let outgoing: Rc<RefCell<Option<incular_animation::AnimationController>>> =
        Rc::new(RefCell::new(None));
    let did_remove = Rc::new(Cell::new(false));
    {
        let controller = controller.clone();
        h.value("item_count", move || controller.item_count() as f64);
    }
    {
        let outgoing = outgoing.clone();
        h.value("outgoing_value", move || {
            outgoing
                .borrow()
                .as_ref()
                .map_or(1., |animation| f64::from(animation.value()))
        });
    }
    h.mount({
        let controller = controller.clone();
        let items = items.clone();
        let did_remove = did_remove.clone();
        let outgoing = outgoing.clone();
        move |phase| {
            if phase == 1 && !did_remove.replace(true) {
                items.borrow_mut().remove(0);
                let builder_controller = controller.clone();
                let id = controller
                    .remove_at_with_builder(0, incular_core::time::Instant::now(), move |item| {
                        let animation = builder_controller
                            .animation_for_id(item.id)
                            .expect("removed item controller");
                        probe(
                            "removed_a",
                            SizeTransition::new(
                                animation,
                                SizedBox::new().height(48.).child(colored(BLUE)),
                            ),
                        )
                    })
                    .expect("live item at index 0");
                if let Some(animation) = controller.animation_for_id(id) {
                    animation.set_value(1.);
                    animation.reverse_on_next_tick();
                    *outgoing.borrow_mut() = Some(animation);
                }
            }
            let builder_items = items.clone();
            let builder_controller = controller.clone();
            AnimatedList::animated(3, move |index, item| {
                let (name, color) = builder_items.borrow()[index];
                let animation = builder_controller
                    .animation_for_id(item.id)
                    .expect("animated item controller");
                probe(
                    name,
                    SizeTransition::new(
                        animation,
                        SizedBox::new().height(48.).child(colored(color)),
                    ),
                )
            })
            .controller(controller.clone())
            .into()
        }
    });
    h.frames(1);
    h.set_phase(1);
    h.frames(22);
}

fn switcher_same_key_no_transition(h: &mut FrameHarness) {
    h.mount(|phase| {
        let color = if phase == 0 { BLUE } else { ORANGE };
        Center::new(AnimatedSwitcher::new(
            DURATION,
            Some(probe("same_key", box_(color, 80., 48.))),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(8);
}

fn switcher_reverse_duration(h: &mut FrameHarness) {
    h.mount(|phase| {
        let child = if phase == 0 {
            probe("switch_a", box_(BLUE, 80., 48.))
        } else {
            probe("switch_b", box_(ORANGE, 140., 64.))
        };
        Center::new(
            AnimatedSwitcher::new(Duration::from_millis(150), Some(child))
                .reverse_duration(Duration::from_millis(450)),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(30);
}

fn switcher_layout_builder_top_left(h: &mut FrameHarness) {
    h.mount(|phase| {
        let child = if phase == 0 {
            probe("switch_a", box_(BLUE, 80., 48.))
        } else {
            probe("switch_b", box_(ORANGE, 140., 64.))
        };
        Center::new(AnimatedSwitcher::new(DURATION, Some(child)).layout_builder(
            |current, previous| {
                Stack::new(previous.into_iter().chain(current))
                    .alignment(Alignment::TOP_LEFT)
                    .into()
            },
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(22);
}

fn animated_size_top_left_alignment(h: &mut FrameHarness) {
    h.mount(|phase| {
        let (width, height) = if phase == 0 { (80., 40.) } else { (180., 100.) };
        Center::new(probe(
            "animated_size",
            AnimatedSize::new(DURATION, probe("size_child", box_(GREEN, width, height)))
                .curve(Curve::Linear)
                .alignment(Alignment::TOP_LEFT)
                .clip_behavior(Clip::HardEdge),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(22);
}
