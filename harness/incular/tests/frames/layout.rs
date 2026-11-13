use std::cell::Cell;
use std::rc::Rc;

use incular_config::{
    Alignment, Axis, Constraints, CrossAxisAlignment, EdgeInsets, MainAxisAlignment, MainAxisSize,
    StackFit, TextDirection, VerticalDirection, WrapAlignment, WrapCrossAlignment,
};
use incular_core::{Color, Offset, Size, Transform as CoreTransform};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    AspectRatio, BoxFit, Center, ColoredBox, Column, ConstrainedBox, Container,
    CustomMultiChildLayout, CustomSingleChildLayout, Expanded, FittedBox, Flexible, Flow,
    FlowDelegate, FlowPaintingContext, FractionallySizedBox, IndexedStack, IntrinsicHeight,
    IntrinsicWidth, LayoutBuilder, LayoutId, LimitedBox, MultiChildLayoutContext,
    MultiChildLayoutDelegate, Offstage, OverflowBox, Padding, Positioned, Row,
    SingleChildLayoutDelegate, SizedBox, SizedOverflowBox, Spacer, Stack, Table, Text, Transform,
    UnconstrainedBox, Visibility, Widget, Wrap,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "layout__fractionally_sized_box__null_factor_tight_parent",
            fractionally_sized_box_null_factor_tight_parent,
        ),
        ("layout__row__main_and_cross_alignment", row_alignment),
        ("layout__flex__expanded_and_spacing", flex_expanded),
        ("layout__stack__positioned", stack_positioned),
        ("layout__indexed_stack__active_child", indexed_stack_active),
        ("layout__wrap__multiple_runs", wrap_runs),
        (
            "layout__aspect_constraints_and_fraction",
            aspect_constraints_fraction,
        ),
        ("layout__fitted_box__contain_alignment", fitted_box),
        ("layout__padding_and_overflow_boxes", padding_overflow),
        ("layout__table__rows_and_columns", table_layout),
        ("layout__baseline__cross_axis", baseline_layout),
        ("layout__custom_single_child", custom_single),
        ("layout__custom_multi_child", custom_multi),
        ("layout__flow__child_transforms", flow_layout),
        ("layout__layout_builder__constraint_size", layout_builder),
        ("layout__intrinsic_width__step", intrinsic_width),
        (
            "layout__box_constraints__limited_overflow_intrinsic",
            box_constraints_limited_overflow_intrinsic,
        ),
        (
            "layout__visibility_offstage_and_transform",
            visibility_transform,
        ),
        (
            "layout__flex__main_alignment_variants",
            main_alignment_variants,
        ),
        ("layout__flex__direction_reversal", direction_reversal),
        ("layout__flex__fit_loose_and_tight", fit_loose_and_tight),
        (
            "layout__flex__spacing_and_fixed_gaps",
            spacing_and_fixed_gaps,
        ),
        (
            "layout__stack__fit_alignment_positioned",
            stack_fit_alignment_positioned,
        ),
        (
            "layout__stack__empty_and_positioned_only",
            stack_empty_positioned_only,
        ),
        (
            "layout__wrap__alignment_and_run_axes",
            wrap_alignment_run_axes,
        ),
        ("layout__fitted_box__all_fits", fitted_box_all_fits),
        ("layout__aspect_ratio__constraint_cases", aspect_ratio_cases),
        (
            "layout__container__composition_order",
            container_composition_order,
        ),
        (
            "layout__intrinsic_width__steps_and_flex_child",
            intrinsic_width_steps_flex,
        ),
        (
            "layout__baseline__without_baseline_child",
            baseline_without_baseline_child,
        ),
        (
            "layout__unconstrained_box__constrained_axis",
            unconstrained_constrained_axis,
        ),
        (
            "layout__layout_builder__rebuild_triggers",
            layout_builder_rebuild_triggers,
        ),
        (
            "layout__visibility__maintain_flags",
            visibility_maintain_flags,
        ),
        (
            "layout__box_constraints__helper_values",
            box_constraints_helpers,
        ),
        (
            "layout__table__default_flex_columns",
            table_default_flex_columns,
        ),
    ]
}

