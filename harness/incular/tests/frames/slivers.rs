use incular_config::EdgeInsets;
use incular_core::{Color, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    ColoredBox, CustomScrollView, KeepAlive, PinnedHeaderSliver, ScrollController, SizedBox,
    Sliver, SliverCrossAxisGroup, SliverFillRemaining, SliverFillViewport, SliverFixedExtentList,
    SliverFloatingHeader, SliverGrid, SliverGridDelegate, SliverList, SliverMainAxisGroup,
    SliverOffstage, SliverOpacity, SliverPadding, SliverPersistentHeader,
    SliverPrototypeExtentList, SliverResizingHeader, SliverToBoxAdapter, SliverVariedExtentList,
    Widget,
};

const BLUE: Color = Color::rgba(0x21, 0x96, 0xF3, 0xFF);
const ORANGE: Color = Color::rgba(0xFF, 0x57, 0x22, 0xFF);
const GREEN: Color = Color::rgba(0x4C, 0xAF, 0x50, 0xFF);

fn solid(color: Color) -> Widget {
    ColoredBox::new(color, SizedBox::shrink()).into()
}

fn row_box(name: &str, height: f32, color: Color) -> Widget {
    probe(name, SizedBox::new().height(height).child(solid(color)))
}

pub fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "slivers__box_padding_fill_remaining__static_and_scroll",
            box_padding_fill_remaining,
        ),
        ("slivers__varied_extent_list__indices", varied_extent_list),
        (
            "slivers__fixed_prototype_extent_lists__indices",
            fixed_prototype_extent_lists,
        ),
        (
            "slivers__grid_and_fill_viewport__geometry",
            grid_and_fill_viewport,
        ),
        (
            "slivers__pinned_header_and_main_axis_group__scroll",
            pinned_header_and_main_axis_group,
        ),
        ("slivers__cross_axis_group__lanes", cross_axis_group),
        (
            "slivers__pinned_header_sliver__child_extent",
            pinned_header_sliver_child_extent,
        ),
        (
            "slivers__custom_scroll_view__cache_extent",
            custom_scroll_view_cache_extent,
        ),
        (
            "slivers__floating_header__default_snap_after_wheel_scroll",
            floating_header_default_snap_after_wheel_scroll,
        ),
        (
            "slivers__resizing_header__collapse_scroll",
            resizing_header_collapse_scroll,
        ),
        (
            "slivers__fill_viewport__pad_ends_fraction",
            fill_viewport_pad_ends_fraction,
        ),
        (
            "slivers__list_builder__default_cache_window",
            list_builder_default_cache_window,
        ),
        (
            "slivers__opacity_offstage__visibility",
            opacity_offstage_visibility,
        ),
        (
            "slivers__fill_remaining_no_scroll_body__shrink_after_preceding_growth",
            fill_remaining_shrink_after_growth,
        ),
        (
            "slivers__keep_alive__retains_scrolled_child",
            keep_alive_retains_scrolled_child,
        ),
    ]
}

fn box_padding_fill_remaining(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let list = SliverList::delegate(incular_widgets::SliverChildListDelegate::new(vec![
                row_box("padded0", 36., ORANGE),
                row_box("padded1", 52., GREEN),
                row_box("padded2", 40., BLUE),
            ]));
            let slivers: Vec<Box<dyn Sliver>> = vec![
                Box::new(SliverToBoxAdapter::new(row_box("adapter", 45., BLUE))),
                Box::new(SliverPadding::new(EdgeInsets::symmetric(20., 8.), list)),
                Box::new(
                    SliverFillRemaining::new(probe("fill_remaining", solid(GREEN)))
                        .has_scroll_body(false),
                ),
            ];
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .into()
        }
    });
    h.frames(2);
    h.down(Offset::new(400., 500.));
    h.frames(1);
    h.move_to(Offset::new(400., 430.));
    h.frames(2);
    h.up();
    h.frames(2);
}

fn varied_extent_list(h: &mut FrameHarness) {
    h.mount(|_| {
        CustomScrollView::new([Box::new(SliverVariedExtentList::new(
            4,
            |index| 30. + index as f32 * 7.,
            |index| {
                probe(
                    &format!("varied{index}"),
                    solid(if index % 2 == 0 { GREEN } else { BLUE }),
                )
            },
        )) as Box<dyn Sliver>])
        .into()
    });
    h.frames(2);
}

