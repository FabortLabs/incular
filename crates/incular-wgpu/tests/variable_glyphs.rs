//! Same font, glyph, ppem and phase must not alias another axis location.
use std::collections::HashSet;
use std::path::Path;

use incular_core::Lerp;
use incular_text::{FontVariation, TextAlign, TextEngine, TextStyle};
use incular_wgpu::{
    GlyphAtlas, GlyphInstanceKey, GlyphRasterRequest, SharedGpuResourceRegistry,
    retire_glyph_page_resources,
};

#[test]
#[ignore = "requires the pinned Flutter SDK font fixture"]
fn shaped_font_axes_control_native_masks_and_atlas_identity() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture = repo
        .join(".tmp/flutter/engine/src/flutter/testing/resources/RobotoSlab-VariableFont_wght.ttf");
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("ParityVariable", std::fs::read(fixture).unwrap()));
    let face = TextStyle::new()
        .font_size(56.)
        .font_family("ParityVariable");
    let style = |weight| {
        face.clone()
            .font_variation_settings(&[FontVariation::weight(weight)])
    };
    let light = style(100.);
    let heavy = style(900.);
    let cases = [
        light.clone(),
        light.lerp(&heavy, 0.5),
        heavy,
        face.clone(),
        style(400.),
        style(500.),
    ];
    let layouts = cases.map(|style| engine.layout("H", &style, None, TextAlign::Start));
    let runs = layouts.each_ref().map(|layout| &layout.lines[0].runs[0]);
    assert!(runs.iter().all(|run| run.font.id() == runs[0].font.id()));
    let glyph = runs[0].glyphs[0].id;
    assert!(runs.iter().all(|run| run.glyphs[0].id == glyph));
    assert!(!runs[0].font.location().is_default());
    assert!(!runs[1].font.location().is_default());
    assert!(!runs[2].font.location().is_default());
    assert!(runs[3].font.location().is_default());
    assert!(runs[4].font.location().is_default());

    let mut atlas = GlyphAtlas::with_max_pages(1);
    let mut registry = SharedGpuResourceRegistry::default();
    let unprotected = HashSet::new();
    let phase = [1, 3];
    let request = GlyphRasterRequest::new(56., 1.);
    let keys = runs.map(|run| GlyphInstanceKey::for_run(request, run, glyph, phase));
    assert_ne!(keys[0], keys[1]);
    assert_ne!(keys[1], keys[2]);
    assert_eq!(keys[3], keys[4]);
    assert_eq!(keys[1], keys[5]);

    let masks = [0, 1, 2, 3].map(|i| {
        let cold = atlas
            .lookup_or_rasterize_at(runs[i], glyph, 1., phase, &unprotected)
            .unwrap();
        let warm = atlas
            .lookup_or_rasterize_at(runs[i], glyph, 1., phase, &unprotected)
            .unwrap();
        assert_eq!(cold.entry, warm.entry);
        assert_eq!(atlas.instance_entry(&keys[i]), Some(cold.entry));
        assert!(warm.bitmap.is_none());
        registry.glyph_instance_identity(keys[i].clone());
        cold.bitmap.unwrap()
    });
    let coverages = masks
        .each_ref()
        .map(|mask| mask.iter().map(|v| u64::from(*v)).sum::<u64>());
    assert!(
        coverages[0] < coverages[1] && coverages[1] < coverages[2],
        "{coverages:?}"
    );
    let explicit_middle = atlas
        .lookup_or_rasterize_at(runs[5], glyph, 1., phase, &unprotected)
        .unwrap();
    assert!(explicit_middle.bitmap.is_none());
    assert_eq!(atlas.instance_entry(&keys[1]), Some(explicit_middle.entry));
    let default = atlas
        .lookup_or_rasterize_at(runs[4], glyph, 1., phase, &unprotected)
        .unwrap();
    assert!(default.bitmap.is_none());
    assert_eq!(
        atlas.entry(request.cache_key(runs[3].font.id(), glyph, phase)),
        Some(default.entry)
    );
    assert_eq!(atlas.font_count(), 1);
    assert_eq!(atlas.counters().font_parser_cache_misses, 1);
    assert_eq!(registry.glyph_count(), 4);
    let light_identity = registry.glyph_instance_identity(keys[0].clone());
    assert_ne!(
        light_identity,
        registry.glyph_instance_identity(keys[1].clone())
    );

    atlas.set_max_pages(0, &unprotected);
    let mut slots = [Some(1_u32)];
    let mut revision = 0;
    assert_eq!(
        retire_glyph_page_resources(&mut atlas, &mut registry, &mut slots, &mut revision),
        1
    );
    assert_eq!(registry.glyph_count(), 0);
    assert!(keys.iter().all(|key| atlas.instance_entry(key).is_none()));

    atlas.set_max_pages(1, &unprotected);
    let fresh = atlas
        .lookup_or_rasterize_at(runs[0], glyph, 1., phase, &unprotected)
        .unwrap();
    assert_eq!(fresh.bitmap.as_ref(), Some(&masks[0]));
    assert_ne!(
        registry.glyph_instance_identity(keys[0].clone()),
        light_identity
    );
}