fn colored(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn row_alignment(h: &mut FrameHarness) {
    h.mount(|_| {
        Row::new([
            probe(
                "row0",
                SizedBox::new().width(50.).height(20.).child(colored(BLUE)),
            ),
            probe(
                "row1",
                SizedBox::new()
                    .width(80.)
                    .height(40.)
                    .child(colored(ORANGE)),
            ),
            probe(
                "row2",
                SizedBox::new().width(60.).height(30.).child(colored(GREEN)),
            ),
        ])
        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .cross_axis_alignment(CrossAxisAlignment::End)
        .into()
    });
    h.frames(1);
}

fn flex_expanded(h: &mut FrameHarness) {
    h.mount(|_| {
        Row::new([
            Widget::from(Expanded::new(probe("expanded0", colored(BLUE)))),
            SizedBox::new().width(12.).into(),
            Widget::from(Expanded::new(probe("expanded1", colored(ORANGE))).flex(2)),
        ])
        .into()
    });
    h.frames(1);
}

fn stack_positioned(h: &mut FrameHarness) {
    h.mount(|_| {
        Stack::new([
            probe(
                "stack_base",
                SizedBox::new()
                    .width(300.)
                    .height(200.)
                    .child(colored(BLUE)),
            ),
            Widget::from(
                Positioned::new(probe("positioned", colored(ORANGE)))
                    .left(35.)
                    .top(45.)
                    .width(70.)
                    .height(50.),
            ),
        ])
        .into()
    });
    h.frames(1);
}

fn indexed_stack_active(h: &mut FrameHarness) {
    h.mount(|_| {
        IndexedStack::new([
            probe(
                "indexed0",
                SizedBox::new().width(100.).height(40.).child(colored(BLUE)),
            ),
            probe(
                "indexed1",
                SizedBox::new()
                    .width(140.)
                    .height(60.)
                    .child(colored(ORANGE)),
            ),
        ])
        .index(1)
        .into()
    });
    h.frames(1);
}

fn wrap_runs(h: &mut FrameHarness) {
    h.mount(|_| {
        SizedBox::new()
            .width(230.)
            .child(
                Wrap::new([
                    probe(
                        "wrap0",
                        SizedBox::new().width(70.).height(20.).child(colored(BLUE)),
                    ),
                    probe(
                        "wrap1",
                        SizedBox::new()
                            .width(80.)
                            .height(30.)
                            .child(colored(ORANGE)),
                    ),
                    probe(
                        "wrap2",
                        SizedBox::new().width(90.).height(25.).child(colored(GREEN)),
                    ),
                    probe(
                        "wrap3",
                        SizedBox::new().width(60.).height(35.).child(colored(BLUE)),
                    ),
                ])
                .spacing(10.)
                .run_spacing(8.)
                .alignment(WrapAlignment::SpaceBetween),
            )
            .into()
    });
    h.frames(1);
}

fn aspect_constraints_fraction(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            SizedBox::new().width(300.).height(180.).child(Widget::from(
                incular_widgets::Align::new(
                    Alignment::TOP_LEFT,
                    probe(
                        "aspect",
                        AspectRatio::new(
                            2.,
                            ConstrainedBox::new(
                                Constraints::at_most(200., 120.),
                                FractionallySizedBox::new(colored(GREEN))
                                    .width_factor(0.5)
                                    .height_factor(0.5),
                            ),
                        ),
                    ),
                ),
            )),
        )
        .into()
    });
    h.frames(1);
}

fn fitted_box(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(probe(
            "fitted",
            SizedBox::new().width(160.).height(100.).child(
                FittedBox::new(
                    SizedBox::new()
                        .width(100.)
                        .height(80.)
                        .child(colored(ORANGE)),
                )
                .alignment(Alignment::BOTTOM_RIGHT),
            ),
        ))
        .into()
    });
    h.frames(1);
}

fn padding_overflow(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(probe(
            "padding",
            Padding::new(
                EdgeInsets::symmetric(20., 15.),
                SizedOverflowBox::new(
                    Size::new(120., 70.),
                    SizedBox::new()
                        .width(180.)
                        .height(90.)
                        .child(colored(GREEN)),
                )
                .alignment(Alignment::BOTTOM_RIGHT),
            ),
        ))
        .into()
    });
    h.frames(1);
}

