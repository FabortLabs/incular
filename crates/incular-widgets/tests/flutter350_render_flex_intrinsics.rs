use incular_config::{Axis, Constraints, CrossAxisAlignment, TextDirection, VerticalDirection};
use incular_core::Size;
use incular_text::{TextEngine, TextStyle};
use incular_widgets::{AspectRatio, Expanded, Flex, SizedBox, Text, Widget, internal::WidgetTree};

fn square(side: f32) -> Widget {
    SizedBox::new().width(side).height(side).into()
}

fn flex(axis: Axis, children: impl IntoIterator<Item = Widget>) -> Flex {
    Flex::new(axis, children).text_direction(TextDirection::Ltr)
}

#[test]
fn main_intrinsic_extent_combines_largest_flex_fraction_and_inflexible_aspect_ratio() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            flex(
                Axis::Horizontal,
                [
                    square(100.),
                    Expanded::new(square(100.)).into(),
                    AspectRatio::new(1., square(100.)).into(),
                ],
            )
            .into(),
        )
        .unwrap();
    let id = tree.render_id(root).unwrap();
    for (height, width) in [(f32::INFINITY, 300.), (300., 500.), (500., 700.)] {
        assert_eq!(tree.min_intrinsic_width(id, height), Some(width));
        assert_eq!(tree.max_intrinsic_width(id, height), Some(width));
        assert_eq!(tree.element_layout_size(root), None);
    }
    let root_before = tree.children(root).unwrap()[0];
    let bounds = Constraints::loose(Size::new(450., 200.));
    assert_eq!(
        tree.dry_layout_size(id, bounds),
        Some(Size::new(450., 200.))
    );
    assert_eq!(tree.element_layout_size(root_before), None);
    tree.layout(bounds).unwrap();
    assert_eq!(tree.element_layout_size(root), Some(Size::new(450., 200.)));
}

fn check_spacing(axis: Axis) {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            flex(axis, [square(100.), square(100.), square(100.)])
                .spacing(16.)
                .into(),
        )
        .unwrap();
    let id = tree.render_id(root).unwrap();
    let expected = if axis.is_horizontal() {
        Size::new(332., 100.)
    } else {
        Size::new(100., 332.)
    };
    for limit in [f32::INFINITY, 300., 500.] {
        assert_eq!(tree.min_intrinsic_width(id, limit), Some(expected.width));
        assert_eq!(tree.max_intrinsic_width(id, limit), Some(expected.width));
        assert_eq!(tree.min_intrinsic_height(id, limit), Some(expected.height));
        assert_eq!(tree.max_intrinsic_height(id, limit), Some(expected.height));
        assert_eq!(tree.element_layout_size(root), None);
    }
}

#[test]
fn horizontal_flex_spacing_only_contributes_to_main_intrinsics() {
    check_spacing(Axis::Horizontal);
}

#[test]
fn vertical_flex_spacing_only_contributes_to_main_intrinsics() {
    check_spacing(Axis::Vertical);
}

const FONT: &str = "Pinned Flutter Test Flex";

fn flutter_font_tree() -> Option<WidgetTree> {
    let font = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH")?;
    let mut engine = TextEngine::with_system_fonts(false);
    assert!(engine.register_font_bytes_as(FONT, std::fs::read(font).unwrap()));
    Some(WidgetTree::with_text_engine(engine))
}

fn letter_text(content: &str, size: f32) -> Text {
    Text::new(content)
        .style(TextStyle::new().font_family(FONT).font_size(size))
        .text_direction(TextDirection::Ltr)
}

fn check_flow(factors: [u32; 2], cases: &[(f32, f32)]) {
    let Some(mut tree) = flutter_font_tree() else {
        return;
    };
    let text = || letter_text("A\u{200b}A\u{200b}A", 10.);
    let root = tree
        .mount(
            flex(
                Axis::Horizontal,
                [
                    square(5.),
                    Expanded::new(text()).flex(factors[0]).into(),
                    Expanded::new(text()).flex(factors[1]).into(),
                ],
            )
            .into(),
        )
        .unwrap();
    let id = tree.render_id(root).unwrap();
    assert_eq!(tree.max_intrinsic_width(id, f32::INFINITY), Some(95.));
    for &(width, height) in cases {
        assert_eq!(
            tree.min_intrinsic_height(id, width),
            Some(height),
            "min at {width}"
        );
        assert_eq!(
            tree.max_intrinsic_height(id, width),
            Some(height),
            "max at {width}"
        );
        assert_eq!(tree.element_layout_size(root), None);
    }
}

#[test]
fn flex_cross_intrinsic_height_tracks_reflow_with_ascending_factors() {
    check_flow(
        [1, 2],
        &[
            (f32::INFINITY, 10.),
            (95., 10.),
            (94., 20.),
            (65., 20.),
            (35., 30.),
        ],
    );
}

#[test]
fn flex_cross_intrinsic_height_tracks_reflow_with_descending_factors() {
    check_flow(
        [2, 1],
        &[(f32::INFINITY, 10.), (95., 10.), (65., 20.), (35., 30.)],
    );
}

