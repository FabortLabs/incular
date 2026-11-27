#![cfg(debug_assertions)]

use incular_core::{ChangeImpact, Invalidation, Lerp};
use incular_text::{TextStyle, TextStyleApply};

fn label(style: &TextStyle) -> Option<&str> {
    style.debug_label.as_deref()
}

#[test]
fn text_style_diagnostics_track_copy_merge_apply_and_lerp() {
    let unknown = TextStyle::new();
    let foo = TextStyle::new().font_size(1.).debug_label("foo");
    let bar = TextStyle::new().font_size(2.).debug_label("bar");
    let baz = TextStyle::new().font_size(3.).debug_label("baz");
    assert_eq!(label(&unknown), None);
    assert_eq!(unknown.to_string(), "TextStyle(<all styles inherited>)");
    assert_eq!(label(&unknown.copy_with(std::convert::identity)), None);
    assert_eq!(
        label(&unknown.copy_with(|copy| copy.debug_label("123"))),
        Some("123")
    );
    assert_eq!(
        label(&unknown.apply_options(TextStyleApply::default())),
        None
    );
    assert_eq!(label(&foo), Some("foo"));
    assert_eq!(
        foo.to_string(),
        "TextStyle(debugLabel: foo, inherit: true, size: 1.0)"
    );
    assert_eq!(label(&foo.merge(&bar)), Some("(foo).merge(bar)"));
    assert_eq!(
        label(&foo.merge(&bar).merge(&baz)),
        Some("((foo).merge(bar)).merge(baz)")
    );
    assert_eq!(
        label(&foo.copy_with(std::convert::identity)),
        Some("(foo).copyWith")
    );
    assert_eq!(
        label(&foo.apply_options(TextStyleApply::default())),
        Some("(foo).apply")
    );
    assert_eq!(label(&foo.lerp(&bar, 0.5)), Some("lerp(foo ⎯0.5→ bar)"));
    assert_eq!(
        label(
            &foo.merge(&bar)
                .lerp(&baz, 0.51)
                .copy_with(std::convert::identity)
        ),
        Some("(lerp((foo).merge(bar) ⎯0.5→ baz)).copyWith")
    );
}

#[test]
fn diagnostic_provenance_does_not_change_rendered_style_or_invalidation() {
    let first = TextStyle::new().font_size(14.).debug_label("original");
    let changed_label = first.clone().debug_label("provenance changed");
    assert_eq!(first, changed_label);
    assert_eq!(first.change_impact(&changed_label), ChangeImpact::None);
    assert_eq!(first.invalidation(&changed_label), Invalidation::NONE);
    assert_ne!(first.to_string(), changed_label.to_string());
    assert_eq!(
        label(&first.merge(&TextStyle::new())),
        Some("(original).merge(unknown)")
    );
    assert_eq!(
        label(&first.merge(&changed_label.clone().inherit(false))),
        Some("provenance changed")
    );
    assert_ne!(first, changed_label.font_size(12.));
}
