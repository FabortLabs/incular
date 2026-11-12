use incular_config::{
    Alignment, Constraints, CrossAxisAlignment, EdgeInsets, MainAxisAlignment, MainAxisSize,
    WrapAlignment,
};
use incular_core::{Color, Offset, Size, Transform as CoreTransform};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    AspectRatio, Center, ColoredBox, Column, ConstrainedBox, CustomMultiChildLayout,
    CustomSingleChildLayout, Expanded, FittedBox, Flow, FlowDelegate, FlowPaintingContext,
    FractionallySizedBox, IndexedStack, IntrinsicHeight, IntrinsicWidth, LayoutBuilder, LayoutId,
    LimitedBox, MultiChildLayoutContext, MultiChildLayoutDelegate, Offstage, OverflowBox, Padding,
    Positioned, Row, SingleChildLayoutDelegate, SizedBox, SizedOverflowBox, Stack, Table,
    Transform, UnconstrainedBox, Visibility, Widget, Wrap,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

pub fn scenarios() -> Vec<Scenario> {
    vec![
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
