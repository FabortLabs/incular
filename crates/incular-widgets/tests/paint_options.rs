//! Paint options that used to be accepted but not painted (paint-016), the
//! continuous and linear shape borders (paint-017), image nine-slice and
//! mirroring (paint-013), and general `ImageFiltered` filters (paint-007).
//! Expected values follow Flutter 3.41's `painting/decoration_image.dart`
//! (`paintImage`), `painting/box_shadow.dart` (`blurStyle`),
//! `painting/continuous_rectangle_border.dart`, `painting/linear_border.dart`
//! and `widgets/image_filter.dart`.

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size, Transform};
use incular_image::ImageHandle;
use incular_rendering::{
    BlendMode, Canvas, ColorFilter, ImageFilter, ImageFilterTarget, ImageSampling, PaintCommand,
};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BlurStyle, BorderRadius, BorderSide, BoxDecoration, BoxFit, BoxShadow, Container,
    ContinuousRectangleBorder, DecorationImage, Directionality, Image, ImageFiltered, ImageRepeat,
    LinearBorder, LinearBorderEdge, Positioned, RawImage, ShapeBorder, SizedBox, Stack, Widget,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn handle(width: u32, height: u32) -> ImageHandle {
    ImageHandle::from_rgba8(width, height, vec![255u8; (width * height * 4) as usize])
        .expect("handle")
}

fn painted(root: Widget, size: Size) -> Vec<PaintCommand> {
    let mut tree = WidgetTree::new();
    tree.mount(root).expect("mount");
    for _ in 0..2 {
        tree.layout(Constraints::tight(size)).expect("layout");
        tree.update_compositor(incular_core::time::Instant::now())
            .expect("composite");
    }
    tree.paint().commands().to_vec()
}

fn placed(child: impl Into<Widget>, width: f32, height: f32) -> Widget {
    Stack::new([Widget::from(
        Positioned::new(child)
            .left(0.)
            .top(0.)
            .width(width)
            .height(height),
    )])
    .into()
}

fn images(commands: &[PaintCommand]) -> Vec<(Rect, Rect)> {
    commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Image {
                source,
                destination,
                ..
            } => Some((*source, *destination)),
            _ => None,
        })
        .collect()
}

fn assert_rect(actual: Rect, expected: Rect) {
    for (a, e) in [
        (actual.origin.x, expected.origin.x),
        (actual.origin.y, expected.origin.y),
        (actual.size.width, expected.size.width),
        (actual.size.height, expected.size.height),
    ] {
        assert!((a - e).abs() < 1e-3, "{actual:?} != {expected:?}");
    }
}

/// `paintImage` with a center slice: `drawImageNine` keeps the 10 px
/// corners of a 30x30 image unscaled in a 100x60 box and stretches the rest.
#[test]
fn decoration_image_center_slice_paints_nine_patches() {
    let mut image = DecorationImage::new(handle(30, 30));
    image.center_slice = Some(rect(10., 10., 10., 10.));
    let decoration = BoxDecoration::new().image(image);
    let commands = painted(
        placed(Container::new().decoration(decoration), 100., 60.),
        Size::new(200., 200.),
    );
    let patches = images(&commands);
    assert_eq!(patches.len(), 9, "{commands:#?}");
    // Top-left corner, center and bottom-right corner.
    assert_rect(patches[0].0, rect(0., 0., 10., 10.));
    assert_rect(patches[0].1, rect(0., 0., 10., 10.));
    assert_rect(patches[4].0, rect(10., 10., 10., 10.));
    assert_rect(patches[4].1, rect(10., 10., 80., 40.));
    assert_rect(patches[8].1, rect(90., 50., 10., 10.));
}

/// `DecorationImage.matchTextDirection` under a right-to-left
/// `Directionality`: the image mirrors about the box center, with the
/// alignment mirrored too, so a top-left aligned image lands top-right.
#[test]
fn decoration_image_mirrors_under_rtl_when_it_matches_text_direction() {
    let mut image = DecorationImage::new(handle(20, 20));
    image.match_text_direction = true;
    image.alignment = incular_config::Alignment::TOP_LEFT;
    let decoration = BoxDecoration::new().image(image);
    let commands = painted(
        Directionality::new(
            TextDirection::Rtl,
            placed(Container::new().decoration(decoration), 100., 60.),
        )
        .into(),
        Size::new(200., 200.),
    );
    let flip = commands.iter().find_map(|command| match command {
        PaintCommand::PushTransform { transform } if transform.coefficients()[0] < 0.0 => {
            Some(*transform)
        }
        _ => None,
    });
    let flip = flip.expect("a mirroring transform");
    // The mirror maps x to 100 - x about the box center at x = 50.
    let mapped = flip.transform_point(Offset::new(10., 0.));
    assert!((mapped.x - 90.).abs() < 1e-3, "{mapped:?}");
    // The alignment was negated before the flip: the destination is
    // right-aligned in local space and mirrors back to the left there.
    let [(_, destination)] = images(&commands)[..] else {
        panic!("one image: {commands:#?}");
    };
    assert_rect(destination, rect(80., 0., 20., 20.));
}

