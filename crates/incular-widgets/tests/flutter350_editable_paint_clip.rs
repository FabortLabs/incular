use incular_config::{Clip, Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_rendering::{ClipCoverage, PaintCommand};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{EditableText, Widget, internal::WidgetTree};

fn field(controller: TextEditingController, mode: Option<Clip>) -> Widget {
    let field = EditableText::new(controller).text_direction(TextDirection::Ltr);
    match mode {
        Some(mode) => field.clip_behavior(mode).into(),
        None => field.into(),
    }
}

#[test]
fn editable_only_clips_overflow_and_applies_the_actual_requested_mode() {
    let bounds = Size::new(60., 30.);
    let rect = Rect::from_origin_size(Offset::ZERO, bounds);
    let controller = TextEditingController::new();
    let mut tree = WidgetTree::new();
    let root = tree.mount(field(controller.clone(), None)).unwrap();
    let render = tree.render_id(root).unwrap();
    for mode in [
        None,
        Some(Clip::None),
        Some(Clip::AntiAlias),
        Some(Clip::AntiAliasWithSaveLayer),
        None,
    ] {
        tree.update(root, field(controller.clone(), mode)).unwrap();
        for overflow in [false, true] {
            controller.set_text(if overflow { "AAAAAAAAAAAA" } else { "A" });
            controller.set_selection(TextSelection::collapsed(0));
            tree.layout(Constraints::tight(bounds)).unwrap();
            tree.update_compositor(incular_core::time::Instant::now())
                .unwrap();
            assert_eq!(tree.render_id(root), Some(render), "reuses the real render");
            let coverage = if !overflow {
                None
            } else {
                match mode {
                    Some(Clip::None) => None,
                    Some(Clip::AntiAlias) => Some(ClipCoverage::AntiAlias),
                    Some(Clip::AntiAliasWithSaveLayer) => {
                        Some(ClipCoverage::AntiAliasWithSaveLayer)
                    }
                    _ => Some(ClipCoverage::HardEdge),
                }
            };
            let described = tree.editable_approximate_paint_clip(root);
            assert_eq!(
                described,
                coverage.map(|_| rect),
                "{mode:?}, overflow={overflow}"
            );
            let paint = tree.paint();
            let mut depth = 0;
            let mut clips = Vec::new();
            let mut max_glyph_x = 0.0_f32;
            for command in paint.commands() {
                match command {
                    PaintCommand::PushClip { rect } => {
                        clips.push((ClipCoverage::HardEdge, *rect));
                        depth += 1;
                    }
                    PaintCommand::PushClipRRect {
                        rrect,
                        coverage,
                        isolation,
                    } => {
                        assert_eq!(
                            isolation.is_some(),
                            *coverage == ClipCoverage::AntiAliasWithSaveLayer
                        );
                        clips.push((*coverage, rrect.rect));
                        depth += 1;
                    }
                    PaintCommand::PopClip => depth -= 1,
                    PaintCommand::GlyphRun { run, .. } => {
                        assert_eq!(depth, i32::from(coverage.is_some()));
                        for glyph in run.glyphs.iter() {
                            max_glyph_x =
                                max_glyph_x.max(run.origin.x + glyph.offset.x + glyph.advance);
                        }
                    }
                    _ => {}
                }
            }
            assert_eq!(depth, 0);
            assert_eq!(
                clips,
                coverage
                    .map(|coverage| (coverage, rect))
                    .into_iter()
                    .collect::<Vec<_>>()
            );
            assert!(max_glyph_x > 0.);
            assert_eq!(
                max_glyph_x > bounds.width,
                overflow,
                "{mode:?}, overflow={overflow}"
            );
        }
    }
}

#[test]
fn isolated_overflow_clip_reuses_layer_and_updates_the_cached_contents() {
    let bounds = Size::new(60., 30.);
    let controller = TextEditingController::with_text("AAAAAAAAAAAA");
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(field(
            controller.clone(),
            Some(Clip::AntiAliasWithSaveLayer),
        ))
        .unwrap();
    let now = incular_core::time::Instant::now();
    tree.set_focused(root, true, now).unwrap();
    let mut paint_selection = |end| {
        controller.set_selection(TextSelection {
            base: 0,
            extent: end,
        });
        tree.layout(Constraints::tight(bounds)).unwrap();
        tree.update_compositor(now).unwrap();
        let list = tree.paint();
        let mut within_clip = false;
        let mut painted_selection = false;
        let mut result = None;
        for command in list.commands() {
            match command {
                PaintCommand::PushClipRRect {
                    isolation: Some(id),
                    ..
                } => {
                    within_clip = true;
                    result = Some(*id);
                }
                PaintCommand::PopClip => within_clip = false,
                PaintCommand::Rect { .. } => {
                    assert!(
                        within_clip,
                        "overflowing selection must be inside the same clip as glyphs"
                    );
                    painted_selection = true;
                }
                PaintCommand::Border { .. } => {
                    panic!("bare editable painted an unrequested border")
                }
                _ => {}
            }
        }
        assert!(painted_selection);
        result.expect("save-layer clip")
    };
    let initial = paint_selection(8);
    let quiet = paint_selection(8);
    let changed = paint_selection(10);
    assert_eq!(initial, quiet, "unchanged paints keep their cache identity");
    assert_eq!(changed.layer, initial.layer);
    assert_ne!(changed.generation, initial.generation);
}
