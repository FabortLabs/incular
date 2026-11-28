use incular_config::{Clip, Constraints};
use incular_core::{Color, Offset, Size};
use incular_rendering::{BlendMode, Brush, DisplayList, PaintCommand};
use incular_widgets::{
    BlurStyle, Border, BoxDecoration, BoxShadow, DecoratedBox, DecoratedShapeBox, Positioned,
    ShapeDecoration, SizedBox, Stack, Widget, internal::WidgetTree,
};

const RED: Color = Color::rgba(255, 0, 0, 255);
const BLUE: Color = Color::rgba(0, 0, 255, 255);

fn scene(shape: bool, style: BlurStyle, radius: f32, alpha: u8) -> DisplayList {
    let shadow = BoxShadow {
        blur_style: style,
        ..BoxShadow::new(Color { alpha, ..RED }, Offset::ZERO, radius, 0.)
    };
    let contents: Widget = if shape {
        DecoratedShapeBox::new(
            ShapeDecoration::new(Border::default()).shadows([shadow]),
            SizedBox::expand(),
        )
        .into()
    } else {
        DecoratedBox::new(SizedBox::expand())
            .decoration(BoxDecoration::new().box_shadow([shadow]))
            .into()
    };
    let body = Stack::new([
        Widget::box_(Size::new(50., 50.), BLUE),
        Widget::from(
            Positioned::new(contents)
                .left(15.)
                .top(15.)
                .width(20.)
                .height(20.),
        ),
    ])
    .clip_behavior(Clip::None);
    let mut tree = WidgetTree::new();
    tree.mount(body.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(50., 50.)))
        .unwrap();
    tree.update_compositor(incular_core::time::Instant::now())
        .unwrap();
    tree.paint()
}

fn red_alphas(commands: &[PaintCommand]) -> Vec<u8> {
    commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Rect { color, .. }
            | PaintCommand::RRect {
                brush: Brush::Solid(color),
                ..
            }
            | PaintCommand::FillPath {
                brush: Brush::Solid(color),
                ..
            } if color.red == 255 => Some(color.alpha),
            _ => None,
        })
        .collect()
}

#[test]
fn opaque_outside_or_inside_mask_cannot_read_the_parent_destination() {
    for shape in [false, true] {
        for (style, expected_mask) in [
            (BlurStyle::Outer, BlendMode::DstOut),
            (BlurStyle::Inner, BlendMode::DstIn),
        ] {
            let display = scene(shape, style, 4., 255);
            let commands = display.commands();
            let backdrop = commands
                .iter()
                .position(|cmd| matches!(cmd, PaintCommand::Rect { color: BLUE, .. }))
                .unwrap();
            let isolate = commands
                .iter()
                .position(|cmd| {
                    matches!(
                        cmd,
                        PaintCommand::PushBlend {
                            mode: BlendMode::SrcOver,
                            ..
                        }
                    )
                })
                .unwrap_or_else(|| {
                    panic!("missing isolated shadow destination: shape={shape}, style={style:?}")
                });
            let blur = commands
                .iter()
                .position(|cmd| matches!(cmd, PaintCommand::PushBlur { .. }))
                .unwrap();
            let mask = commands.iter().position(|cmd| matches!(cmd, PaintCommand::PushBlend { mode, .. } if *mode == expected_mask)).unwrap();
            assert!(backdrop < isolate && isolate < blur && blur < mask);
            assert_eq!(red_alphas(commands), [255, 255]);
        }
    }
}

#[test]
fn translucent_masks_apply_alpha_once_after_combining_the_source_and_blur() {
    for shape in [false, true] {
        for style in [BlurStyle::Solid, BlurStyle::Outer, BlurStyle::Inner] {
            let display = scene(shape, style, 4., 128);
            let commands = display.commands();
            let alpha = commands.iter().position(|cmd| matches!(cmd, PaintCommand::PushOpacity { alpha, .. } if *alpha == 128. / 255.)).unwrap();
            let blur = commands
                .iter()
                .position(|cmd| matches!(cmd, PaintCommand::PushBlur { .. }))
                .unwrap();
            assert!(alpha < blur);
            assert_eq!(red_alphas(commands), [255, 255]);
        }
        let normal = scene(shape, BlurStyle::Normal, 4., 128);
        assert_eq!(red_alphas(normal.commands()), [128]);
        assert!(
            !normal
                .commands()
                .iter()
                .any(|cmd| matches!(cmd, PaintCommand::PushOpacity { .. }))
        );
    }
}

