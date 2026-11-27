use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

use incular_text::{FontFamily, TextStyle, TextStyleApply};

#[test]
fn copies_and_applications_replace_the_package_without_duplicating_it() {
    let bare = TextStyle::new().font_family("fontFamily");
    let foo = TextStyle::new().packaged_font_family("foo", "fontFamily");
    let bar = TextStyle::new().packaged_font_family("bar", "fontFamily");

    assert_ne!(foo, bar);
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    assert_ne!(hasher.hash_one(&foo), hasher.hash_one(&bar));
    assert_eq!(bare.family.name(), "fontFamily");
    assert_eq!(
        bare.copy_with(|style| style.font_package("bar"))
            .family
            .name(),
        "packages/bar/fontFamily"
    );
    assert_eq!(foo.family.name(), "packages/foo/fontFamily");
    assert_eq!(
        foo.copy_with(|style| style.font_package("bar"))
            .family
            .name(),
        "packages/bar/fontFamily"
    );
    assert_eq!(
        TextStyle::new().merge(&bar).family.name(),
        "packages/bar/fontFamily"
    );
    for (prior, package) in [(TextStyle::new(), "foo"), (foo, "bar")] {
        let applied = prior.apply_options(TextStyleApply {
            font_family: Some(FontFamily::named("fontFamily")),
            font_package: Some(package),
            ..TextStyleApply::default()
        });
        assert_eq!(
            applied.family.name(),
            format!("packages/{package}/fontFamily")
        );
    }
}

#[test]
fn requalifying_keeps_fallback_order_and_does_not_materialize_defaults() {
    let style = TextStyle::new()
        .packaged_fallback_families("old", ["One", "Two"])
        .copy_with(|style| style.font_package("new"));
    assert!(!style.has_explicit_family);
    assert!(style.has_explicit_fallback_families);
    assert_eq!(
        style
            .fallback_families
            .iter()
            .map(FontFamily::name)
            .collect::<Vec<_>>(),
        ["packages/new/One", "packages/new/Two"]
    );
    let empty = TextStyle::new()
        .fallback_families(std::iter::empty::<&str>())
        .font_package("new");
    assert!(empty.has_explicit_fallback_families);
    assert!(empty.fallback_families.is_empty());
    assert!(
        !TextStyle::new()
            .font_package("new")
            .has_explicit_fallback_families
    );
}