/// `backgroundBlendMode` blends the background fill alone.
#[test]
fn background_blend_mode_blends_the_fill() {
    let decoration = BoxDecoration::new()
        .color(Color::rgba(255, 0, 0, 255))
        .background_blend_mode(BlendMode::Multiply);
    let commands = painted(
        placed(Container::new().decoration(decoration), 50., 50.),
        Size::new(100., 100.),
    );
    assert!(
        commands.iter().any(|command| matches!(
            command,
            PaintCommand::PushBlend {
                mode: BlendMode::Multiply,
                ..
            }
        )),
        "{commands:#?}"
    );
}

fn shadow_commands(style: BlurStyle) -> Vec<PaintCommand> {
    let shadow = BoxShadow {
        blur_style: style,
        ..BoxShadow::new(Color::rgba(0, 0, 0, 128), Offset::ZERO, 8., 0.)
    };
    painted(
        placed(
            Container::new().decoration(BoxDecoration::new().box_shadow([shadow])),
            40.,
            40.,
        ),
        Size::new(100., 100.),
    )
}

/// `BlurStyle.outer` / `inner` combine the blur with the shape mask
/// (`SkBlurMask`: outer `b * (1 - s)`, inner `b * s`), and the shadow
/// color's alpha applies once to the combined mask.
#[test]
fn blur_styles_combine_the_blur_with_the_shape() {
    for (style, mode) in [
        (BlurStyle::Outer, BlendMode::DstOut),
        (BlurStyle::Inner, BlendMode::DstIn),
    ] {
        let commands = shadow_commands(style);
        assert!(
            commands.iter().any(
                |command| matches!(command, PaintCommand::PushBlend { mode: m, .. } if *m == mode)
            ),
            "{style:?}: {commands:#?}"
        );
        assert!(
            commands.iter().any(|command| matches!(
                command,
                PaintCommand::PushOpacity { alpha, .. } if (*alpha - 128. / 255.).abs() < 1e-3
            )),
            "{style:?}: {commands:#?}"
        );
    }
    // Solid: the shape over the blur, no blend.
    let solid = shadow_commands(BlurStyle::Solid);
    assert!(
        solid
            .iter()
            .any(|command| matches!(command, PaintCommand::PushBlur { .. }))
    );
    assert!(
        !solid
            .iter()
            .any(|command| matches!(command, PaintCommand::PushBlend { .. }))
    );
    // Normal: no opacity group, as before.
    let normal = shadow_commands(BlurStyle::Normal);
    assert!(
        !normal
            .iter()
            .any(|command| matches!(command, PaintCommand::PushOpacity { .. }))
    );
}

/// `Image.centerSlice` keeps the image's own layout size and nine-slices
/// the paint.
#[test]
fn image_center_slice_keeps_layout_and_nine_slices() {
    let image = Image::new(handle(30, 30))
        .width(90.)
        .height(30.)
        .center_slice(rect(10., 10., 10., 10.));
    let commands = painted(
        Stack::new([Widget::from(image)]).into(),
        Size::new(200., 200.),
    );
    let patches = images(&commands);
    assert_eq!(patches.len(), 9, "{commands:#?}");
    assert_rect(patches[4].1, rect(10., 10., 70., 10.));
}

/// `RawImage.colorBlendMode`: the color blends through `ColorFilter.mode`.
#[test]
fn raw_image_color_blend_mode_uses_the_mode_filter() {
    let color = Color::rgba(0, 0, 255, 255);
    let commands = painted(
        Stack::new([Widget::from(
            RawImage::new()
                .image(handle(10, 10))
                .color(color)
                .color_blend_mode(BlendMode::Multiply),
        )])
        .into(),
        Size::new(100., 100.),
    );
    let expected = ColorFilter::blend(color, BlendMode::Multiply);
    assert!(
        commands.iter().any(|command| matches!(
            command,
            PaintCommand::PushColorFilter { filter, .. } if *filter == expected
        )),
        "{commands:#?}"
    );
}

/// `ContinuousRectangleBorder._getPath`: radii clamp to the shortest side,
/// and the inner path deflates by the side width.
#[test]
fn continuous_rectangle_border_paths() {
    let border = ContinuousRectangleBorder::new(
        BorderSide::solid(Color::BLACK, 2.),
        BorderRadius::circular(500.),
    );
    let area = rect(0., 0., 100., 60.);
    let outer = border.outer_path(area);
    let bounds = outer.bounds().expect("bounds");
    assert_rect(bounds, area);
    assert!(outer.contains(Offset::new(50., 30.), incular_rendering::FillRule::NonZero));
    let inner = border.inner_path(area).bounds().expect("bounds");
    assert_rect(inner, rect(2., 2., 96., 56.));
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, area);
    assert!(matches!(
        canvas.finish().commands(),
        [PaintCommand::StrokePath { .. }]
    ));
}