#[test]
fn radius_zero_draws_the_original_shadow_once_regardless_of_mask_style() {
    for shape in [false, true] {
        for style in [
            BlurStyle::Normal,
            BlurStyle::Solid,
            BlurStyle::Outer,
            BlurStyle::Inner,
        ] {
            let display = scene(shape, style, 0., 128);
            assert_eq!(red_alphas(display.commands()), [128]);
            assert!(!display.commands().iter().any(|cmd| matches!(
                cmd,
                PaintCommand::PushBlur { .. }
                    | PaintCommand::PushBlend { .. }
                    | PaintCommand::PushOpacity { .. }
            )));
        }
    }
}

#[test]
fn wide_offset_shadow_moves_its_mask_without_adding_a_border_or_moving_the_box_fill() {
    use incular_core::{Rect, Transform};
    const AMBER: Color = Color::rgba(255, 193, 7, 255);
    const INK: Color = Color::rgba(244, 67, 54, 255);
    let canvas = Size::new(128., 128.);
    let form =
        |x, y, width, height| Rect::from_origin_size(Offset::new(x, y), Size::new(width, height));
    for (style, mask) in [
        (BlurStyle::Normal, None),
        (BlurStyle::Solid, None),
        (BlurStyle::Outer, Some(BlendMode::DstOut)),
        (BlurStyle::Inner, Some(BlendMode::DstIn)),
    ] {
        let box_face = DecoratedBox::new(SizedBox::expand()).decoration(
            BoxDecoration::new()
                .color(Color::BLACK)
                .box_shadow([BoxShadow {
                    blur_style: style,
                    ..BoxShadow::new(INK, Offset::new(4., 4.), 16., 2.)
                }]),
        );
        let root = Stack::new([
            Widget::box_(canvas, AMBER),
            Positioned::new(box_face)
                .left(32.)
                .top(32.)
                .width(64.)
                .height(64.)
                .into(),
        ])
        .clip_behavior(Clip::None);
        let mut tree = WidgetTree::new();
        tree.mount(root.into()).unwrap();
        tree.layout(Constraints::tight(canvas)).unwrap();
        tree.update_compositor(incular_core::time::Instant::now())
            .unwrap();
        let display = tree.paint();
        let mut transforms = vec![Transform::IDENTITY];
        let mut fills = Vec::new();
        let mut modes = Vec::new();
        let mut sigmas = Vec::new();
        for command in display.commands() {
            let transform = *transforms.last().unwrap();
            match command {
                PaintCommand::PushTransform { transform: next } => {
                    transforms.push(transform.then(*next))
                }
                PaintCommand::PopTransform => {
                    transforms.pop();
                }
                PaintCommand::Rect { rect, color } => {
                    fills.push((transform.transform_rect_bbox(*rect), *color))
                }
                PaintCommand::RRect {
                    rrect,
                    brush: Brush::Solid(color),
                } => fills.push((transform.transform_rect_bbox(rrect.rect), *color)),
                PaintCommand::PushBlend { mode, .. } => modes.push(*mode),
                PaintCommand::PushBlur { blur, .. } => sigmas.push((blur.sigma_x, blur.sigma_y)),
                PaintCommand::Border { .. } | PaintCommand::StrokePath { .. } => {
                    panic!("a shadow must not paint a border: {style:?}, {command:?}")
                }
                _ => {}
            }
        }
        assert_eq!(fills[0], (form(0., 0., 128., 128.), AMBER));
        assert_eq!(
            fills.last().copied(),
            Some((form(32., 32., 64., 64.), Color::BLACK))
        );
        let count = if style == BlurStyle::Normal { 1 } else { 2 };
        assert_eq!(fills.len(), 2 + count, "{style:?}");
        assert!(
            fills[1..1 + count]
                .iter()
                .all(|&(rect, color)| rect == form(34., 34., 68., 68.) && color == INK),
            "{style:?}: {fills:?}"
        );
        assert_eq!(sigmas.len(), 1);
        let expected = 16. * 0.57735 + 0.5;
        assert!((sigmas[0].0 - expected).abs() < 0.00001);
        assert_eq!(sigmas[0].0, sigmas[0].1);
        if let Some(mask) = mask {
            assert_eq!(modes, [BlendMode::SrcOver, mask]);
        } else {
            assert!(modes.is_empty());
        }
    }
}
