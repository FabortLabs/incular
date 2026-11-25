use incular_config::{Axis, Clip, Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{ClipCoverage, DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{Flex, Widget};

fn view(mode: Option<Clip>) -> Widget {
    let mut flex = Flex::new(
        Axis::Vertical,
        [Widget::box_(Size::new(200., 200.), Color::WHITE)],
    )
    .text_direction(TextDirection::Ltr);
    if let Some(mode) = mode {
        flex = flex.clip_behavior(mode);
    }
    flex.into()
}

fn paint(runtime: &mut Runtime, height: f32) -> DisplayList {
    runtime
        .run_frame(Constraints::at_most(100., height))
        .unwrap()
        .0
}

fn clip(list: &DisplayList) -> Vec<(ClipCoverage, Rect, bool)> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::PushClip { rect } => Some((ClipCoverage::HardEdge, *rect, false)),
            PaintCommand::PushClipRRect {
                rrect,
                coverage,
                isolation,
            } => Some((*coverage, rrect.rect, isolation.is_some())),
            _ => None,
        })
        .collect()
}

#[test]
fn flex_records_the_configured_clip_only_when_the_main_axis_overflows() {
    let mut runtime = Runtime::new(view(None)).unwrap();
    let root = runtime.tree().root().unwrap();
    let bounds = Rect::from_origin_size(Offset::ZERO, Size::new(100., 100.));
    assert_eq!(clip(&paint(&mut runtime, 100.)), []);

    for (mode, coverage, isolated) in [
        (Clip::HardEdge, ClipCoverage::HardEdge, false),
        (Clip::AntiAlias, ClipCoverage::AntiAlias, false),
        (
            Clip::AntiAliasWithSaveLayer,
            ClipCoverage::AntiAliasWithSaveLayer,
            true,
        ),
    ] {
        runtime.schedule_update(root, view(Some(mode))).unwrap();
        let overflow = paint(&mut runtime, 100.);
        assert_eq!(clip(&overflow), [(coverage, bounds, isolated)]);
        let commands = overflow.commands();
        let push = commands
            .iter()
            .position(|c| {
                matches!(
                    c,
                    PaintCommand::PushClip { .. } | PaintCommand::PushClipRRect { .. }
                )
            })
            .unwrap();
        let pop = commands
            .iter()
            .position(|c| matches!(c, PaintCommand::PopClip))
            .unwrap();
        assert!(commands[push + 1..pop].iter().any(|command| matches!(
            command,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )));

        let fitting = paint(&mut runtime, 250.);
        assert_eq!(clip(&fitting), []);
        assert!(fitting.commands().iter().any(|command| matches!(
            command,
            PaintCommand::Rect {
                color: Color::WHITE,
                ..
            }
        )));
        assert_eq!(
            clip(&paint(&mut runtime, 100.)),
            [(coverage, bounds, isolated)]
        );
        assert_eq!(runtime.tree().root(), Some(root));
    }

    runtime
        .schedule_update(root, view(Some(Clip::None)))
        .unwrap();
    assert_eq!(clip(&paint(&mut runtime, 100.)), []);
}
