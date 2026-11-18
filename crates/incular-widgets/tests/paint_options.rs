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
    BlendMode, Canvas, ColorFilter, ImageFilter, ImageFilterTarget, PaintCommand,
};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BlurStyle, BorderRadius, BorderSide, BoxDecoration, BoxShadow, Container,
    ContinuousRectangleBorder, DecorationImage, Directionality, Image, ImageFiltered, LinearBorder,
    LinearBorderEdge, Positioned, RawImage, ShapeBorder, SizedBox, Stack, Widget,
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
        SizedBox::new().width(10.).height(10.),
    )
    .expect("matrix is supported");
    let commands = painted(
        Stack::new([Widget::from(matrix)]).into(),
        Size::new(100., 100.),
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, PaintCommand::PushTransform { .. })),
        "{commands:#?}"
    );

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