/// `LinearBorder.paint`: a bottom edge of half the width, aligned to the
/// end, fills a `side.width` tall strip; dimensions cover present edges.
#[test]
fn linear_border_paints_only_its_edges() {
    let border = LinearBorder::bottom(
        BorderSide::solid(Color::BLACK, 4.),
        LinearBorderEdge::new(0.5, 1.0),
    );
    let area = rect(0., 0., 100., 40.);
    let mut canvas = Canvas::default();
    border.paint(&mut canvas, area);
    let list = canvas.finish();
    let [PaintCommand::FillPath { path, .. }] = list.commands() else {
        panic!("one edge: {:?}", list.commands());
    };
    assert_rect(path.bounds().expect("bounds"), rect(50., 36., 50., 4.));
    assert_rect(
        border.inner_path(area).bounds().expect("bounds"),
        rect(0., 0., 100., 36.),
    );
    let mut empty = Canvas::default();
    LinearBorder::NONE.paint(&mut empty, area);
    assert!(empty.finish().commands().is_empty());
}

/// `ImageFiltered` with a composed filter runs each stage in order (inner
/// first), and refuses dilate and erode instead of painting unfiltered.
#[test]
fn image_filtered_executes_composed_filters_and_refuses_morphology() {
    let filter = ImageFilter::compose(
        ImageFilter::color(ColorFilter::linear_to_srgb_gamma()),
        ImageFilter::blur(3., 3.),
    );
    let child = SizedBox::new()
        .width(20.)
        .height(20.)
        .child(Container::new().color(Color::WHITE));
    let filtered = ImageFiltered::new(filter, child).expect("supported");
    let commands = painted(
        Stack::new([Widget::from(filtered)]).into(),
        Size::new(100., 100.),
    );
    let blur = commands
        .iter()
        .position(|command| matches!(command, PaintCommand::PushBlur { .. }))
        .expect("blur stage");
    let color = commands
        .iter()
        .position(|command| matches!(command, PaintCommand::PushColorFilter { .. }))
        .expect("color stage");
    // The outer color filter wraps the inner blur.
    assert!(color < blur, "{commands:#?}");

    let matrix = ImageFiltered::new(
        ImageFilter::matrix(Transform::scale(2.)),
        Widget::box_(Size::new(10., 10.), Color::WHITE),
    )
    .expect("matrix is supported");
    let unfiltered = painted(
        Stack::new([Widget::from(matrix.clone().enabled(false))]).into(),
        Size::new(100., 100.),
    );
    assert!(
        !unfiltered
            .iter()
            .any(|command| matches!(command, PaintCommand::PushMatrixFilter { .. }))
    );
    let commands = painted(
        Stack::new([Widget::from(matrix)]).into(),
        Size::new(100., 100.),
    );
    let (matrix, sampling, source, output) = commands
        .iter()
        .find_map(|command| match command {
            PaintCommand::PushMatrixFilter {
                matrix,
                sampling,
                bounds,
                filtered_bounds,
                ..
            } => Some((*matrix, *sampling, *bounds, *filtered_bounds)),
            _ => None,
        })
        .expect("matrix resamples the child raster");
    assert_eq!(matrix, Transform::scale(2.));
    assert_eq!(sampling, ImageSampling::Linear);
    assert_rect(source, rect(0., 0., 10., 10.));
    assert_rect(output, rect(0., 0., 20., 20.));

    let refused = ImageFiltered::new(
        ImageFilter::compose(ImageFilter::blur(1., 1.), ImageFilter::dilate(2., 2.)),
        SizedBox::new(),
    )
    .expect_err("dilate has no subtree stage");
    assert_eq!(refused.target, ImageFilterTarget::Subtree);
    assert_eq!(refused.filter, ImageFilter::dilate(2., 2.));
}

/// `BackdropFilter` executes blur, matrix, colour and composed filters and
/// refuses morphology.
#[test]
fn backdrop_filter_accepts_only_executable_filters() {
    // paint-001 (blockers-render-input): blur, matrix, colour and composed
    // backdrop filters execute; only dilation and erosion are refused.
    use incular_widgets::BackdropFilter;
    assert!(BackdropFilter::image_filter(ImageFilter::blur(2., 2.), SizedBox::new()).is_ok());
    assert!(
        BackdropFilter::image_filter(ImageFilter::matrix(Transform::scale(2.)), SizedBox::new())
            .is_ok()
    );
    assert!(
        BackdropFilter::image_filter(ImageFilter::color(ColorFilter::invert(1.)), SizedBox::new())
            .is_ok()
    );
    let refused = BackdropFilter::image_filter(
        ImageFilter::compose(ImageFilter::dilate(1., 1.), ImageFilter::blur(2., 2.)),
        SizedBox::new(),
    )
    .expect_err("no backdrop morphology stage");
    assert_eq!(refused.target, ImageFilterTarget::Backdrop);
    assert_eq!(refused.filter, ImageFilter::dilate(1., 1.));

    // A composition reaches the backdrop stage whole.
    let composed = ImageFilter::compose(
        ImageFilter::color(ColorFilter::invert(1.)),
        ImageFilter::blur(3., 3.),
    );
    let filter =
        BackdropFilter::image_filter(composed.clone(), SizedBox::new().width(40.).height(30.))
            .expect("executable");
    let mut tree = WidgetTree::new();
    tree.mount(Widget::from(filter)).expect("mount");
    tree.layout(Constraints::tight(Size::new(40., 30.)))
        .expect("layout");
    let list = tree.paint();
    let filters: Vec<_> = list
        .commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushBackdropFilter { image_filter, .. } => {
                Some(image_filter.as_deref().cloned())
            }
            _ => None,
        })
        .collect();
    assert_eq!(filters, [Some(composed)]);
}