fn fixed_prototype_extent_lists(h: &mut FrameHarness) {
    h.mount(|_| {
        let fixed = SliverFixedExtentList::new(4, 38., |index| {
            probe(
                &format!("fixed{index}"),
                solid(if index % 2 == 0 { BLUE } else { ORANGE }),
            )
        });
        let prototype = SliverPrototypeExtentList::new(4, SizedBox::new().height(44.), |index| {
            probe(
                &format!("prototype{index}"),
                solid(if index % 2 == 0 { ORANGE } else { GREEN }),
            )
        });
        CustomScrollView::new([
            Box::new(fixed) as Box<dyn Sliver>,
            Box::new(SliverToBoxAdapter::new(row_box("between", 24., GREEN))),
            Box::new(prototype),
        ])
        .into()
    });
    h.frames(4);
}

fn grid_and_fill_viewport(h: &mut FrameHarness) {
    h.mount(|_| {
        let grid = SliverGrid::builder(
            6,
            SliverGridDelegate::fixed_cross_axis_count(3)
                .main_axis_extent(54.)
                .main_axis_spacing(6.)
                .cross_axis_spacing(8.),
            |index| {
                probe(
                    &format!("grid{index}"),
                    solid(if index % 2 == 0 { BLUE } else { ORANGE }),
                )
            },
        );
        let fill =
            SliverFillViewport::new([probe("page0", solid(GREEN)), probe("page1", solid(BLUE))])
                .viewport_fraction(0.75)
                .pad_ends(false);
        let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(grid), Box::new(fill)];
        CustomScrollView::new(slivers).into()
    });
    h.frames(3);
}

fn pinned_header_and_main_axis_group(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let rows = SliverFixedExtentList::new(20, 46., |index| {
                row_box(
                    &format!("row{index}"),
                    46.,
                    if index % 2 == 0 { ORANGE } else { GREEN },
                )
            });
            let group = SliverMainAxisGroup::new([
                Box::new(
                    SliverPersistentHeader::new(70., probe("pinned", solid(BLUE))).pinned(true),
                ) as Box<dyn Sliver>,
                Box::new(rows),
            ]);
            let slivers: Vec<Box<dyn Sliver>> = vec![
                Box::new(group),
                Box::new(SliverToBoxAdapter::new(row_box("after_group", 50., BLUE))),
            ];
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .into()
        }
    });
    h.frames(1);
    controller.jump_to(135.);
    h.frames(4);
}

fn cross_axis_group(h: &mut FrameHarness) {
    h.mount(|_| {
        let group = SliverCrossAxisGroup::new([
            Box::new(SliverToBoxAdapter::new(probe(
                "lane_left",
                SizedBox::new().height(100.).child(solid(BLUE)),
            ))) as Box<dyn Sliver>,
            Box::new(SliverToBoxAdapter::new(probe(
                "lane_right",
                SizedBox::new().height(100.).child(solid(ORANGE)),
            ))) as Box<dyn Sliver>,
        ]);
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(group),
            Box::new(SliverToBoxAdapter::new(row_box("after_lanes", 50., GREEN))),
        ];
        CustomScrollView::new(slivers).into()
    });
    h.frames(2);
}

fn pinned_header_sliver_child_extent(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let slivers: Vec<Box<dyn Sliver>> = vec![
                Box::new(PinnedHeaderSliver::new(probe(
                    "pinned_child",
                    SizedBox::new().height(60.).child(solid(BLUE)),
                ))),
                Box::new(SliverFixedExtentList::new(20, 42., |index| {
                    row_box(
                        &format!("body{index}"),
                        42.,
                        if index % 2 == 0 { GREEN } else { ORANGE },
                    )
                })),
            ];
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .into()
        }
    });
    controller.jump_to(80.);
    h.frames(3);
}

fn custom_scroll_view_cache_extent(h: &mut FrameHarness) {
    h.mount(|_| {
        let rows = SliverFixedExtentList::new(30, 40., |index| {
            probe(
                &format!("cache{index}"),
                solid(if index % 2 == 0 { BLUE } else { ORANGE }),
            )
        });
        CustomScrollView::new([Box::new(rows) as Box<dyn Sliver>])
            .cache_extent(180.)
            .into()
    });
    h.frames(2);
}

