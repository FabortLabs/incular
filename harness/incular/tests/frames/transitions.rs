use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use incular_animation::{AnimationController, DrivenAnimation, LinearTween};
use incular_config::{Alignment, MainAxisSize};
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::OpacityController;
use incular_widgets::{
    AlignTransition, AnimatedBuilder, BoxDecoration, Center, ColoredBox, Column, CustomScrollView,
    DecoratedBoxTransition, DefaultTextStyleTransition, FadeTransition, ListenableBuilder,
    MatrixTransition, PositionedTransition, RelativePositionedTransition, RelativeRect,
    RotationTransition, ScaleTransition, SizeTransition, SizedBox, SlideTransition, Sliver,
    SliverFadeTransition, SliverToBoxAdapter, Stack, Text, TextStyle, Transform, TweenAnimation,
    Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "transitions__compositor__fade_slide_scale_rotation",
            compositor_transitions,
        ),
        ("transitions__layout__align", align_transition),
        ("transitions__layout__size_clip", size_transition),
        (
            "transitions__positioned__relative_rect",
            positioned_relative_rect,
        ),
        ("transitions__builders__cached_child", builders_cached_child),
        ("transitions__paint__decoration_matrix", decoration_matrix),
        ("transitions__text__default_style", default_text_style),
        ("transitions__sliver_fade__opacity", sliver_fade_transition),
    ]
}

fn controller() -> AnimationController {
    AnimationController::new(Duration::from_millis(200)).with_value(0.0)
}

fn colored(color: Color, width: f32, height: f32) -> Widget {
    SizedBox::new()
        .width(width)
        .height(height)
        .child(ColoredBox::new(color, SizedBox::shrink()))
        .into()
}

fn compositor_transitions(h: &mut FrameHarness) {
    let controller = controller();
    h.value("value", {
        let controller = controller.clone();
        move || f64::from(controller.value())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let slide = DrivenAnimation::new(
                controller.clone(),
                LinearTween {
                    begin: Offset::ZERO,
                    end: Offset::new(0.4, -0.2),
                },
            );
            Center::new(FadeTransition::from_animation(
                controller.clone(),
                SlideTransition::from_animation(
                    slide,
                    ScaleTransition::from_animation(
                        controller.clone(),
                        RotationTransition::from_animation(
                            controller.clone(),
                            probe("box", colored(BLUE, 80.0, 40.0)),
                        ),
                    ),
                ),
            ))
            .into()
        }
    });
    h.frames(2);
    controller.set_value(0.6);
    h.frames(1);
    controller.set_value(1.0);
    h.frames(2);
}

fn align_transition(h: &mut FrameHarness) {
    let controller = controller().with_value(0.25);
    h.value("value", {
        let controller = controller.clone();
        move || f64::from(controller.value())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            Center::new(
                SizedBox::new()
                    .width(240.0)
                    .height(120.0)
                    .child(AlignTransition::new(
                        TweenAnimation::new(
                            controller.clone(),
                            Alignment::TOP_LEFT,
                            Alignment::BOTTOM_RIGHT,
                        ),
                        probe("box", colored(ORANGE, 100.0, 50.0)),
                    )),
            )
            .into()
        }
    });
    h.frames(2);
    controller.set_value(0.5);
    h.frames(1);
    controller.set_value(1.0);
    h.frames(2);
}

fn size_transition(h: &mut FrameHarness) {
    let controller = controller().with_value(1.0);
    h.value("value", {
        let controller = controller.clone();
        move || f64::from(controller.value())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            Center::new(
                SizedBox::new().width(240.0).height(120.0).child(probe(
                    "box",
                    SizeTransition::new(controller.clone(), colored(ORANGE, 100.0, 50.0))
                        .axis(incular_config::Axis::Horizontal)
                        .axis_alignment(-1.0)
                        .fixed_cross_axis_size_factor(1.0),
                )),
            )
            .into()
        }
    });
    h.frames(2);
    controller.set_value(0.5);
    h.frames(1);
    controller.set_value(1.0);
    h.frames(2);
}

fn positioned_relative_rect(h: &mut FrameHarness) {
    let controller = controller();
    h.value("value", {
        let controller = controller.clone();
        move || f64::from(controller.value())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            Center::new(
                SizedBox::new()
                    .width(300.0)
                    .height(180.0)
                    .child(Stack::new([
                        Widget::from(PositionedTransition::new(
                            TweenAnimation::new(
                                controller.clone(),
                                RelativeRect::from_ltrb(0.0, 0.0, 200.0, 130.0),
                                RelativeRect::from_ltrb(120.0, 60.0, 20.0, 10.0),
                            ),
                            probe("positioned", ColoredBox::new(GREEN, SizedBox::shrink())),
                        )),
                        Widget::from(RelativePositionedTransition::new(
                            TweenAnimation::new(
                                controller.clone(),
                                Rect::from_origin_size(
                                    Offset::new(10.0, 20.0),
                                    Size::new(60.0, 50.0),
                                ),
                                Rect::from_origin_size(
                                    Offset::new(180.0, 100.0),
                                    Size::new(100.0, 70.0),
                                ),
                            ),
                            Size::new(300.0, 180.0),
                            probe("relative", ColoredBox::new(BLUE, SizedBox::shrink())),
                        )),
                    ])),
            )
            .into()
        }
    });
    h.frames(2);
    controller.set_value(0.5);
    h.frames(1);
    controller.set_value(1.0);
    h.frames(2);
}