fn table_layout(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Table::new(
                2,
                [
                    probe("table0", SizedBox::new().height(30.).child(colored(BLUE))),
                    SizedBox::new()
                        .width(80.)
                        .height(40.)
                        .child(colored(ORANGE))
                        .into(),
                    SizedBox::new().height(50.).child(colored(GREEN)).into(),
                    SizedBox::new()
                        .width(100.)
                        .height(35.)
                        .child(colored(BLUE))
                        .into(),
                ],
            )
            .column_spacing(0.)
            .row_spacing(0.),
        )
        .into()
    });
    h.frames(1);
}

fn baseline_layout(h: &mut FrameHarness) {
    h.mount(|_| {
        Row::new([
            probe(
                "baseline0",
                incular_widgets::Text::new("Inter 12")
                    .style(incular_text::TextStyle::default().font_size(12.)),
            ),
            probe(
                "baseline1",
                incular_widgets::Text::new("Inter 24")
                    .style(incular_text::TextStyle::default().font_size(24.)),
            ),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Baseline)
        .into()
    });
    h.frames(1);
}

struct SingleDelegate;

impl SingleChildLayoutDelegate for SingleDelegate {
    fn get_size(&self, constraints: Constraints) -> Size {
        constraints.constrain(Size::new(300., 200.))
    }
    fn get_constraints_for_child(&self, _: Constraints) -> Constraints {
        Constraints::tight(Size::new(70., 40.))
    }
    fn get_position_for_child(&self, _: Size, _: Size) -> Offset {
        Offset::new(40., 30.)
    }
    fn should_relayout(&self, _: &Self) -> bool {
        false
    }
}

fn custom_single(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(probe(
            "custom_single",
            CustomSingleChildLayout::new(
                SingleDelegate,
                SizedBox::new().width(70.).height(40.).child(colored(BLUE)),
            ),
        ))
        .into()
    });
    h.frames(1);
}

struct MultiDelegate;

impl MultiChildLayoutDelegate for MultiDelegate {
    fn get_size(&self, constraints: Constraints) -> Size {
        constraints.constrain(Size::new(200., 150.))
    }
    fn perform_layout(&self, size: Size, children: &mut MultiChildLayoutContext<'_>) {
        children.layout_child("a", Constraints::tight(Size::new(40., 30.)));
        children.position_child("a", Offset::new(20., 25.));
        children.layout_child("b", Constraints::tight(Size::new(50., 20.)));
        children.position_child("b", Offset::new(90., 70.));
        let _ = size;
    }
    fn should_relayout(&self, _: &Self) -> bool {
        false
    }
}

fn custom_multi(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(CustomMultiChildLayout::new(
            MultiDelegate,
            [
                LayoutId::new("a", probe("multi_a", colored(BLUE))),
                LayoutId::new("b", probe("multi_b", colored(ORANGE))),
            ],
        ))
        .into()
    });
    h.frames(1);
}

struct SimpleFlow;

impl FlowDelegate for SimpleFlow {
    fn get_size(&self, constraints: Constraints) -> Size {
        constraints.constrain(Size::new(300., 200.))
    }
    fn paint_children(&self, context: &mut FlowPaintingContext<'_>) {
        context.paint_child(0, CoreTransform::translation(Offset::new(40., 50.)), 1.);
        context.paint_child(1, CoreTransform::translation(Offset::new(120., 80.)), 1.);
    }
    fn should_repaint(&self, _: &Self) -> bool {
        false
    }
}

fn flow_layout(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(Flow::new(
            SimpleFlow,
            [
                probe(
                    "flow0",
                    SizedBox::new().width(40.).height(30.).child(colored(BLUE)),
                ),
                probe(
                    "flow1",
                    SizedBox::new()
                        .width(50.)
                        .height(40.)
                        .child(colored(ORANGE)),
                ),
            ],
        ))
        .into()
    });
    h.frames(1);
}

fn layout_builder(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(LayoutBuilder::new(|_, constraints| {
            let w = constraints.max_width() / 2.;
            SizedBox::new()
                .width(w)
                .height(40.)
                .child(probe("layout_builder", colored(BLUE)))
                .into()
        }))
        .into()
    });
    h.frames(1);
}

fn intrinsic_width(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(probe(
            "intrinsic",
            IntrinsicWidth::new(
                SizedBox::new()
                    .width(83.)
                    .height(40.)
                    .child(colored(ORANGE)),
            )
            .step_width(20.),
        ))
        .into()
    });
    h.frames(1);
}

