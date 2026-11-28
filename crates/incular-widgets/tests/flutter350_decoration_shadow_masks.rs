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
