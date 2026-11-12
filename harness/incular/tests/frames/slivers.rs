use incular_config::EdgeInsets;
use incular_core::{Color, Offset};
use incular_parity_harness::{FrameHarness, Scenario, probe};
use incular_widgets::{
    ColoredBox, CustomScrollView, PinnedHeaderSliver, ScrollController, SizedBox, Sliver,
    SliverCrossAxisGroup, SliverFillRemaining, SliverFillViewport, SliverFixedExtentList,
    SliverGrid, SliverGridDelegate, SliverList, SliverMainAxisGroup, SliverPadding,
    SliverPersistentHeader, SliverPrototypeExtentList, SliverToBoxAdapter, SliverVariedExtentList,
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