#[test]
fn dry_baseline_uses_actual_offsets_and_does_not_change_live_geometry() {
    for (axis, cross, reverse) in [
        (
            Axis::Horizontal,
            CrossAxisAlignment::Baseline,
            VerticalDirection::Down,
        ),
        (
            Axis::Horizontal,
            CrossAxisAlignment::End,
            VerticalDirection::Up,
        ),
        (
            Axis::Vertical,
            CrossAxisAlignment::Center,
            VerticalDirection::Up,
        ),
    ] {
        let mut tree = WidgetTree::new();
        let children = [
            Widget::from(Text::new("A").text_direction(TextDirection::Ltr)),
            square(34.),
            Widget::from(
                Text::new("B")
                    .text_direction(TextDirection::Ltr)
                    .style(TextStyle::new().font_size(32.)),
            ),
        ];
        let root = tree
            .mount(
                flex(axis, children)
                    .cross_axis_alignment(cross)
                    .vertical_direction(reverse)
                    .spacing(4.)
                    .into(),
            )
            .unwrap();
        let id = tree.render_id(root).unwrap();
        let bounds = Constraints::loose(Size::new(240., 140.));
        let size = tree.dry_layout_size(id, bounds);
        let baseline = tree.dry_alphabetic_baseline(id, bounds);
        assert!(size.is_some() && baseline.is_some());
        assert_eq!(tree.element_layout_size(root), None);
        assert_eq!(tree.baseline(id), None);
        tree.layout(bounds).unwrap();
        assert_eq!(tree.element_layout_size(root), size);
        assert_eq!(tree.baseline(id), baseline);
        let _ = tree.dry_layout_size(id, Constraints::loose(Size::new(115., 80.)));
        assert_eq!(tree.element_layout_size(root), size);
        assert_eq!(tree.baseline(id), baseline);
    }
}

#[derive(Clone, Copy)]
enum GridBaseline {
    Top,
    Bottom,
    None,
}

// Use an actual reflowing paragraph with the pinned 10px grid advance.
// A zero-height baseline marker moves before/after the body in a Column
// without changing the measured grid size. Unbaselined grids wrap fixed boxes.
fn grid_child(placement: GridBaseline) -> Widget {
    use incular_config::MainAxisSize;
    use incular_widgets::{Baseline, Wrap};
    let body = letter_text(&["A"; 10].join("\u{200b}"), 10.);
    if matches!(placement, GridBaseline::None) {
        return Wrap::new((0..10).map(|_| square(10.)))
            .text_direction(TextDirection::Ltr)
            .into();
    }
    let marker = SizedBox::new()
        .width(0.)
        .height(0.)
        .child(Baseline::new(0., letter_text("A", 10.)));
    let vertical = match placement {
        GridBaseline::Top => VerticalDirection::Down,
        GridBaseline::Bottom => VerticalDirection::Up,
        GridBaseline::None => unreachable!(),
    };
    flex(Axis::Vertical, [marker.into(), body.into()])
        .main_axis_size(MainAxisSize::Min)
        .vertical_direction(vertical)
        .into()
}

fn baseline_grid_row(first: GridBaseline, second: GridBaseline) -> Widget {
    use incular_widgets::Flexible;
    flex(
        Axis::Horizontal,
        [
            Expanded::new(grid_child(first)).flex(2).into(),
            Flexible::new(grid_child(second)).flex(1).into(),
        ],
    )
    .cross_axis_alignment(CrossAxisAlignment::Baseline)
    .into()
}

#[test]
fn baseline_aligned_flex_dry_size_reserves_greatest_ascent_and_descent() {
    let Some(mut tree) = flutter_font_tree() else {
        return;
    };
    let root = tree
        .mount(baseline_grid_row(GridBaseline::Top, GridBaseline::Bottom))
        .unwrap();
    let render = tree.render_id(root).unwrap();
    for (width, height, baseline) in [(200., 30., 20.), (300., 20., 10.)] {
        let constraints = Constraints::loose(Size::new(width, 100.));
        assert_eq!(
            tree.dry_layout_size(render, constraints),
            Some(Size::new(width, height))
        );
        assert_eq!(
            tree.dry_alphabetic_baseline(render, constraints),
            Some(baseline)
        );
        tree.layout(constraints).unwrap();
        assert_eq!(
            tree.element_layout_size(root),
            Some(Size::new(width, height))
        );
        assert_eq!(tree.baseline(render), Some(baseline));
    }
}

#[test]
fn baseline_aligned_flex_cross_intrinsics_reserve_both_sides_of_baseline() {
    let Some(mut tree) = flutter_font_tree() else {
        return;
    };
    let root = tree
        .mount(baseline_grid_row(GridBaseline::Top, GridBaseline::Bottom))
        .unwrap();
    let render = tree.render_id(root).unwrap();
    assert_eq!(tree.max_intrinsic_height(render, 200.), Some(30.));
    assert_eq!(tree.min_intrinsic_height(render, 200.), Some(30.));
    assert_eq!(tree.max_intrinsic_height(render, 300.), Some(20.));
    assert_eq!(tree.min_intrinsic_height(render, 300.), Some(20.));
    assert_eq!(tree.element_layout_size(root), None);
}

#[test]
fn unbaselined_flex_child_reserves_height_but_does_not_move_the_baseline() {
    let Some(mut tree) = flutter_font_tree() else {
        return;
    };
    let root = tree
        .mount(baseline_grid_row(GridBaseline::Bottom, GridBaseline::None))
        .unwrap();
    let render = tree.render_id(root).unwrap();
    for (width, height, baseline) in [(200., 20., 10.), (300., 10., 10.)] {
        let constraints = Constraints::loose(Size::new(width, 100.));
        assert_eq!(
            tree.dry_layout_size(render, constraints),
            Some(Size::new(width, height))
        );
        assert_eq!(
            tree.dry_alphabetic_baseline(render, constraints),
            Some(baseline)
        );
        tree.layout(constraints).unwrap();
        assert_eq!(
            tree.element_layout_size(root),
            Some(Size::new(width, height))
        );
        assert_eq!(tree.baseline(render), Some(baseline));
    }
}