fn box_constraints_limited_overflow_intrinsic(h: &mut FrameHarness) {
    h.mount(|_| {
        Column::new([
            probe(
                "limited",
                UnconstrainedBox::new(
                    LimitedBox::new(SizedBox::expand().child(colored(BLUE)))
                        .max_width(120.)
                        .max_height(60.),
                ),
            ),
            SizedBox::new()
                .width(100.)
                .height(80.)
                .child(
                    OverflowBox::new(probe("overflow", colored(ORANGE)))
                        .min_width(150.)
                        .max_width(150.)
                        .min_height(110.)
                        .max_height(110.),
                )
                .into(),
            probe(
                "intrinsic_height",
                IntrinsicHeight::new(Row::new([
                    SizedBox::new().width(40.).height(20.).child(colored(GREEN)),
                    SizedBox::new().width(40.).height(50.).child(colored(BLUE)),
                ])),
            ),
        ])
        .main_axis_size(MainAxisSize::Min)
        .into()
    });
    h.frames(1);
}

fn visibility_transform(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new([
                probe(
                    "transformed",
                    Transform::translation(
                        Offset::new(25., 15.),
                        SizedBox::new().width(60.).height(40.).child(colored(BLUE)),
                    ),
                ),
                Offstage::new(probe(
                    "offstage",
                    SizedBox::new()
                        .width(90.)
                        .height(25.)
                        .child(colored(ORANGE)),
                ))
                .into(),
                Visibility::new(probe(
                    "hidden",
                    SizedBox::new().width(50.).height(20.).child(colored(GREEN)),
                ))
                .visible(false)
                .maintain_animation(true)
                .maintain_state(true)
                .into(),
            ])
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn w<const N: usize>(items: [Widget; N]) -> [Widget; N] {
    items
}
fn sized_block(width: f32, height: f32, color: Color) -> Widget {
    SizedBox::new()
        .width(width)
        .height(height)
        .child(colored(color))
        .into()
}

fn main_alignment_variants(h: &mut FrameHarness) {
    fn aligned_row(alignment: MainAxisAlignment, tag: &str) -> Widget {
        SizedBox::new()
            .width(300.)
            .height(30.)
            .child(
                Row::new(w([
                    probe(&format!("{tag}0"), sized_block(40., 20., BLUE)),
                    probe(&format!("{tag}1"), sized_block(60., 20., ORANGE)),
                ]))
                .main_axis_alignment(alignment),
            )
            .into()
    }
    h.mount(|_| {
        Center::new(
            Column::new(w([
                aligned_row(MainAxisAlignment::Start, "start"),
                aligned_row(MainAxisAlignment::End, "end"),
                aligned_row(MainAxisAlignment::Center, "center"),
                aligned_row(MainAxisAlignment::SpaceBetween, "between"),
                aligned_row(MainAxisAlignment::SpaceAround, "around"),
                aligned_row(MainAxisAlignment::SpaceEvenly, "evenly"),
                Row::new(w([
                    probe("min0", sized_block(40., 20., BLUE)),
                    probe("min1", sized_block(60., 20., ORANGE)),
                ]))
                .main_axis_size(MainAxisSize::Min)
                .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn direction_reversal(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new(w([
                SizedBox::new()
                    .width(300.)
                    .height(20.)
                    .child(
                        Row::new(w([
                            probe("rtl0", sized_block(50., 20., BLUE)),
                            Spacer::new().into(),
                            probe("rtl1", sized_block(80., 20., ORANGE)),
                        ]))
                        .text_direction(TextDirection::Rtl),
                    )
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(60.)
                    .child(
                        Row::new(w([
                            probe("vup0", sized_block(50., 20., GREEN)),
                            probe("vup1", sized_block(40., 40., BLUE)),
                        ]))
                        .vertical_direction(VerticalDirection::Up)
                        .cross_axis_alignment(CrossAxisAlignment::Start),
                    )
                    .into(),
                SizedBox::new()
                    .width(80.)
                    .height(120.)
                    .child(
                        Column::new(w([
                            probe("cup0", sized_block(30., 20., ORANGE)),
                            probe("cup1", sized_block(30., 40., GREEN)),
                        ]))
                        .vertical_direction(VerticalDirection::Up)
                        .main_axis_alignment(MainAxisAlignment::Start),
                    )
                    .into(),
                SizedBox::new()
                    .width(100.)
                    .height(60.)
                    .child(
                        Column::new(w([
                            probe("ctr0", sized_block(30., 20., BLUE)),
                            probe("ctr1", sized_block(60., 20., ORANGE)),
                        ]))
                        .text_direction(TextDirection::Rtl)
                        .cross_axis_alignment(CrossAxisAlignment::Start),
                    )
                    .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn fit_loose_and_tight(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(SizedBox::new().width(400.).height(40.).child(Row::new(w([
            Widget::from(Flexible::new(probe("fit_loose_color", colored(BLUE)))),
            Widget::from(Flexible::new(probe(
                "fit_loose_sized",
                sized_block(30., 20., ORANGE),
            ))),
            Widget::from(Expanded::new(probe(
                "fit_tight_sized",
                sized_block(30., 20., GREEN),
            ))),
            Widget::from(Flexible::new(probe(
                "fit_loose_fraction",
                FractionallySizedBox::new(colored(BLUE)).width_factor(1.),
            ))),
        ]))))
        .into()
    });
    h.frames(1);
}

fn spacing_and_fixed_gaps(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new(w([
                SizedBox::new()
                    .width(250.)
                    .height(30.)
                    .child(
                        Row::new(w([
                            probe("sp0", sized_block(40., 20., BLUE)),
                            probe("sp1", sized_block(60., 20., ORANGE)),
                            probe("sp2", sized_block(30., 20., GREEN)),
                        ]))
                        .spacing(10.),
                    )
                    .into(),
                SizedBox::new()
                    .width(200.)
                    .height(30.)
                    .child(
                        Row::new(w([
                            probe("flex_fixed", sized_block(60., 20., BLUE)),
                            Widget::from(Expanded::new(probe("flex_fill", colored(ORANGE)))),
                        ]))
                        .spacing(10.)
                        .main_axis_alignment(MainAxisAlignment::Center),
                    )
                    .into(),
                Row::new(w([
                    probe("minrow0", sized_block(20., 20., GREEN)),
                    probe("minrow1", sized_block(30., 20., BLUE)),
                ]))
                .main_axis_size(MainAxisSize::Min)
                .spacing(8.)
                .into(),
                Column::new(w([
                    probe("vcol0", sized_block(40., 10., BLUE)),
                    probe("vcol1", sized_block(40., 10., ORANGE)),
                    probe("vcol2", sized_block(40., 10., GREEN)),
                ]))
                .main_axis_size(MainAxisSize::Min)
                .spacing(6.)
                .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn stack_fit_alignment_positioned(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new(w([
                SizedBox::new()
                    .width(300.)
                    .height(100.)
                    .child(
                        Stack::new(w([probe("s_loose", sized_block(60., 40., BLUE))]))
                            .alignment(Alignment::BOTTOM_RIGHT),
                    )
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(100.)
                    .child(
                        Stack::new(w([probe("s_expand", colored(ORANGE))])).fit(StackFit::Expand),
                    )
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(100.)
                    .child(incular_widgets::Align::new(
                        Alignment::TOP_LEFT,
                        Stack::new(w([probe("s_intrinsic", sized_block(70., 30., GREEN))])),
                    ))
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(100.)
                    .child(Stack::new(w([
                        sized_block(300., 100., BLUE),
                        Widget::from(
                            Positioned::new(probe("s_pos", colored(ORANGE)))
                                .right(10.)
                                .bottom(5.)
                                .width(30.)
                                .height(20.),
                        ),
                    ])))
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(100.)
                    .child(Stack::new(w([
                        sized_block(300., 100., BLUE),
                        Widget::from(
                            Positioned::new(probe("s_neg", colored(ORANGE)))
                                .left(-20.)
                                .right(10.)
                                .top(10.)
                                .height(30.),
                        ),
                    ])))
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(100.)
                    .child(Center::new(
                        Stack::new(w([probe("s_pass", sized_block(900., 50., GREEN))]))
                            .fit(StackFit::Passthrough),
                    ))
                    .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn stack_empty_positioned_only(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Row::new(w([
                SizedBox::new()
                    .width(150.)
                    .height(100.)
                    .child(probe("empty_stack", Stack::new(Vec::<Widget>::new())))
                    .into(),
                SizedBox::new()
                    .width(150.)
                    .height(100.)
                    .child(probe(
                        "positioned_only",
                        Stack::new(w([Widget::from(
                            Positioned::new(colored(BLUE))
                                .left(10.)
                                .top(10.)
                                .width(50.)
                                .height(20.),
                        )])),
                    ))
                    .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn wrap_alignment_run_axes(h: &mut FrameHarness) {
    h.mount(|_| {
        Column::new(w([
            SizedBox::new()
                .width(230.)
                .height(120.)
                .child(
                    Wrap::new(w([
                        probe("wa0", sized_block(70., 20., BLUE)),
                        probe("wa1", sized_block(80., 30., ORANGE)),
                        probe("wa2", sized_block(90., 25., GREEN)),
                        probe("wa3", sized_block(60., 35., BLUE)),
                    ]))
                    .spacing(10.)
                    .run_spacing(8.)
                    .alignment(WrapAlignment::Center)
                    .run_alignment(WrapAlignment::End)
                    .cross_axis_alignment(WrapCrossAlignment::End),
                )
                .into(),
            SizedBox::new()
                .width(200.)
                .height(150.)
                .child(
                    Wrap::new(w([
                        probe("vw0", sized_block(40., 30., BLUE)),
                        probe("vw1", sized_block(60., 50., ORANGE)),
                        probe("vw2", sized_block(30., 40., GREEN)),
                        probe("vw3", sized_block(50., 20., BLUE)),
                    ]))
                    .direction(Axis::Vertical)
                    .spacing(6.)
                    .run_spacing(10.)
                    .alignment(WrapAlignment::Start)
                    .run_alignment(WrapAlignment::SpaceBetween)
                    .cross_axis_alignment(WrapCrossAlignment::Center),
                )
                .into(),
            SizedBox::new()
                .width(230.)
                .child(
                    Wrap::new(w([
                        probe("rw0", sized_block(70., 20., BLUE)),
                        probe("rw1", sized_block(80., 30., ORANGE)),
                        probe("rw2", sized_block(90., 25., GREEN)),
                    ]))
                    .text_direction(TextDirection::Rtl)
                    .spacing(10.)
                    .run_spacing(8.),
                )
                .into(),
            SizedBox::new()
                .width(230.)
                .child(
                    Wrap::new(w([
                        probe("sa0", sized_block(70., 20., BLUE)),
                        probe("sa1", sized_block(80., 30., ORANGE)),
                    ]))
                    .alignment(WrapAlignment::SpaceAround),
                )
                .into(),
        ]))
        .main_axis_size(MainAxisSize::Min)
        .into()
    });
    h.frames(1);
}

fn fitted_box_all_fits(h: &mut FrameHarness) {
    fn fitted(fit: BoxFit, tag: &str, alignment: Alignment) -> Widget {
        SizedBox::new()
            .width(120.)
            .height(80.)
            .child(
                FittedBox::new(probe(tag, sized_block(200., 100., BLUE)))
                    .fit(fit)
                    .alignment(alignment),
            )
            .into()
    }
    h.mount(|_| {
        Center::new(
            Column::new(w([
                Row::new(w([
                    fitted(BoxFit::Fill, "fb_fill", Alignment::CENTER),
                    fitted(BoxFit::Contain, "fb_contain", Alignment::CENTER),
                    fitted(BoxFit::Cover, "fb_cover", Alignment::CENTER),
                    fitted(BoxFit::FitWidth, "fb_fit_width", Alignment::CENTER),
                ]))
                .spacing(10.)
                .main_axis_size(MainAxisSize::Min)
                .into(),
                SizedBox::new().height(10.).into(),
                Row::new(w([
                    fitted(BoxFit::FitHeight, "fb_fit_height", Alignment::CENTER),
                    fitted(BoxFit::None, "fb_none", Alignment::CENTER),
                    fitted(BoxFit::ScaleDown, "fb_scale_down", Alignment::CENTER),
                ]))
                .spacing(10.)
                .main_axis_size(MainAxisSize::Min)
                .into(),
                SizedBox::new().height(10.).into(),
                Row::new(w([
                    fitted(BoxFit::Contain, "fb_contain_tl", Alignment::TOP_LEFT),
                    fitted(BoxFit::Cover, "fb_cover_br", Alignment::BOTTOM_RIGHT),
                    fitted(BoxFit::None, "fb_none_tl", Alignment::TOP_LEFT),
                ]))
                .spacing(10.)
                .main_axis_size(MainAxisSize::Min)
                .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn aspect_ratio_cases(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new(w([
                SizedBox::new()
                    .width(800.)
                    .height(100.)
                    .child(Center::new(probe(
                        "ar_wide",
                        AspectRatio::new(4., colored(BLUE)),
                    )))
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(100.)
                    .child(probe("ar_tight", AspectRatio::new(2., colored(ORANGE))))
                    .into(),
                SizedBox::new()
                    .width(800.)
                    .height(100.)
                    .child(Center::new(ConstrainedBox::new(
                        Constraints::at_most(200., 300.),
                        probe("ar_narrow", AspectRatio::new(0.5, colored(GREEN))),
                    )))
                    .into(),
                SizedBox::new()
                    .width(800.)
                    .height(100.)
                    .child(Center::new(probe(
                        "ar_child",
                        AspectRatio::new(2., probe("ar_child_inner", sized_block(40., 40., BLUE))),
                    )))
                    .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn container_composition_order(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new(w([
                probe(
                    "container",
                    Container::with_child(probe("container_child", sized_block(30., 20., ORANGE)))
                        .width(160.)
                        .height(100.)
                        .margin(EdgeInsets::all(10.))
                        .padding(EdgeInsets::symmetric(6., 4.))
                        .color(BLUE)
                        .alignment(Alignment::BOTTOM_RIGHT),
                ),
                SizedBox::new().height(20.).into(),
                probe(
                    "rotated",
                    Container::new()
                        .width(100.)
                        .height(50.)
                        .color(ORANGE)
                        .transform(CoreTransform::rotation(0.5)),
                ),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn intrinsic_width_steps_flex(h: &mut FrameHarness) {
    h.mount(|_| {
        Column::new(w([
            probe(
                "iw_steps",
                IntrinsicWidth::new(
                    Column::new(w([
                        sized_block(60., 22., BLUE),
                        sized_block(80., 10., ORANGE),
                    ]))
                    .main_axis_size(MainAxisSize::Min),
                )
                .step_width(25.)
                .step_height(15.),
            ),
            probe(
                "iw_flex",
                IntrinsicWidth::new(Row::new(w([
                    SizedBox::new()
                        .width(40.)
                        .height(20.)
                        .child(colored(GREEN))
                        .into(),
                    Widget::from(Expanded::new(probe(
                        "iw_flex_child",
                        sized_block(20., 20., BLUE),
                    ))),
                ]))),
            ),
        ]))
        .main_axis_size(MainAxisSize::Min)
        .into()
    });
    h.frames(1);
}

fn baseline_without_baseline_child(h: &mut FrameHarness) {
    h.mount(|_| {
        Column::new(w([
            Row::new(w([
                probe("bl_box", sized_block(40., 30., BLUE)),
                probe(
                    "bl_text",
                    Text::new("Inter 24").style(incular_text::TextStyle::default().font_size(24.)),
                ),
            ]))
            .cross_axis_alignment(CrossAxisAlignment::Baseline)
            .into(),
            Column::new(w([
                probe(
                    "col_text",
                    Text::new("Inter 12").style(incular_text::TextStyle::default().font_size(12.)),
                ),
                probe("col_box", sized_block(40., 30., ORANGE)),
            ]))
            .cross_axis_alignment(CrossAxisAlignment::Baseline)
            .into(),
        ]))
        .main_axis_size(MainAxisSize::Min)
        .into()
    });
    h.frames(1);
}

fn unconstrained_constrained_axis(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new(w([
                SizedBox::new()
                    .width(300.)
                    .height(200.)
                    .child(
                        UnconstrainedBox::new(probe("ub_h", sized_block(400., 260., BLUE)))
                            .constrained_axis(Axis::Horizontal),
                    )
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(200.)
                    .child(
                        UnconstrainedBox::new(probe("ub_v", sized_block(400., 100., ORANGE)))
                            .constrained_axis(Axis::Vertical),
                    )
                    .into(),
                SizedBox::new()
                    .width(300.)
                    .height(200.)
                    .child(UnconstrainedBox::new(probe(
                        "ub_none",
                        sized_block(400., 260., GREEN),
                    )))
                    .into(),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn layout_builder_rebuild_triggers(h: &mut FrameHarness) {
    let builds = Rc::new(Cell::new(0u32));
    let counter = Rc::clone(&builds);
    h.value("builds", move || f64::from(builds.get()));
    h.mount(move |phase| {
        let counter = Rc::clone(&counter);
        Center::new(
            SizedBox::new()
                .width(if phase == 0 { 300. } else { 200. })
                .child(LayoutBuilder::new(move |_, _constraints| {
                    counter.set(counter.get() + 1);
                    probe(
                        "lb_rebuild",
                        SizedBox::new().height(40.).child(colored(BLUE)),
                    )
                })),
        )
        .into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(2);
    h.set_phase(2);
    h.frames(2);
}

fn visibility_maintain_flags(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            Column::new(w([
                probe(
                    "vis_size",
                    Visibility::new(sized_block(50., 20., BLUE))
                        .visible(false)
                        .maintain_size(true)
                        .maintain_animation(true)
                        .maintain_state(true),
                ),
                probe(
                    "vis_state",
                    Visibility::new(sized_block(60., 10., ORANGE))
                        .visible(false)
                        .maintain_state(true),
                ),
                probe(
                    "vis_default",
                    Visibility::new(sized_block(40., 30., GREEN)).visible(false),
                ),
                probe(
                    "vis_replacement",
                    Visibility::new(sized_block(40., 30., ORANGE))
                        .visible(false)
                        .replacement(sized_block(25., 25., BLUE)),
                ),
            ]))
            .main_axis_size(MainAxisSize::Min),
        )
        .into()
    });
    h.frames(1);
}

fn box_constraints_helpers(h: &mut FrameHarness) {
    let c = Constraints::new(50., 200., 20., 100.);
    h.value("constrain_w", move || {
        f64::from(c.constrain(Size::new(10., 500.)).width)
    });
    h.value("constrain_h", move || {
        f64::from(c.constrain(Size::new(10., 500.)).height)
    });
    h.value("loosen_min_w", move || f64::from(c.loosen().min_width()));
    h.value("tighten_min_w", move || {
        f64::from(c.tighten(Some(120.), None).min_width())
    });
    h.value("tighten_max_w", move || {
        f64::from(c.tighten(Some(120.), None).max_width())
    });
    h.value("deflate_min_w", move || {
        f64::from(c.deflate(20., 20.).min_width())
    });
    h.value("deflate_max_w", move || {
        f64::from(c.deflate(20., 20.).max_width())
    });
    h.value("biggest_w", move || f64::from(c.biggest().width));
    h.value("smallest_h", move || f64::from(c.smallest().height));
    h.value("tight_min_w", move || {
        f64::from(Constraints::tight(Size::new(70., 40.)).min_width())
    });
    h.value(
        "bounded_w",
        move || if c.is_width_bounded() { 1. } else { 0. },
    );
    h.value("unbounded_w", move || {
        if Constraints::new(1., f32::INFINITY, 0., f32::INFINITY).is_width_bounded() {
            1.
        } else {
            0.
        }
    });
    h.mount(|_| sized_block(10., 10., BLUE));
    h.frames(1);
}

fn table_default_flex_columns(h: &mut FrameHarness) {
    // Non-equivalent diagnostic: Flutter's default is FlexColumnWidth; Incular's
    // Table exposes only max-content (intrinsic) columns, so this twin
    // records Incular's intrinsic layout for the same children.
    h.mount(|_| {
        Center::new(probe(
            "table",
            Table::new(
                2,
                [
                    SizedBox::new().height(30.).child(colored(BLUE)).into(),
                    sized_block(80., 40., ORANGE),
                    SizedBox::new().height(50.).child(colored(GREEN)).into(),
                    sized_block(100., 35., BLUE),
                ],
            ),
        ))
        .into()
    });
    h.frames(1);
}

/// Orchestrator: a null factor keeps the parent minimum in Flutter
/// (`shifted_box.dart` `_getInnerConstraints`) under a tight parent.
fn fractionally_sized_box_null_factor_tight_parent(h: &mut FrameHarness) {
    h.mount(|_| {
        Center::new(
            SizedBox::new().width(200.).height(100.).child(
                FractionallySizedBox::new(probe(
                    "child",
                    SizedBox::new().width(50.).height(20.).child(colored(BLUE)),
                ))
                .height_factor(0.5),
            ),
        )
        .into()
    });
    h.frames(1);
}