fn builders_cached_child(h: &mut FrameHarness) {
    let controller = controller();
    let animated_builds = Rc::new(Cell::new(0.0));
    let listenable_builds = Rc::new(Cell::new(0.0));
    h.value("value", {
        let controller = controller.clone();
        move || f64::from(controller.value())
    });
    h.value("animated_builds", {
        let count = Rc::clone(&animated_builds);
        move || count.get()
    });
    h.value("listenable_builds", {
        let count = Rc::clone(&listenable_builds);
        move || count.get()
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let animated_count = Rc::clone(&animated_builds);
            let listenable_count = Rc::clone(&listenable_builds);
            let animated_controller = controller.clone();
            let listenable_controller = controller.clone();
            Center::new(
                Column::new([
                    Widget::from(
                        AnimatedBuilder::new(animated_controller.clone(), move |_, child| {
                            animated_count.set(animated_count.get() + 1.0);
                            Transform::translation(
                                Offset::new(animated_controller.value() * 40.0, 0.0),
                                child.expect("animated child"),
                            )
                            .into()
                        })
                        .child(probe("animated_child", colored(BLUE, 30.0, 20.0))),
                    ),
                    Widget::from(
                        ListenableBuilder::new(listenable_controller.clone(), move |_, child| {
                            listenable_count.set(listenable_count.get() + 1.0);
                            Transform::translation(
                                Offset::new(0.0, listenable_controller.value() * 30.0),
                                child.expect("listenable child"),
                            )
                            .into()
                        })
                        .child(probe("listenable_child", colored(ORANGE, 30.0, 20.0))),
                    ),
                ])
                .main_axis_size(MainAxisSize::Min),
            )
            .into()
        }
    });
    h.frames(1);
    controller.set_value(0.5);
    h.frames(1);
    controller.set_value(1.0);
    h.frames(1);
}

fn decoration_matrix(h: &mut FrameHarness) {
    let controller = controller();
    h.value("value", {
        let controller = controller.clone();
        move || f64::from(controller.value())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            Center::new(
                Column::new([
                    probe(
                        "decoration",
                        DecoratedBoxTransition::new(
                            TweenAnimation::new(
                                controller.clone(),
                                BoxDecoration::new().color(BLUE),
                                BoxDecoration::new().color(ORANGE),
                            ),
                            SizedBox::new().width(80.0).height(40.0),
                        ),
                    ),
                    Widget::from(MatrixTransition::new(
                        controller.clone(),
                        |value| CoreTransform::translation(Offset::new(value * 60.0, 0.0)),
                        probe("matrix", colored(GREEN, 50.0, 30.0)),
                    )),
                ])
                .main_axis_size(MainAxisSize::Min),
            )
            .into()
        }
    });
    h.frames(2);
    controller.set_value(0.5);
    h.frames(1);
    controller.set_value(1.0);
    h.frames(2);
}

fn default_text_style(h: &mut FrameHarness) {
    let controller = controller();
    h.value("value", {
        let controller = controller.clone();
        move || f64::from(controller.value())
    });
    let begin = TextStyle::default()
        .font_family("Inter")
        .font_size(14.0)
        .color(Color::BLACK);
    let end = TextStyle::default()
        .font_family("Inter")
        .font_size(22.0)
        .color(BLUE);
    h.mount({
        let controller = controller.clone();
        move |_| {
            Center::new(Widget::from(DefaultTextStyleTransition::new(
                TweenAnimation::new(controller.clone(), begin.clone(), end.clone()),
                probe("text", Text::new("Explicit transition text")),
            )))
            .into()
        }
    });
    h.frames(2);
    controller.set_value(0.5);
    h.frames(1);
    controller.set_value(1.0);
    h.frames(2);
}

fn sliver_fade_transition(h: &mut FrameHarness) {
    let opacity = OpacityController::new();
    opacity.set_opacity(0.25);
    h.value("opacity", {
        let opacity = opacity.clone();
        move || f64::from(opacity.opacity())
    });
    h.mount({
        let opacity = opacity.clone();
        move |_| {
            let sliver: Box<dyn Sliver> = Box::new(SliverFadeTransition::new(
                opacity.clone(),
                SliverToBoxAdapter::new(probe("sliver_child", colored(GREEN, 800.0, 80.0))),
            ));
            CustomScrollView::new([sliver]).into()
        }
    });
    h.frames(1);
    opacity.set_opacity(0.75);
    h.frames(1);
}
