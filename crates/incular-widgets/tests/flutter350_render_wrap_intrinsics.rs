//! Flutter 3.50.0 rendering/wrap_test.dart: fixed intrinsic geometry and clips.
use incular_config::{Axis, Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_widgets::{ConstrainedBox, Widget, Wrap, internal::WidgetTree};

fn wrap(axis: Axis, count: usize) -> Wrap {
    Wrap::new(
        (0..count).map(|_| {
            ConstrainedBox::empty(Constraints::new(80., f32::INFINITY, 80., f32::INFINITY))
        }),
    )
    .direction(axis)
    .spacing(5.)
    .run_spacing(5.)
    .text_direction(TextDirection::Ltr)
}

fn verify_cross_intrinsics(axis: Axis, cases: &[(f32, f32)], count: usize) {
    let mut tree = WidgetTree::new();
    let root = tree.mount(wrap(axis, count).into()).unwrap();
    let render = tree.render_id(root).unwrap();
    for &(limit, expected) in cases {
        let (min, max, bounds) = if axis.is_horizontal() {
            (
                tree.min_intrinsic_height(render, limit),
                tree.max_intrinsic_height(render, limit),
                Constraints::at_most(limit, f32::INFINITY),
            )
        } else {
            (
                tree.min_intrinsic_width(render, limit),
                tree.max_intrinsic_width(render, limit),
                Constraints::at_most(f32::INFINITY, limit),
            )
        };
        assert_eq!(min, Some(expected), "minimum at {limit}");
        assert_eq!(max, Some(expected), "maximum at {limit}");
        assert_eq!(
            tree.dry_layout_size(render, bounds)
                .map(|size| axis.cross_extent(size)),
            Some(expected),
            "dry at {limit}",
        );
        assert_eq!(tree.element_layout_size(root), None, "dry stays unmeasured");
    }
}

#[test]
fn horizontal_and_vertical_intrinsics_count_run_spacing_once() {
    let cases = [
        (245., 165.),
        (250., 80.),
        (80., 250.),
        (79., 250.),
        (f32::INFINITY, 80.),
    ];
    for axis in [Axis::Horizontal, Axis::Vertical] {
        verify_cross_intrinsics(axis, &cases, 3);
    }
}

#[test]
fn a_single_run_has_no_trailing_run_spacing() {
    let cases = [(100., 80.), (79., 80.), (80., 80.)];
    for axis in [Axis::Horizontal, Axis::Vertical] {
        verify_cross_intrinsics(axis, &cases, 1);
    }
}

#[test]
fn parallel_intrinsics_take_child_maximum_or_sum_without_spacing() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut tree = WidgetTree::new();
        let root = tree.mount(wrap(axis, 3).into()).unwrap();
        let render = tree.render_id(root).unwrap();
        let (min, max) = if axis.is_horizontal() {
            (
                tree.min_intrinsic_width(render, 0.),
                tree.max_intrinsic_width(render, 0.),
            )
        } else {
            (
                tree.min_intrinsic_height(render, 0.),
                tree.max_intrinsic_height(render, 0.),
            )
        };
        assert_eq!((min, max), (Some(80.), Some(240.)));
    }
}

#[test]
fn an_empty_constrained_child_uses_enforced_minimums_in_dry_and_live_layout() {
    let mut tree = WidgetTree::new();
    let widget = ConstrainedBox::empty(Constraints::new(80., 160., 80., 160.));
    let root = tree.mount(widget.into()).unwrap();
    assert!(tree.children(root).unwrap().is_empty());
    let render = tree.render_id(root).unwrap();
    let bounds = Constraints::at_most(79., 180.);
    let expected = Size::new(79., 80.);
    assert_eq!(tree.dry_layout_size(render, bounds), Some(expected));
    assert_eq!(tree.element_layout_size(root), None);
    tree.layout(bounds).unwrap();
    assert_eq!(tree.element_layout_size(root), Some(expected));
}

#[test]
fn a_cross_axis_overflow_uses_only_the_explicit_clip_mode() {
    for (mode, expected) in [
        (None, None),
        (Some(Clip::None), None),
        (Some(Clip::HardEdge), Some(ClipCoverage::HardEdge)),
        (Some(Clip::AntiAlias), Some(ClipCoverage::AntiAlias)),
        (
            Some(Clip::AntiAliasWithSaveLayer),
            Some(ClipCoverage::AntiAliasWithSaveLayer),
        ),
    ] {
        let child = Widget::box_(Size::new(200., 200.), Color::BLACK).with_key(1_u64);
        let wrap = Wrap::new([child]).text_direction(TextDirection::Ltr);
        let scene: Widget = match mode {
            Some(mode) => wrap.clip_behavior(mode).into(),
            None => wrap.into(),
        };
        let mut tree = WidgetTree::new();
        let root = tree.mount(scene).unwrap();
        tree.layout(Constraints::at_most(100., 100.)).unwrap();
        assert_eq!(tree.element_layout_size(root), Some(Size::new(100., 100.)));
        let list = tree.paint();
        let clips: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushClip { rect } => Some((*rect, ClipCoverage::HardEdge)),
                PaintCommand::PushClipRRect {
                    rrect, coverage, ..
                } => Some((rrect.rect, *coverage)),
                _ => None,
            })
            .collect();
        let rect = Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.));
        assert_eq!(
            clips,
            expected
                .into_iter()
                .map(|coverage| (rect, coverage))
                .collect::<Vec<_>>()
        );
    }
}