fn floating_header_default_snap_after_wheel_scroll(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let rows = SliverFixedExtentList::new(18, 40., |index| {
                row_box(
                    &format!("floating_body{index}"),
                    40.,
                    if index % 2 == 0 { ORANGE } else { GREEN },
                )
            });
            let slivers: Vec<Box<dyn Sliver>> = vec![
                Box::new(SliverFloatingHeader::new(row_box(
                    "floating_header",
                    70.,
                    BLUE,
                ))),
                Box::new(rows),
            ];
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .into()
        }
    });
    h.frames(2);
    controller.jump_to(100.);
    h.frames(2);
    h.wheel(Offset::new(400., 300.), Offset::new(0., -40.));
    h.frames(30);
}

fn resizing_header_collapse_scroll(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let header = SliverResizingHeader::builder()
                .min_extent(40.)
                .max_extent(120.)
                .child(probe("resizing", solid(BLUE)))
                .build();
            let rows = SliverFixedExtentList::new(30, 40., |index| {
                row_box(
                    &format!("resize_body{index}"),
                    40.,
                    if index % 2 == 0 { ORANGE } else { GREEN },
                )
            });
            let slivers: Vec<Box<dyn Sliver>> = vec![Box::new(header), Box::new(rows)];
            CustomScrollView::new(slivers)
                .controller(controller.clone())
                .into()
        }
    });
    h.frames(2);
    controller.jump_to(60.);
    h.frames(2);
    controller.jump_to(200.);
    h.frames(2);
}

fn fill_viewport_pad_ends_fraction(h: &mut FrameHarness) {
    h.mount(|_| {
        let fill = SliverFillViewport::new([
            probe("fill_page0", solid(GREEN)),
            probe("fill_page1", solid(BLUE)),
            probe("fill_page2", solid(ORANGE)),
        ])
        .viewport_fraction(0.8)
        .pad_ends(true);
        CustomScrollView::new([Box::new(fill) as Box<dyn Sliver>]).into()
    });
    h.frames(2);
}

fn list_builder_default_cache_window(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let list = SliverList::builder(60, |index| {
                row_box(
                    &format!("list{index}"),
                    50.,
                    if index % 2 == 0 { BLUE } else { ORANGE },
                )
            });
            CustomScrollView::new([Box::new(list) as Box<dyn Sliver>])
                .controller(controller.clone())
                .into()
        }
    });
    h.frames(1);
    controller.jump_to(1000.);
    h.frames(2);
}

fn opacity_offstage_visibility(h: &mut FrameHarness) {
    h.mount(|_| {
        let opacity = SliverOpacity::builder()
            .opacity(0.5)
            .sliver(SliverToBoxAdapter::new(row_box("opacity_child", 60., BLUE)))
            .build();
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(opacity),
            Box::new(SliverOffstage::new(
                true,
                SliverToBoxAdapter::new(row_box("offstage_child", 50., ORANGE)),
            )),
            Box::new(SliverToBoxAdapter::new(row_box(
                "visible_after",
                45.,
                GREEN,
            ))),
        ];
        CustomScrollView::new(slivers).into()
    });
    h.frames(2);
}

fn fill_remaining_shrink_after_growth(h: &mut FrameHarness) {
    h.mount(|phase| {
        let adapter_height = if phase == 0 { 45. } else { 200. };
        let slivers: Vec<Box<dyn Sliver>> = vec![
            Box::new(SliverToBoxAdapter::new(row_box(
                "grow_adapter",
                adapter_height,
                BLUE,
            ))),
            Box::new(
                SliverFillRemaining::new(probe("fill_after_growth", solid(GREEN)))
                    .has_scroll_body(false),
            ),
        ];
        CustomScrollView::new(slivers).into()
    });
    h.frames(2);
    h.set_phase(1);
    h.frames(2);
}

fn keep_alive_retains_scrolled_child(h: &mut FrameHarness) {
    let controller = ScrollController::new();
    h.value("offset", {
        let controller = controller.clone();
        move || f64::from(controller.offset())
    });
    h.mount({
        let controller = controller.clone();
        move |_| {
            let list = SliverList::builder(40, |index| {
                KeepAlive::new(
                    index == 2,
                    row_box(
                        &format!("keep{index}"),
                        50.,
                        if index % 2 == 0 { BLUE } else { ORANGE },
                    ),
                )
            });
            CustomScrollView::new([Box::new(list) as Box<dyn Sliver>])
                .controller(controller.clone())
                .into()
        }
    });
    h.frames(1);
    controller.jump_to(1000.);
    h.frames(2);
}
