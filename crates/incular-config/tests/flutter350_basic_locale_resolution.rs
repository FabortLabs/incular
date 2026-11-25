use incular_config::{Locale, LocaleResolver};

fn locales(tags: &[&str]) -> Vec<Locale> {
    tags.iter().map(|tag| tag.parse().unwrap()).collect()
}

fn assert_selection(preferred: &[&str], supported: &[&str], expected: &str) {
    assert_eq!(
        LocaleResolver::resolve_basic(&locales(preferred), &locales(supported)),
        Some(expected.parse().unwrap()),
        "preferred: {preferred:?}, supported: {supported:?}",
    );
}

#[test]
fn flutter_basic_locale_list_resolution() {
    // All six original cases in widgets/app_test.dart:708.
    assert_selection(&["zh", "un", "en"], &["en"], "en");
    assert_selection(&["en", "en-US"], &["en-US"], "en-US");
    assert_selection(&["zh-Hant-HK"], &["zh-HK", "zh-Hant"], "zh-Hant");
    assert_selection(&["zh", "zh-Hant-TW"], &["zh-Hant-TW"], "zh-Hant-TW");
    assert_selection(&["en", "ar-TN"], &["fr-TN"], "fr-TN");
    assert_selection(&["tn"], &["zh", "un", "en"], "zh");
}

#[test]
fn deferred_language_region_order_and_first_supported_ties() {
    assert_selection(
        &["zh", "zh-Hant-TW"],
        &["zh-CN", "zh-Hant-TW"],
        "zh-Hant-TW",
    );
    assert_selection(&["zh", "en-US"], &["zh-CN", "en-US"], "zh-CN");
    assert_selection(&["de", "zh", "en-US"], &["zh-CN", "en-US"], "en-US");
    assert_selection(&["de", "zh", "it", "en-US"], &["en-US", "zh-CN"], "zh-CN");
    assert_selection(&["de", "ar-TN"], &["en-US", "fr-TN"], "fr-TN");
    assert_selection(&["zh-Hant-HK"], &["zh-Hant-TW", "zh-Hant-MO"], "zh-Hant-TW");
    assert_selection(&[], &["fr-CA", "en-US"], "fr-CA");
    assert_eq!(LocaleResolver::resolve_basic(&locales(&["en"]), &[]), None);
}
