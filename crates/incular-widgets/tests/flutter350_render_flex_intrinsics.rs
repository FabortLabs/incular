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
