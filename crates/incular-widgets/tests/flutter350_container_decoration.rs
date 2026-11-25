use incular_config::{Clip, Constraints, EdgeInsets};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    BorderRadius, BoxDecoration, BoxShadow, BoxShape, Container, Radius, Widget,
};

#[test]
fn builder_and_fluent_plain_decoration_fill_the_same_sized_box() {
    let size = Size::new(40.0, 40.0);
    let decoration = BoxDecoration::new().color(Color::WHITE);
    let containers = [
        Container::builder()
            .width(size.width)
            .height(size.height)
            .decoration(decoration.clone())
            .build(),
        Container::new()
            .width(size.width)
            .height(size.height)
            .decoration(decoration),
    ];
    for (index, container) in containers.into_iter().enumerate() {
        let mut tree = WidgetTree::new();
        tree.mount(container.into()).expect("mount");
        tree.layout(Constraints::loose(Size::new(100.0, 100.0)))
            .expect("layout");
        let bounds = Rect::from_origin_size(Offset::ZERO, size);
        assert!(
            tree.paint().commands().iter().any(|command| match command {
                PaintCommand::Rect { rect, color } => *rect == bounds && *color == Color::WHITE,
                PaintCommand::RRect {
                    rrect,
                    brush: Brush::Solid(color),
                } => rrect.rect == bounds && *color == Color::WHITE,
                _ => false,
            }),
            "constructor {index} did not paint its 40×40 decoration"
        );
    }
}

fn painted(container: Container) -> incular_rendering::DisplayList {
    let mut tree = WidgetTree::new();
    tree.mount(container.into()).expect("mount");
    tree.layout(Constraints::loose(Size::new(200.0, 200.0)))
        .expect("layout");
    tree.paint()
}

#[test]
fn a_nonsquare_circle_clips_only_the_child_and_only_when_requested() {
    let size = Size::new(100.0, 60.0);
    for mode in [Clip::None, Clip::HardEdge] {
        let commands = painted(
            Container::new()
                .width(size.width)
                .height(size.height)
                .clip_behavior(mode)
                .decoration(BoxDecoration::new().shape(BoxShape::Circle))
                .child(Widget::box_(size, Color::WHITE)),
        );
        let clips: Vec<_> = commands
            .commands()
            .iter()
            .filter(|command| is_clip(command))
            .collect();
        if mode == Clip::None {
            assert!(clips.is_empty());
        } else {
            assert_eq!(clips.len(), 1);
            let PaintCommand::PushClipRRect { rrect, .. } = clips[0] else {
                panic!("expected an inscribed circle, got {:?}", clips[0]);
            };
            assert_eq!(
                rrect.rect,
                Rect::from_origin_size(Offset::new(20.0, 0.0), Size::new(60.0, 60.0))
            );
            assert!(rrect.contains(Offset::new(50.0, 30.0)));
            assert!(!rrect.contains(Offset::new(10.0, 30.0)));
        }
    }
}

fn is_clip(command: &PaintCommand) -> bool {
    matches!(
        command,
        PaintCommand::PushClip { .. }
            | PaintCommand::PushClipRRect { .. }
            | PaintCommand::PushClipPath { .. }
            | PaintCommand::PushClipOval { .. }
    )
}

#[test]
fn child_clip_does_not_clip_the_decoration_shadow() {
    let child_color = Color::rgba(10, 9, 8, 255);
    let size = Size::new(50.0, 50.0);
    let list = painted(
        Container::new()
            .decoration(
                BoxDecoration::new()
                    .border_radius(BorderRadius::all(Radius::circular(30.0)))
                    .box_shadow([BoxShadow {
                        color: Color::BLACK,
                        spread_radius: 10.0,
                        blur_radius: 20.0,
                        ..BoxShadow::default()
                    }]),
            )
            .clip_behavior(Clip::HardEdge)
            .child(Widget::box_(size, child_color))
            .margin(EdgeInsets::all(30.0)),
    );
    let commands = list.commands();
    let shadow = commands
        .iter()
        .position(|command| {
            matches!(command,
                PaintCommand::RRect { rrect, brush: Brush::Solid(Color::BLACK) }
                if rrect.rect.size == Size::new(70.0, 70.0)
            )
        })
        .expect("shadow fill");
    let clip = commands.iter().position(is_clip).expect("child clip");
    let child = commands
        .iter()
        .position(|command| {
            matches!(command,
                PaintCommand::Rect { color, .. } if *color == child_color
            )
        })
        .expect("child fill");
    assert!(shadow < clip && clip < child, "{commands:?}");
}