#[test]
fn flutter350_color_filtered_reuses_layer_when_the_color_changes() {
    use incular_widgets::{ColorFiltered, Placeholder};

    let filter = |color| ColorFilter::blend(color, BlendMode::Color);
    let widget = |color| ColorFiltered::new(filter(color), Placeholder::new()).into();
    let draw = |tree: &mut WidgetTree| {
        tree.layout(Constraints::tight(Size::new(40., 20.)))
            .unwrap();
        tree.update_compositor(incular_core::time::Instant::now())
            .unwrap();
        tree.paint()
            .commands()
            .iter()
            .find_map(|cmd| match cmd {
                PaintCommand::PushColorFilter { layer, filter, .. } => Some((*layer, *filter)),
                _ => None,
            })
            .expect("non-identity color filter layer")
    };
    let red = Color::rgba(255, 0, 0, 255);
    let green = Color::rgba(0, 255, 0, 255);
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget(red)).unwrap();
    let (before, initial) = draw(&mut tree);
    assert_eq!(initial, filter(red));
    tree.update(root, widget(green)).unwrap();
    let (after, updated) = draw(&mut tree);
    assert_eq!(updated, filter(green));
    assert_eq!(after, before, "repaint keeps the same color filter layer");
}

#[test]
fn flutter350_color_filtered_at_zero_area_keeps_zero_layout_size() {
    use incular_widgets::ColorFiltered;

    let filter = ColorFilter::blend(Color::rgba(0, 0, 255, 255), BlendMode::Color);
    let mut tree = WidgetTree::new();
    tree.mount(ColorFiltered::without_child(filter).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    assert_eq!(tree.root_layout_size(), Some(Size::ZERO));
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    let _ = tree.paint();
}

#[test]
fn childless_color_filter_uses_minimum_constraints_without_a_matte() {
    use incular_widgets::ColorFiltered;

    let filter = ColorFilter::blend(Color::WHITE, BlendMode::Src);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(ColorFiltered::without_child(filter).into())
        .unwrap();
    let place = |tree: &mut WidgetTree, constraints| {
        tree.layout(constraints).unwrap();
        tree.update_compositor(incular_core::time::Instant::now())
            .unwrap();
        let list = tree.paint();
        (
            tree.root_layout_size(),
            list.commands()
                .iter()
                .any(|command| matches!(command, PaintCommand::Rect { .. })),
        )
    };
    let loose = Constraints::new(0., 40., 0., 20.);
    assert_eq!(place(&mut tree, loose), (Some(Size::ZERO), false));
    let size = Size::new(40., 20.);
    let tight = Constraints::tight(size);
    assert_eq!(place(&mut tree, tight), (Some(size), false));

    tree.update(
        root,
        ColorFiltered::new(filter, Widget::box_(Size::new(12., 8.), Color::BLACK)).into(),
    )
    .unwrap();
    assert_eq!(place(&mut tree, loose), (Some(Size::new(12., 8.)), true));

    tree.update(root, ColorFiltered::without_child(filter).into())
        .unwrap();
    assert_eq!(place(&mut tree, loose), (Some(Size::ZERO), false));
}

fn pushes_filter(commands: &[PaintCommand], expected: ColorFilter) -> bool {
    commands.iter().any(|command| {
        matches!(
            command,
            PaintCommand::PushColorFilter { filter, .. } if *filter == expected
        )
    })
}

/// `Image.color` and `Image.colorBlendMode` (paint-014): `srcIn` by
/// default, or the given mode, as `RawImage` paints them.
#[test]
fn image_color_tints_through_the_blend_mode() {
    let color = Color::rgba(0, 0, 255, 255);
    let tinted = painted(
        Stack::new([Widget::from(Image::new(handle(10, 10)).color(color))]).into(),
        Size::new(100., 100.),
    );
    assert!(
        pushes_filter(&tinted, ColorFilter::tint(color)),
        "{tinted:#?}"
    );
    let blended = painted(
        Stack::new([Widget::from(
            Image::new(handle(10, 10))
                .color(color)
                .color_blend_mode(BlendMode::Multiply),
        )])
        .into(),
        Size::new(100., 100.),
    );
    assert!(
        pushes_filter(&blended, ColorFilter::blend(color, BlendMode::Multiply)),
        "{blended:#?}"
    );
}

/// `FadeInImage.color`/`colorBlendMode` and `placeholderColor`/
/// `placeholderColorBlendMode` tint each image separately (paint-014).
#[test]
fn fade_in_image_tints_the_placeholder_and_the_image_separately() {
    let red = Color::rgba(255, 0, 0, 255);
    let blue = Color::rgba(0, 0, 255, 255);
    let placeholder = painted(
        Stack::new([Widget::from(
            incular_widgets::FadeInImage::new(handle(10, 10), None)
                .placeholder_color(red)
                .placeholder_color_blend_mode(BlendMode::Modulate)
                .image_color(blue),
        )])
        .into(),
        Size::new(100., 100.),
    );
    assert!(pushes_filter(
        &placeholder,
        ColorFilter::blend(red, BlendMode::Modulate)
    ));
    assert!(!pushes_filter(&placeholder, ColorFilter::tint(blue)));
    // A synchronously present target shows alone, with its own tint.
    let target = painted(
        Stack::new([Widget::from(
            incular_widgets::FadeInImage::new(handle(10, 10), Some(handle(10, 10)))
                .placeholder_color(red)
                .image_color(blue),
        )])
        .into(),
        Size::new(100., 100.),
    );
    assert!(pushes_filter(&target, ColorFilter::tint(blue)));
    assert!(!pushes_filter(&target, ColorFilter::tint(red)));
}

fn raw_image_render_blend(kind: Option<&incular_widgets::internal::RenderKind>) -> BlendMode {
    let Some(incular_widgets::internal::RenderKind::Image {
        blend_mode: mode, ..
    }) = kind
    else {
        panic!("expected a retained destination blend")
    };
    *mode
}

fn raw_image_destination_blend(
    commands: &[PaintCommand],
) -> Option<(incular_rendering::LayerId, BlendMode, Rect)> {
    commands.iter().find_map(|command| match command {
        PaintCommand::PushBlend {
            layer,
            mode,
            bounds,
            ..
        } => Some((*layer, *mode, *bounds)),
        _ => None,
    })
}

#[test]
fn raw_image_defaults_to_source_over_and_skips_the_effect_layer() {
    for raw in [RawImage::new(), RawImage::builder().build()] {
        let mut tree = WidgetTree::new();
        let id = tree.mount(raw.image(handle(10, 10)).into()).expect("mount");
        let render = tree.render_id(id).expect("raw image render");
        assert_eq!(
            raw_image_render_blend(tree.render_object_kind(render)),
            BlendMode::SrcOver
        );
        let size = Size::new(20., 20.);
        tree.layout(Constraints::tight(size)).expect("layout");
        tree.update_compositor(incular_core::time::Instant::now())
            .expect("composite");
        let display = tree.paint();
        assert_eq!(images(display.commands()).len(), 1);
        assert_eq!(raw_image_destination_blend(display.commands()), None);
    }
}

#[test]
fn raw_image_retains_its_render_and_image_across_destination_blend_updates() {
    let mut tree = WidgetTree::new();
    let image = handle(10, 10);
    let first = RawImage::builder()
        .image(image.clone())
        .blend_mode(BlendMode::Plus)
        .build();
    let id = tree.mount(first.into()).expect("mount");
    let render = tree.render_id(id).expect("raw image render");
    let mut first_custom_layer = None;
    let mut previous_work = None;
    for mode in [
        BlendMode::Plus,
        BlendMode::Multiply,
        BlendMode::SrcOver,
        BlendMode::Plus,
    ] {
        if first_custom_layer.is_some() {
            tree.update(
                id,
                RawImage::new().image(image.clone()).blend_mode(mode).into(),
            )
            .expect("update");
        }
        let size = Size::new(20., 20.);
        tree.layout(Constraints::tight(size)).expect("layout");
        tree.update_compositor(incular_core::time::Instant::now())
            .expect("composite");

        assert_eq!(tree.render_id(id), Some(render));
        assert_eq!(tree.render_size(render), Some(size));
        assert_eq!(
            raw_image_render_blend(tree.render_object_kind(render)),
            mode
        );
        let display = tree.paint();
        assert!(display.commands().iter().any(|command| matches!(
            command,
            PaintCommand::Image { image: active, .. } if active == &image
        )));
        let blend = raw_image_destination_blend(display.commands());
        if mode == BlendMode::SrcOver {
            assert_eq!(blend, None);
        } else {
            let (layer, actual, _) = blend.expect("destination effect");
            assert_eq!(actual, mode);
            if mode == BlendMode::Multiply {
                assert_eq!(Some(layer), first_custom_layer);
            }
            first_custom_layer = Some(layer);
        }
        let diagnostics = tree.diagnostics();
        let work = (diagnostics.layouts, diagnostics.paints);
        if let Some(previous) = previous_work {
            assert_eq!(
                work, previous,
                "changing only the blend must reuse the layout and picture"
            );
        }
        previous_work = Some(work);
    }
}

#[test]
fn raw_image_destination_blend_follows_tint_and_opacity_and_skips_absent_image() {
    let tint = Color::rgba(0, 0, 255, 255);
    let display = painted(
        RawImage::new()
            .image(handle(10, 10))
            .color(tint)
            .color_blend_mode(BlendMode::Screen)
            .opacity(0.5)
            .blend_mode(BlendMode::Multiply)
            .into(),
        Size::new(10., 10.),
    );
    let stages: Vec<_> = display
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushBlend { mode, .. } if *mode == BlendMode::Multiply => {
                Some("destination")
            }
            PaintCommand::PushOpacity { alpha, .. } if (*alpha - 0.5).abs() < f32::EPSILON => {
                Some("opacity")
            }
            PaintCommand::PushColorFilter { filter, .. }
                if *filter == ColorFilter::blend(tint, BlendMode::Screen) =>
            {
                Some("tint")
            }
            PaintCommand::Image { .. } => Some("image"),
            _ => None,
        })
        .collect();
    assert_eq!(stages, ["destination", "opacity", "tint", "image"]);

    let absent = painted(
        RawImage::new().blend_mode(BlendMode::Clear).into(),
        Size::new(10., 10.),
    );
    assert!(images(&absent).is_empty());
    assert_eq!(raw_image_destination_blend(&absent), None);
}

