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

#[test]
fn control_container_paints_background_child_and_foreground_in_reference_order() {
    use incular_config::Alignment;
    use incular_core::Transform;
    use incular_rendering::CornerRadii;
    use incular_widgets::{Align, SizedBox};

    const GREEN: Color = Color::rgba(0, 255, 0, 255);
    const YELLOW: Color = Color::rgba(255, 255, 0, 255);
    const FOREGROUND: Color = Color::rgba(0, 0, 255, 127);

    let child = SizedBox::new()
        .width(25.)
        .height(33.)
        .child(Container::new().decoration(BoxDecoration::new().color(YELLOW)));
    let container = Container::new()
        .alignment(Alignment::BOTTOM_RIGHT)
        .padding(EdgeInsets::all(7.))
        .color(GREEN)
        .foreground_decoration(BoxDecoration::new().color(FOREGROUND))
        .width(53.)
        .height(76.)
        .constraints(Constraints::new(50., 55., 78., 82.))
        .margin(EdgeInsets::all(5.))
        .child(child);
    let mut tree = WidgetTree::new();
    tree.mount(Align::new(Alignment::TOP_LEFT, container).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let mut transforms = vec![Transform::IDENTITY];
    let mut fills = Vec::new();
    for command in tree.paint().commands() {
        let current = *transforms.last().unwrap();
        match command {
            PaintCommand::PushTransform { transform } => transforms.push(current.then(*transform)),
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Rect { rect, color } => {
                fills.push((current.transform_rect_bbox(*rect), *color))
            }
            PaintCommand::RRect {
                rrect,
                brush: Brush::Solid(color),
            } => {
                assert_eq!(rrect.radii, CornerRadii::ZERO);
                fills.push((current.transform_rect_bbox(rrect.rect), *color));
            }
            _ => panic!("no border, clip, or extra paint expected: {command:?}"),
        }
    }
    let rect = |x, y, w, h| Rect::from_origin_size(Offset::new(x, y), Size::new(w, h));
    assert_eq!(
        fills,
        [
            (rect(5., 5., 53., 78.), GREEN),
            (rect(26., 43., 25., 33.), YELLOW),
            (rect(5., 5., 53., 78.), FOREGROUND),
        ]
    );
}

#[test]
fn missing_directionality_for_directional_container_radii_is_a_detailed_error() {
    use incular_widgets::{Border, BorderRadiusDirectional, internal::TreeError};

    let decoration = BoxDecoration::new()
        .border(Border::new(1., Color::BLACK))
        .directional_border_radius(BorderRadiusDirectional::circular(1.));
    let mut tree = WidgetTree::new();
    let error = tree
        .mount(Container::new().decoration(decoration).into())
        .and_then(|_| tree.layout(Constraints::tight(Size::new(80., 80.))))
        .expect_err("a directional radius cannot resolve without Directionality");
    assert!(
        matches!(
            error,
            TreeError::MissingDirectionality {
                widget: "Container",
                ..
            }
        ),
        "{error:?}"
    );
    let message = error.to_string();
    assert!(message.contains("No TextDirection found."));
    assert!(message.contains("without a Directionality ancestor"));
}

#[test]
fn directional_decoration_reads_the_local_scope_and_cannot_cross_a_lookup_boundary() {
    use incular_widgets::{
        BorderRadiusDirectional, DecoratedBox, Directionality, LookupBoundary, SizedBox,
        TextDirection, internal::TreeError,
    };

    let decorate = || {
        DecoratedBox::new(SizedBox::shrink()).decoration(
            BoxDecoration::new()
                .color(Color::WHITE)
                .directional_border_radius(BorderRadiusDirectional::circular(4.)),
        )
    };
    let mut tree = WidgetTree::new();
    let root: Widget = Directionality::new(TextDirection::Rtl, decorate()).into();
    tree.mount(root).unwrap();
    tree.layout(Constraints::tight(Size::new(80., 80.)))
        .unwrap();
    assert!(tree.paint().commands().iter().any(|command| {
        matches!(command, PaintCommand::RRect { rrect, .. }
            if rrect.radii.top_left == 4.)
    }));

    let mut tree = WidgetTree::new();
    let root: Widget =
        Directionality::new(TextDirection::Rtl, LookupBoundary::new(decorate())).into();
    let error = tree
        .mount(root)
        .expect_err("the boundary hides the outer direction");
    assert!(matches!(
        error,
        TreeError::MissingDirectionality {
            widget: "DecoratedBox",
            ..
        }
    ));
}
