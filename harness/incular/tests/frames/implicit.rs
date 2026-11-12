//! Frame scenarios for Flutter's implicitly animated widget family.

use std::{cell::Cell, rc::Rc, time::Duration};

use incular_config::{Alignment, EdgeInsets, TextDirection};
use incular_core::{Color, Offset, Transform};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_text::{FontFamily, TextStyle};
use incular_widgets::{
    AnimatedAlign, AnimatedContainer, AnimatedDefaultTextStyle, AnimatedFractionallySizedBox,
    AnimatedOpacity, AnimatedPadding, AnimatedPhysicalModel, AnimatedPositioned,
    AnimatedPositionedDirectional, AnimatedRotation, AnimatedScale, AnimatedSlide, Border,
    BorderRadius, Center, ColoredBox, Curves, CustomScrollView, Directionality, SizedBox, Sliver,
    SliverAnimatedOpacity, SliverToBoxAdapter, Stack, Text, TweenAnimationBuilder, Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        ("implicit__container__fields", container_fields),
        ("implicit__padding__edge_insets", padding_edge_insets),
        (
            "implicit__align__alignment_and_factors",
            align_alignment_and_factors,
        ),
        (
            "implicit__positioned__edges_and_size",
            positioned_edges_and_size,
        ),
        (
            "implicit__positioned_directional__direction",
            positioned_directional_direction,
        ),
        ("implicit__scale__alignment", scale_alignment),
        (
            "implicit__rotation__turns_and_alignment",
            rotation_turns_and_alignment,
        ),
        (
            "implicit__slide__fractional_offset",
            slide_fractional_offset,
        ),
        ("implicit__opacity__opacity", opacity_opacity),
        ("implicit__sliver_opacity__opacity", sliver_opacity),
        (
            "implicit__default_text_style__text_style",
            default_text_style,
        ),
        (
            "implicit__physical_model__color_elevation_radius",
            physical_model,
        ),
        (
            "implicit__fractional_sized_box__factors",
            fractional_sized_box,
        ),
        (
            "implicit__tween_animation_builder__initial_and_retarget",
            tween_animation_builder,
        ),
        (
            "implicit__retarget__current_value_and_on_end",
            retarget_current_value_and_on_end,
        ),
        ("implicit__zero_duration__completion", zero_duration),
        (
            "implicit__disabled_animations__reduced_motion",
            disabled_animations,
        ),
    ]
}

fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn container_fields(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(probe(
            "box",
            AnimatedContainer::new(Duration::from_millis(320))
                .curve(Curves::EASE_IN_OUT)
                .width(if phase == 0 { 100. } else { 180. })
                .height(if phase == 0 { 70. } else { 110. })
                .margin(EdgeInsets::all(if phase == 0 { 8. } else { 18. }))
                .padding(EdgeInsets::all(if phase == 0 { 4. } else { 12. }))
                .alignment(if phase == 0 {
                    Alignment::TOP_LEFT
                } else {
                    Alignment::BOTTOM_RIGHT
                })
                .transform(Transform::translation(Offset::new(
                    if phase == 0 { 3. } else { 23. },
                    0.,
                )))
                .transform_alignment(if phase == 0 {
                    Alignment::TOP_LEFT
                } else {
                    Alignment::CENTER
                })
                .decoration(
                    incular_widgets::BoxDecoration::new()
                        .color(if phase == 0 { BLUE } else { ORANGE })
                        .border_radius(BorderRadius::circular(if phase == 0 { 2. } else { 18. }))
                        .border(Border::new(
                            if phase == 0 { 1. } else { 5. },
                            if phase == 0 { GREEN } else { BLUE },
                        )),
                )
                .child(colored(GREEN)),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn padding_edge_insets(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedPadding::new(
                EdgeInsets::from_ltrb(
                    if phase == 0 { 4. } else { 28. },
                    if phase == 0 { 8. } else { 20. },
                    if phase == 0 { 16. } else { 2. },
                    if phase == 0 { 3. } else { 23. },
                ),
                Duration::from_millis(320),
                probe(
                    "box",
                    SizedBox::new().width(80.).height(40.).child(colored(BLUE)),
                ),
            )
            .curve(Curves::EASE_IN_OUT),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn align_alignment_and_factors(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedAlign::new(
                if phase == 0 {
                    Alignment::TOP_LEFT
                } else {
                    Alignment::BOTTOM_RIGHT
                },
                Duration::from_millis(320),
                probe(
                    "box",
                    SizedBox::new()
                        .width(64.)
                        .height(32.)
                        .child(colored(ORANGE)),
                ),
            )
            .width_factor(if phase == 0 { 1. } else { 2. })
            .height_factor(if phase == 0 { 1. } else { 1.5 })
            .curve(Curves::EASE_IN_OUT),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn positioned_edges_and_size(h: &mut FrameHarness) {
    h.mount(|phase| {
        Stack::new([Widget::from(
            AnimatedPositioned::new(Duration::from_millis(320), probe("box", colored(GREEN)))
                .curve(Curves::EASE_IN_OUT)
                .left(if phase == 0 { 10. } else { 310. })
                .top(if phase == 0 { 20. } else { 140. })
                .width(if phase == 0 { 70. } else { 120. })
                .height(if phase == 0 { 50. } else { 90. }),
        )])
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn positioned_directional_direction(h: &mut FrameHarness) {
    h.mount(|phase| {
        Directionality::new(
            if phase == 0 {
                TextDirection::Ltr
            } else {
                TextDirection::Rtl
            },
            Stack::new([Widget::from(
                AnimatedPositionedDirectional::new(
                    Duration::from_millis(320),
                    probe("box", colored(BLUE)),
                )
                .curve(Curves::EASE_IN_OUT)
                .start(if phase == 0 { 12. } else { 44. })
                .top(24.)
                .width(80.)
                .height(40.),
            )]),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn scale_alignment(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedScale::new(
                if phase == 0 { 1. } else { 1.7 },
                Duration::from_millis(320),
                probe(
                    "box",
                    SizedBox::new()
                        .width(100.)
                        .height(60.)
                        .child(colored(GREEN)),
                ),
            )
            .alignment(Alignment::BOTTOM_LEFT)
            .curve(Curves::EASE_IN_OUT),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn rotation_turns_and_alignment(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedRotation::new(
                if phase == 0 { 0. } else { 0.375 },
                Duration::from_millis(320),
                probe(
                    "box",
                    SizedBox::new()
                        .width(100.)
                        .height(60.)
                        .child(colored(ORANGE)),
                ),
            )
            .alignment(Alignment::TOP_RIGHT)
            .curve(Curves::EASE_IN_OUT),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn slide_fractional_offset(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedSlide::new(
                if phase == 0 {
                    Offset::ZERO
                } else {
                    Offset::new(0.75, -0.5)
                },
                Duration::from_millis(320),
                probe(
                    "box",
                    SizedBox::new().width(80.).height(40.).child(colored(BLUE)),
                ),
            )
            .curve(Curves::EASE_IN_OUT),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn opacity_opacity(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedOpacity::new(
                if phase == 0 { 1. } else { 0.25 },
                Duration::from_millis(320),
                probe(
                    "box",
                    SizedBox::new().width(80.).height(40.).child(colored(BLUE)),
                ),
            )
            .curve(Curves::EASE_IN_OUT),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn sliver_opacity(h: &mut FrameHarness) {
    h.mount(|phase| {
        CustomScrollView::new([Box::new(
            SliverAnimatedOpacity::new(
                if phase == 0 { 1. } else { 0.25 },
                Duration::from_millis(320),
                SliverToBoxAdapter::new(probe(
                    "box",
                    SizedBox::new().width(80.).height(40.).child(colored(GREEN)),
                )),
            )
            .curve(Curves::EASE_IN_OUT),
        ) as Box<dyn Sliver>])
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn default_text_style(h: &mut FrameHarness) {
    h.mount(|phase| {
        let style = TextStyle::new()
            .family(FontFamily::named("Inter"))
            .size(if phase == 0 { 14. } else { 30. })
            .color(if phase == 0 { Color::BLACK } else { BLUE });
        Center::new(
            AnimatedDefaultTextStyle::new(
                style,
                Duration::from_millis(320),
                probe("text", Text::new("Inter 123")),
            )
            .curve(Curves::EASE_IN_OUT),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn physical_model(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(probe(
            "model",
            AnimatedPhysicalModel::new(
                if phase == 0 { BLUE } else { ORANGE },
                if phase == 0 { GREEN } else { BLUE },
                if phase == 0 { 1. } else { 18. },
                Duration::from_millis(320),
                SizedBox::new().width(100.).height(60.),
            )
            .border_radius(BorderRadius::circular(if phase == 0 { 2. } else { 24. }))
            .clip_behavior(incular_config::Clip::AntiAlias)
            .curve(Curves::EASE_IN_OUT),
        ))
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn fractional_sized_box(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            SizedBox::new().width(240.).height(180.).child(
                AnimatedFractionallySizedBox::new(
                    Duration::from_millis(320),
                    probe("box", colored(ORANGE)),
                )
                .alignment(if phase == 0 {
                    Alignment::TOP_LEFT
                } else {
                    Alignment::BOTTOM_RIGHT
                })
                .width_factor(if phase == 0 { 0.25 } else { 0.75 })
                .height_factor(if phase == 0 { 0.4 } else { 0.8 })
                .curve(Curves::EASE_IN_OUT),
            ),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(24);
}

fn tween_animation_builder(h: &mut FrameHarness) {
    let sampled = Rc::new(Cell::new(0.0_f64));
    let on_end = Rc::new(Cell::new(0_u32));
    let sample_out = Rc::clone(&sampled);
    h.value("tween", move || sample_out.get());
    let ended_out = Rc::clone(&on_end);
    h.value("on_end", move || f64::from(ended_out.get()));
    h.mount({
        let sampled = Rc::clone(&sampled);
        let on_end = Rc::clone(&on_end);
        move |phase| {
            let ended = Rc::clone(&on_end);
            let sample = Rc::clone(&sampled);
            TweenAnimationBuilder::new(
                if phase == 0 { 100. } else { 240. },
                Duration::from_millis(320),
                move |_, value, child| {
                    sample.set(value);
                    SizedBox::new()
                        .width(80.)
                        .height(40.)
                        .child(child.unwrap_or_else(|| colored(BLUE)))
                },
            )
            .begin(if phase == 0 { 0. } else { 500. })
            .curve(Curves::EASE_IN_OUT)
            .on_end(move || ended.set(ended.get() + 1))
            .child(probe("box", colored(BLUE)))
            .into()
        }
    });
    h.frames(8);
    h.set_phase(1);
    h.frames(24);
}

fn retarget_current_value_and_on_end(h: &mut FrameHarness) {
    let ended = Rc::new(Cell::new(0_u32));
    let ended_out = Rc::clone(&ended);
    h.value("on_end", move || f64::from(ended_out.get()));
    h.mount({
        let ended = Rc::clone(&ended);
        move |phase| {
            let ended = Rc::clone(&ended);
            Center::new(
                AnimatedOpacity::new(
                    match phase {
                        0 => 1.,
                        1 => 0.,
                        _ => 0.55,
                    },
                    Duration::from_millis(320),
                    probe(
                        "box",
                        SizedBox::new().width(80.).height(40.).child(colored(BLUE)),
                    ),
                )
                .curve(Curves::EASE_IN_OUT)
                .on_end(move || ended.set(ended.get() + 1)),
            )
            .into()
        }
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(7);
    h.set_phase(2);
    h.frames(24);
}

fn zero_duration(h: &mut FrameHarness) {
    let ended = Rc::new(Cell::new(0_u32));
    let ended_out = Rc::clone(&ended);
    h.value("on_end", move || f64::from(ended_out.get()));
    h.mount({
        let ended = Rc::clone(&ended);
        move |phase| {
            let ended = Rc::clone(&ended);
            Center::new(
                AnimatedOpacity::new(
                    if phase == 0 { 1. } else { 0.3 },
                    Duration::ZERO,
                    probe(
                        "box",
                        SizedBox::new().width(80.).height(40.).child(colored(BLUE)),
                    ),
                )
                .on_end(move || ended.set(ended.get() + 1)),
            )
            .into()
        }
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(4);
}

fn disabled_animations(h: &mut FrameHarness) {
    h.mount(|phase| {
        Center::new(
            AnimatedOpacity::new(
                if phase == 0 { 1. } else { 0.3 },
                Duration::from_millis(400),
                probe(
                    "box",
                    SizedBox::new().width(80.).height(40.).child(colored(BLUE)),
                ),
            )
            .curve(Curves::LINEAR),
        )
        .into()
    });
    h.frames(2);
    let mut environment = h.runtime().environment();
    environment.reduced_motion = true;
    h.runtime().set_environment(environment);
    h.set_phase(1);
    h.frames(8);
}