#[test]
fn raw_image_replacement_blend_stays_inside_the_fitted_image() {
    for (repeat, expected) in [
        (ImageRepeat::NoRepeat, rect(15., 5., 10., 10.)),
        (ImageRepeat::RepeatX, rect(0., 5., 40., 10.)),
        (ImageRepeat::RepeatY, rect(15., 0., 10., 20.)),
        (ImageRepeat::Repeat, rect(0., 0., 40., 20.)),
    ] {
        let display = painted(
            RawImage::new()
                .image(handle(10, 10))
                .repeat(repeat)
                .blend_mode(BlendMode::Src)
                .into(),
            Size::new(40., 20.),
        );
        assert_eq!(
            raw_image_destination_blend(&display)
                .expect("destination effect")
                .2,
            expected
        );
    }
}

#[test]
fn raw_image_defaults_to_fill_only_when_center_slice_is_set() {
    let source = handle(30, 30);
    let slice = rect(10., 10., 10., 10.);
    let centered = rect(35., 15., 30., 30.);
    let fills = rect(0., 0., 100., 60.);
    for (uses_slice, fit, expected) in [
        (false, None, centered),
        (true, Some(BoxFit::ScaleDown), centered),
        (true, Some(BoxFit::Fill), fills),
        (true, None, fills),
    ] {
        let mut raw = RawImage::builder()
            .image(source.clone())
            .width(100.)
            .height(60.)
            .build();
        if uses_slice {
            raw = raw.center_slice(slice);
        }
        if let Some(fit) = fit {
            raw = raw.fit(fit);
        }
        let commands = painted(raw.into(), Size::new(100., 60.));
        let patches = images(&commands);
        assert_eq!(patches.len(), if uses_slice { 9 } else { 1 });
        let mut area = 0.;
        for (index, (_, patch)) in patches.iter().enumerate() {
            assert_eq!(patch.intersection(expected), Some(*patch));
            assert!(
                patches[..index]
                    .iter()
                    .all(|(_, old)| !old.intersects(*patch)),
                "overlapping nine-slice patches"
            );
            area += patch.size.width * patch.size.height;
        }
        assert_eq!(
            area,
            expected.size.width * expected.size.height,
            "slice={uses_slice} fit={fit:?}"
        );
    }
}