fn verify_wrapping_text(axis: Axis) {
    use incular_text::{TextEngine, TextStyle};
    use incular_widgets::{RotatedBox, Text};

    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FONT: &str = "Pinned Flutter Test for dry Wrap";
    let mut engine = TextEngine::with_system_fonts(false);
    assert!(engine.register_font_bytes_as(FONT, std::fs::read(font).unwrap()));
    let text = Text::new("A very very very very very very very very long text")
        .style(TextStyle::new().font_family(FONT).font_size(15.))
        .text_direction(TextDirection::Ltr);
    let child: Widget = match axis {
        Axis::Horizontal => text.into(),
        Axis::Vertical => RotatedBox::new(1, text).into(),
    };
    let mut tree = WidgetTree::with_text_engine(engine);
    let scene = Wrap::new([child])
        .direction(axis)
        .text_direction(TextDirection::Ltr);
    let root = tree.mount(scene.into()).unwrap();
    let render = tree.render_id(root).unwrap();
    let child = tree.children(root).unwrap()[0];
    for (limit, expect) in [(f32::INFINITY, 15.), (600., 30.), (300., 45.)] {
        let measured = if axis.is_horizontal() {
            tree.max_intrinsic_height(render, limit)
        } else {
            tree.max_intrinsic_width(render, limit)
        };
        assert_eq!(measured, Some(expect), "{axis:?} at {limit}");
        assert_eq!(tree.element_layout_size(root), None);
        assert_eq!(tree.element_layout_size(child), None);
    }
    let constraints = if axis.is_horizontal() {
        Constraints::at_most(300., f32::INFINITY)
    } else {
        Constraints::at_most(f32::INFINITY, 300.)
    };
    tree.layout(constraints).unwrap();
    let previous = tree.element_layout_size(root);
    let wide = Constraints::loose(Size::new(600., 600.));
    let queried = tree.dry_layout_size(render, wide).unwrap();
    assert_eq!(axis.cross_extent(queried), 30.);
    assert_eq!(tree.element_layout_size(root), previous);
    assert_eq!(previous.map(|size| axis.cross_extent(size)), Some(45.));
}

#[test]
fn horizontal_wrap_dry_height_reflows_paragraph_at_the_supplied_width() {
    verify_wrapping_text(Axis::Horizontal);
}

#[test]
fn vertical_wrap_dry_width_reflows_a_quarter_turned_paragraph() {
    verify_wrapping_text(Axis::Vertical);
}

#[test]
fn flutter_test_font_wrap_propagates_exact_ascent_under_the_baseline_parent() {
    use incular_text::{TextEngine, TextStyle};
    use incular_widgets::{Baseline, Center, Text};
    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FONT: &str = "Pinned Flutter Test for Wrap baselines";
    let mut engine = TextEngine::with_system_fonts(false);
    assert!(engine.register_font_bytes_as(FONT, std::fs::read(font).unwrap()));
    let caption: Widget = Text::new("X")
        .text_direction(TextDirection::Ltr)
        .style(TextStyle::new().font_family(FONT).font_size(100.))
        .into();
    let wrap =
        Widget::from(Wrap::new([caption]).text_direction(TextDirection::Ltr)).with_key(101_u64);
    let baseline = Widget::from(Baseline::new(175., wrap)).with_key(102_u64);
    let mut tree = WidgetTree::with_text_engine(engine);
    tree.mount(Center::new(baseline).into()).unwrap();
    let wrap = tree.element_with_key(&101_u64.into()).unwrap();
    let id = tree.render_id(wrap).unwrap();
    let viewport = Size::new(800., 600.);
    assert_eq!(
        tree.dry_alphabetic_baseline(id, Constraints::loose(viewport)),
        Some(75.)
    );
    tree.layout(Constraints::tight(viewport)).unwrap();
    assert_eq!(tree.element_layout_size(wrap), Some(Size::new(100., 100.)));
    assert_eq!(
        tree.element_layout_size(tree.element_with_key(&102_u64.into()).unwrap()),
        Some(Size::new(100., 200.))
    );
}

#[test]
fn dry_wrap_baseline_respects_positioned_run_order_without_measuring_live_geometry() {
    use incular_config::VerticalDirection;
    use incular_text::{TextEngine, TextStyle};
    use incular_widgets::Text;
    let Some(font) = std::env::var_os("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    const FONT: &str = "Pinned Flutter Test for dry Wrap runs";
    for (direction, expected) in [(VerticalDirection::Down, 30.), (VerticalDirection::Up, 15.)] {
        let mut engine = TextEngine::with_system_fonts(false);
        assert!(engine.register_font_bytes_as(FONT, std::fs::read(&font).unwrap()));
        let caption = |size| {
            Text::new("X")
                .text_direction(TextDirection::Ltr)
                .style(TextStyle::new().font_family(FONT).font_size(size))
        };
        let wrap = Wrap::new([caption(40.), caption(20.)])
            .text_direction(TextDirection::Ltr)
            .vertical_direction(direction)
            .run_spacing(7.);
        let mut tree = WidgetTree::with_text_engine(engine);
        let root = tree.mount(wrap.into()).unwrap();
        let id = tree.render_id(root).unwrap();
        let bounds = Constraints::at_most(50., 100.);
        assert_eq!(tree.dry_layout_size(id, bounds), Some(Size::new(40., 67.)));
        assert_eq!(tree.dry_alphabetic_baseline(id, bounds), Some(expected));
        assert_eq!(tree.element_layout_size(root), None);
        tree.layout(Constraints::at_most(50., 101.)).unwrap();
        assert_eq!(tree.baseline(id), Some(expected));
        assert_eq!(tree.dry_alphabetic_baseline(id, bounds), Some(expected));
    }
}
