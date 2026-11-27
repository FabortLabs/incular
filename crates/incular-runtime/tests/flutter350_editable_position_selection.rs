use std::time::Duration;

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, Size,
    TextDirection, Transform, time::Instant,
};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{
    TextAffinity, TextEditingController, TextSelection, TextStyle, register_application_font_as,
};
use incular_widgets::EditableText;

fn ahem() -> Option<TextStyle> {
    let path = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../.tmp/flutter/packages/flutter_tools/static/Ahem.ttf")
        });
    if !path.is_file() {
        eprintln!("Pinned Flutter Ahem is unavailable; position comparison not run");
        return None;
    }
    let family = "Pinned position selection Ahem";
    register_application_font_as(family, std::fs::read(path).unwrap());
    Some(
        TextStyle::new()
            .font_family(family)
            .font_size(10.)
            .height(Some(1.)),
    )
}

fn frame(runtime: &mut Runtime, time: Instant) -> DisplayList {
    runtime
        .run_frame_at(Constraints::loose(Size::new(100., 100.)), time)
        .unwrap()
        .0
}

fn glyph_origins(list: &DisplayList) -> Vec<Offset> {
    let mut transforms = vec![Transform::IDENTITY];
    let mut origins = Vec::new();
    for command in list.commands() {
        match command {
            PaintCommand::PushTransform { transform } => {
                transforms.push(transforms.last().unwrap().then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::GlyphRun { run, .. } => {
                origins.push(transforms.last().unwrap().transform_point(run.origin));
            }
            _ => {}
        }
    }
    assert_eq!(transforms.len(), 1);
    origins
}

fn touch(runtime: &mut Runtime, phase: PointerPhase, x: f32, at: Instant) {
    let buttons = if phase == PointerPhase::Up {
        0
    } else {
        PRIMARY_POINTER_BUTTON
    };
    let _ = runtime.handle_input_at(
        InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 1,
            kind: PointerDeviceKind::Touch,
            position: Offset::new(x, 2.),
            phase,
            buttons,
            button: Some(PRIMARY_POINTER_BUTTON),
            sample: Default::default(),
        },
        at,
    );
}

#[test]
fn selection_and_glyph_paint_follow_a_fixed_vertical_editor_offset() {
    let Some(style) = ahem() else { return };
    let controller = TextEditingController::with_text("test\ntest");
    controller.set_selection(TextSelection::collapsed(4));
    let editor = |offset| {
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .style(style.clone())
            .max_lines(Some(2))
            .cursor_width(1.)
            .select_all_on_focus(false)
            .fixed_scroll_offset(offset)
            .into()
    };
    let mut runtime = Runtime::new(editor(None)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let time = Instant::now();
    let before = glyph_origins(&frame(&mut runtime, time));
    assert_eq!(before.len(), 2);
    let first_point = Offset::new(0., 2.);
    let field = runtime.tree().root().unwrap();
    assert!(runtime.select_editable_position_at(field, first_point, None));
    assert_eq!(controller.selection(), TextSelection::collapsed(0));

    runtime.schedule_update(field, editor(Some(10.))).unwrap();
    let scrolled = glyph_origins(&frame(&mut runtime, time + Duration::from_millis(1)));
    assert_eq!(
        scrolled,
        before
            .into_iter()
            .map(|origin| origin - Offset::new(0., 10.))
            .collect::<Vec<_>>()
    );
    assert!(runtime.select_editable_position_at(field, first_point, None));
    assert_eq!(controller.selection(), TextSelection::collapsed(5));

    touch(
        &mut runtime,
        PointerPhase::Down,
        10.,
        time + Duration::from_millis(10),
    );
    touch(
        &mut runtime,
        PointerPhase::Up,
        10.,
        time + Duration::from_millis(35),
    );
    let _ = frame(&mut runtime, time + Duration::from_millis(40));
    assert_eq!(controller.selection(), TextSelection::collapsed(6));

    touch(
        &mut runtime,
        PointerPhase::Down,
        20.,
        time + Duration::from_millis(700),
    );
    let _ = frame(&mut runtime, time + Duration::from_millis(1300));
    assert_eq!(controller.selection(), TextSelection::new(5, 9));
    touch(
        &mut runtime,
        PointerPhase::Up,
        20.,
        time + Duration::from_millis(1310),
    );

    let last = Offset::new(30., 2.);
    assert!(runtime.select_editable_words_in_range(field, last, last));
    assert_eq!(controller.selection(), TextSelection::new(5, 9));
}

#[test]
fn a_reversed_position_range_preserves_base_and_extent_direction() {
    let Some(style) = ahem() else { return };
    let controller = TextEditingController::with_text("abc def ghi");
    let mut runtime = Runtime::new(
        EditableText::new(controller.clone())
            .text_direction(TextDirection::Ltr)
            .style(style)
            .into(),
    )
    .unwrap();
    let _ = frame(&mut runtime, Instant::now());
    let field = runtime.tree().root().unwrap();
    let (from, to) = (Offset::new(30., 2.), Offset::new(10., 2.));
    let affinity = runtime
        .tree()
        .text_field_position_at(field, from)
        .unwrap()
        .1;
    assert!(runtime.select_editable_position_at(field, from, Some(to)));
    assert_eq!(controller.selection(), TextSelection::new(3, 1));
    assert_eq!(controller.caret_affinity(), affinity);
    assert_eq!(runtime.focused_element(), None);
    assert!(!runtime.select_editable_position_at(
        field,
        from,
        Some(Offset::new(f32::INFINITY, 2.))
    ));
    assert_eq!(controller.selection(), TextSelection::new(3, 1));
}

#[test]
fn platform_word_selection_at_the_second_space_matches_read_only_and_ios_rules() {
    let Some(style) = ahem() else { return };
    for (platform, read_only, text, expected) in [
        (
            TextShortcutPlatform::Android,
            true,
            "  test",
            TextSelection::new(1, 2),
        ),
        (
            TextShortcutPlatform::Ios,
            false,
            "  test",
            TextSelection::new(1, 6),
        ),
        (
            TextShortcutPlatform::Ios,
            false,
            "   ",
            TextSelection::collapsed(1),
        ),
    ] {
        let controller = TextEditingController::with_text(text);
        controller.set_selection(TextSelection::collapsed(4));
        let mut runtime = Runtime::new(
            EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr)
                .style(style.clone())
                .read_only(read_only)
                .into(),
        )
        .unwrap();
        runtime.set_text_shortcut_platform(platform);
        let _ = frame(&mut runtime, Instant::now());
        let field = runtime.tree().root().unwrap();
        let point = Offset::new(10., 2.);
        for (x, affinity) in [
            (9., TextAffinity::Upstream),
            (10., TextAffinity::Downstream),
            (11., TextAffinity::Downstream),
        ] {
            assert_eq!(
                runtime
                    .tree()
                    .text_field_position_at(field, Offset::new(x, 2.)),
                Some((1, affinity))
            );
        }
        assert!(runtime.select_editable_words_in_range(field, point, point));
        assert_eq!(controller.selection(), expected, "{platform:?} {text:?}");
        assert_eq!(controller.text(), text);
    }
}