#[test]
fn raw_image_inverts_tinted_pixels_in_a_single_retained_filter() {
    let red = Color::rgba(255, 0, 0, 255);
    for (tint_mode, expected, transparent) in [
        (BlendMode::SrcIn, [0., 1., 1., 1.], [0.; 4]),
        (BlendMode::Screen, [0., 0., 0., 1.], [0., 1., 1., 1.]),
    ] {
        let commands = painted(
            RawImage::new()
                .image(handle(2, 2))
                .color(red)
                .color_blend_mode(tint_mode)
                .invert_colors(true)
                .blend_mode(BlendMode::Src)
                .into(),
            Size::new(6., 4.),
        );
        let [filter] = commands
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushColorFilter { filter, .. } => Some(*filter),
                _ => None,
            })
            .collect::<Vec<_>>()[..]
        else {
            panic!("one filter should retain tint and the post-tint inversion");
        };
        assert_eq!(filter.apply([1., 1., 1., 1.]), expected);
        assert_eq!(filter.apply([1., 1., 1., 0.]), transparent);
        assert_eq!(
            raw_image_destination_blend(&commands).expect("blend").2,
            rect(2., 1., 2., 2.)
        );
    }
}

#[test]
fn raw_image_zero_opacity_preserves_coverage_only_for_destructive_blends() {
    let background = [1., 0., 0., 1.];
    for mode in [
        BlendMode::Clear,
        BlendMode::Src,
        BlendMode::SrcIn,
        BlendMode::DstIn,
        BlendMode::SrcOut,
        BlendMode::DstAtop,
        BlendMode::Modulate,
        BlendMode::SrcOver,
        BlendMode::DstOut,
        BlendMode::Plus,
        BlendMode::Multiply,
    ] {
        let commands = painted(
            RawImage::new()
                .image(handle(2, 2))
                .opacity(0.)
                .blend_mode(mode)
                .into(),
            Size::new(6., 4.),
        );
        let on_transparent = incular_rendering::blend_premultiplied(mode, [0.; 4], background);
        if on_transparent == background {
            assert!(images(&commands).is_empty(), "{mode:?}: {commands:#?}");
        } else {
            assert_eq!(on_transparent, [0.; 4]);
            assert_eq!(
                raw_image_destination_blend(&commands).expect("blend").2,
                rect(2., 1., 2., 2.),
                "{mode:?}"
            );
            assert_eq!(images(&commands).len(), 1);
            let Some(filter) = commands.iter().find_map(|command| match command {
                PaintCommand::PushColorFilter { filter, .. } => Some(*filter),
                _ => None,
            }) else {
                panic!("{mode:?} must feed a transparent source to the blend stage")
            };
            assert_eq!(filter.apply([1., 1., 1., 1.]), [0.; 4]);
        }
    }
}

