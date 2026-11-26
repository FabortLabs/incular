//! Flutter's package-qualified font styles and actual named face selection.

use incular_core::Lerp;
use incular_text::{FontFamily, TextAlign, TextEngine, TextStyle, register_application_font_as};
use std::sync::Arc;

const INTER: &[u8] = include_bytes!("../../../harness/flutter/fonts/Inter-Regular.ttf");

fn named(style: &TextStyle) -> Vec<&str> {
    style
        .fallback_families
        .iter()
        .map(FontFamily::name)
        .collect()
}

#[test]
fn copying_and_applying_package_fallbacks_does_not_prefix_them_again() {
    let ordinary = TextStyle::new().font_family("test");
    assert_eq!(ordinary.family.name(), "test");
    let primary = TextStyle::new().packaged_font_family("p", "test");
    assert_eq!(primary.family.name(), "packages/p/test");
    let fallback = TextStyle::new().packaged_fallback_families("p", ["test", "test2"]);
    assert!(!TextStyle::new().has_explicit_fallback_families);
    let cleared = TextStyle::new().packaged_fallback_families("p", [] as [&str; 0]);
    assert!(cleared.has_explicit_fallback_families && cleared.fallback_families.is_empty());
    for copy in [
        fallback.clone(),
        fallback.apply(None, None, None, None, None, None),
    ] {
        assert_eq!(named(&copy), ["packages/p/test", "packages/p/test2"]);
        assert_eq!(copy, fallback);
    }
}

#[test]
fn package_names_survive_merge_and_animation_endpoints() {
    let a = TextStyle::new()
        .packaged_font_family("p", "font1")
        .packaged_fallback_families("p", ["fallback1"]);
    let b = TextStyle::new()
        .packaged_font_family("p", "font2")
        .packaged_fallback_families("p", ["fallback2"]);
    for (style, primary, fallback) in [
        (
            TextStyle::new().merge(&a),
            "packages/p/font1",
            "packages/p/fallback1",
        ),
        (a.lerp(&b, 0.), "packages/p/font1", "packages/p/fallback1"),
        (a.lerp(&b, 1.), "packages/p/font2", "packages/p/fallback2"),
    ] {
        assert_eq!(style.family.name(), primary);
        assert_eq!(named(&style), [fallback]);
    }
}

#[test]
fn fallback_order_and_empty_override_are_preserved() {
    let original = TextStyle::new()
        .font_family("foo")
        .fallback_families(["Roboto", "test"]);
    assert_eq!(named(&original), ["Roboto", "test"]);
    let renamed = original.clone().font_family("bar");
    assert_eq!(renamed.family.name(), "bar");
    assert_eq!(named(&renamed), ["Roboto", "test"]);
    let changed = renamed.fallback_families(["Banana"]);
    assert_eq!(named(&changed), ["Banana"]);
    let cleared = original.merge(&TextStyle::new().fallback_families([] as [&str; 0]));
    assert!(cleared.has_explicit_fallback_families);
    assert!(cleared.fallback_families.is_empty());
}

#[test]
fn a_late_package_alias_replaces_cached_fallback_and_is_used_in_both_font_slots() {
    let family = FontFamily::packaged("audit_pack", "Body");
    let primary = TextStyle::new().family(family.clone()).size(23.);
    let fallback = TextStyle::new()
        .packaged_font_family("audit_pack", "Unregistered")
        .packaged_fallback_families("audit_pack", ["Missing", "Body"])
        .size(23.);
    let mut first = TextEngine::new();
    let old = first.layout("iiiiiiii", &primary, None, TextAlign::Start);
    let generation = first.font_database_generation();

    // The trailer distinguishes these bytes even on a host that already
    // has the unmodified Inter file. TrueType ignores bytes beyond its tables.
    let mut identifiable = INTER.to_vec();
    identifiable.extend_from_slice(b"incular named test fixture");
    register_application_font_as(family.name(), identifiable.clone());

    let with_alias = first.layout("iiiiiiii", &primary, None, TextAlign::Start);
    let through_fallback = first.layout("ΨΨΨ", &fallback, None, TextAlign::Start);
    let primary_greek = first.layout("ΨΨΨ", &primary, None, TextAlign::Start);
    let shaped_font = &with_alias.lines[0].runs[0].font;
    assert!(first.font_database_generation() > generation);
    assert!(!Arc::ptr_eq(&old, &with_alias));
    assert_eq!(shaped_font.bytes().as_ref(), identifiable);
    assert_eq!(
        through_fallback.lines[0].runs[0].font.id(),
        shaped_font.id()
    );
    assert_eq!(through_fallback.metrics.size, primary_greek.metrics.size);

    let alternate = FontFamily::packaged("different_pack", "Body");
    let mut second = TextEngine::new();
    assert!(second.register_font_bytes_as(alternate.name(), INTER));
    assert!(!second.register_font_bytes_as(alternate.name(), b"invalid font".as_slice()));
    let other = second.layout(
        "iiiiiiii",
        &TextStyle::new().family(alternate).size(23.),
        None,
        TextAlign::Start,
    );
    assert_eq!(other.lines[0].runs[0].font.bytes().as_ref(), INTER);
}