#[test]
fn raw_image_nine_slice_and_mirroring_keep_blends_inside_the_painted_raster() {
    use incular_config::Alignment;
    let source = handle(30, 30);
    let slice = rect(10., 10., 10., 10.);
    let boundary = rect(0., 0., 100., 60.);
    let clone = || {
        RawImage::new()
            .image(source.clone())
            .blend_mode(BlendMode::Src)
    };
    for (raw, expected, count) in [
        (clone().center_slice(slice), boundary, 9),
        (
            clone().center_slice(slice).fit(BoxFit::ScaleDown),
            rect(35., 15., 30., 30.),
            9,
        ),
        (
            clone()
                .match_text_direction(true)
                .alignment(Alignment::TOP_LEFT),
            rect(0., 0., 30., 30.),
            1,
        ),
        (
            clone()
                .center_slice(slice)
                .fit(BoxFit::ScaleDown)
                .match_text_direction(true)
                .alignment(Alignment::BOTTOM_RIGHT),
            rect(70., 30., 30., 30.),
            9,
        ),
    ] {
        let display = painted(
            Directionality::new(TextDirection::Rtl, raw).into(),
            boundary.size,
        );
        assert_eq!(images(&display).len(), count);
        assert_eq!(
            raw_image_destination_blend(&display).expect("blend").2,
            expected
        );
    }
}

#[test]
fn raw_image_reacts_to_direction_changes_without_replacing_its_render() {
    let image = handle(10, 10);
    let raw = || {
        RawImage::new()
            .image(image.clone())
            .match_text_direction(true)
    };
    let mut tree = WidgetTree::new();
    assert!(matches!(
        tree.mount(raw().into()),
        Err(
            incular_widgets::internal::TreeError::MissingDirectionality {
                widget: "RawImage",
                ..
            }
        ),
    ));

    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, raw()).into())
        .expect("mount");
    let constraints = Constraints::tight(Size::new(20., 15.));
    tree.layout(constraints).expect("build initial scope");
    let child = tree.children(root).expect("inherited scope child")[0];
    let render = tree.render_id(child).expect("image render");
    let mut before = None;
    for direction in [TextDirection::Ltr, TextDirection::Rtl, TextDirection::Ltr] {
        if before.is_some() {
            tree.update(root, Directionality::new(direction, raw()).into())
                .expect("change direction");
        }
        tree.layout(constraints).expect("layout");
        tree.update_compositor(incular_core::time::Instant::now())
            .expect("composite");
        let display = tree.paint();
        assert_eq!(tree.render_id(child), Some(render));
        assert_eq!(tree.render_size(render), Some(Size::new(20., 15.)));
        assert_eq!(images(display.commands()).len(), 1);
        let flipped = display.commands().iter().any(|command| {
            matches!(
                command,
                PaintCommand::PushTransform { transform } if transform.coefficients()[0] < 0.
            )
        });
        assert_eq!(flipped, direction == TextDirection::Rtl);
        let layouts = tree.diagnostics().layouts;
        if let Some(last) = before {
            // The inherited scope itself may be visited again. The
            // image changes paint only and keeps its measured size.
            assert!(layouts <= last + 1);
        }
        before = Some(layouts);
    }
}

#[test]
fn raw_image_tint_opacity_and_blend_edit_the_same_render_and_keep_existing_stages() {
    let source = handle(10, 10);
    let blue = Color::rgba(0, 0, 255, 255);
    let green = Color::rgba(0, 255, 0, 255);
    let raw = || RawImage::new().image(source.clone());
    let mut tree = WidgetTree::new();
    let id = tree.mount(raw().into()).expect("mount");
    let render = tree.render_id(id).expect("image render");
    let constraints = Constraints::tight(Size::new(20., 20.));
    tree.layout(constraints).expect("initial layout");
    tree.update_compositor(incular_core::time::Instant::now())
        .expect("composite");
    let _ = tree.paint();
    let before = tree.diagnostics();
    let mut retained = (None, None, None);

    for (image, expected, rgba) in [
        (raw().color(blue), [true, false, false], [0., 0., 1., 1.]),
        (
            raw().color(blue).opacity(0.5),
            [true, true, false],
            [0., 0., 1., 1.],
        ),
        (
            raw().color(blue).opacity(0.5).blend_mode(BlendMode::Src),
            [true, true, true],
            [0., 0., 1., 1.],
        ),
        (
            raw().color(green).opacity(0.5).blend_mode(BlendMode::Src),
            [true, true, true],
            [0., 1., 0., 1.],
        ),
        (
            raw().color(green).blend_mode(BlendMode::Src),
            [true, false, true],
            [0., 1., 0., 1.],
        ),
        (
            raw().blend_mode(BlendMode::Src),
            [false, false, true],
            [1.; 4],
        ),
        (raw(), [false, false, false], [1.; 4]),
    ] {
        tree.update(id, image.into()).expect("change image paint");
        tree.layout(constraints).expect("layout");
        tree.update_compositor(incular_core::time::Instant::now())
            .expect("composite");
        let display = tree.paint();
        tree.verify_invariants().expect("raster layer ownership");
        assert_eq!(tree.render_id(id), Some(render));
        assert_eq!(images(display.commands()).len(), 1);
        let filter = display.commands().iter().find_map(|command| match command {
            PaintCommand::PushColorFilter { layer, filter, .. } => {
                assert_eq!(filter.apply([1.; 4]), rgba);
                Some(*layer)
            }
            _ => None,
        });
        let opacity = display.commands().iter().find_map(|command| match command {
            PaintCommand::PushOpacity { layer, alpha, .. } => {
                assert_eq!(*alpha, 0.5);
                Some(*layer)
            }
            _ => None,
        });
        let blend = raw_image_destination_blend(display.commands()).map(|(layer, mode, _)| {
            assert_eq!(mode, BlendMode::Src);
            layer
        });
        assert_eq!(
            [filter.is_some(), opacity.is_some(), blend.is_some()],
            expected
        );
        for (old, new) in [
            (retained.0, filter),
            (retained.1, opacity),
            (retained.2, blend),
        ] {
            if let (Some(old), Some(new)) = (old, new) {
                assert_eq!(old, new, "an existing raster stage must not be replaced");
            }
        }
        retained = (filter, opacity, blend);
        let after = tree.diagnostics();
        assert_eq!(after.layouts, before.layouts);
        assert_eq!(after.paints, before.paints);
    }

    tree.update(
        id,
        RawImage::new()
            .color(green)
            .opacity(0.5)
            .blend_mode(BlendMode::Src)
            .into(),
    )
    .expect("clear the styled image");
    tree.layout(constraints).expect("layout");
    tree.update_compositor(incular_core::time::Instant::now())
        .expect("composite");
    let display = tree.paint();
    assert_eq!(tree.render_id(id), Some(render));
    assert!(images(display.commands()).is_empty());
    assert!(!display.commands().iter().any(|command| matches!(
        command,
        PaintCommand::PushColorFilter { .. }
            | PaintCommand::PushOpacity { .. }
            | PaintCommand::PushBlend { .. }
    )));
    tree.verify_invariants()
        .expect("no raster layers after clear");
}

#[test]
fn raw_image_updates_loaded_configuration_on_the_original_render() {
    use incular_config::Alignment;
    use incular_widgets::internal::RenderKind;
    let original_image = handle(10, 10);
    let replacement = handle(2, 2);
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            Directionality::new(
                TextDirection::Ltr,
                RawImage::new().image(original_image.clone()),
            )
            .into(),
        )
        .expect("mount");
    tree.layout(Constraints::loose(Size::new(20., 20.)))
        .expect("initial layout");
    let id = tree.children(root).expect("direction scope")[0];
    let render = tree.render_id(id).expect("image render");
    let config = |kind: Option<&RenderKind>| match kind {
        Some(RenderKind::Image {
            image,
            width,
            height,
            scale,
            fit,
            alignment,
            repeat,
            center_slice,
            flip_horizontally,
            sampling,
            ..
        }) => (
            image.as_ref().map(ImageHandle::id),
            *width,
            *height,
            *scale,
            *fit,
            *alignment,
            *repeat,
            *center_slice,
            *flip_horizontally,
            *sampling,
        ),
        _ => panic!("expected the original render to remain an image"),
    };
    assert_eq!(
        config(tree.render_object_kind(render)),
        (
            Some(original_image.id()),
            None,
            None,
            1.0,
            BoxFit::ScaleDown,
            Alignment::CENTER,
            ImageRepeat::NoRepeat,
            None,
            false,
            ImageSampling::Linear,
        )
    );

    let center_slice = rect(0., 0., 1., 1.);
    let changed = RawImage::new()
        .image(replacement.clone())
        .width(1.)
        .height(1.)
        .scale(2.)
        .color(Color::BLACK)
        .opacity(0.)
        .color_blend_mode(BlendMode::Difference)
        .fit(BoxFit::Contain)
        .alignment(Alignment::TOP_CENTER)
        .repeat(ImageRepeat::Repeat)
        .center_slice(center_slice)
        .match_text_direction(true)
        .invert_colors(true)
        .filter_quality(incular_rendering::FilterQuality::High);
    tree.update(id, changed.into())
        .expect("update raster configuration");
    tree.layout(Constraints::loose(Size::new(20., 20.)))
        .expect("updated layout");
    assert_eq!(tree.render_id(id), Some(render));
    assert_eq!(tree.render_size(render), Some(Size::new(1., 1.)));
    assert_eq!(
        config(tree.render_object_kind(render)),
        (
            Some(replacement.id()),
            Some(1.),
            Some(1.),
            2.,
            BoxFit::Contain,
            Alignment::TOP_CENTER,
            ImageRepeat::Repeat,
            Some(center_slice),
            false,
            ImageSampling::Linear,
        )
    );
    assert!(
        images(tree.paint().commands()).is_empty(),
        "zero opacity paints no image under SrcOver"
    );
}
