use incular_web::{
    BrowserRouteInformationProvider, FALLBACK_FONT_FILES, UrlStrategy, fallback_font_file,
};
use incular_widgets::{RouteInformation, RouteInformationProvider, RouteInformationReportingType};

#[test]
fn scripts_map_to_their_noto_fonts() {
    for (character, file) in [
        ('ا', "NotoSansArabic-Regular.ttf"),
        ('ש', "NotoSansHebrew-Regular.ttf"),
        ('क', "NotoSansDevanagari-Regular.ttf"),
        ('த', "NotoSansTamil-Regular.ttf"),
        ('ก', "NotoSansThai-Regular.ttf"),
        ('ა', "NotoSansGeorgian-Regular.ttf"),
        ('Ա', "NotoSansArmenian-Regular.ttf"),
        ('ሀ', "NotoSansEthiopic-Regular.ttf"),
        ('한', "NotoSansKR-Regular.otf"),
        ('か', "NotoSansJP-Regular.otf"),
        ('ㄅ', "NotoSansTC-Regular.otf"),
        ('😀', "NotoEmoji-Regular.ttf"),
        ('☀', "NotoEmoji-Regular.ttf"),
        ('∑', "NotoSansMath-Regular.ttf"),
        ('⬡', "NotoSansSymbols2-Regular.ttf"),
        ('ǅ', "NotoSans-Regular.ttf"),
    ] {
        assert_eq!(
            fallback_font_file(character, "en-US"),
            Some(file),
            "{character}"
        );
    }
}

#[test]
fn shared_ideographs_follow_the_page_language() {
    for (language, file) in [
        ("en", "NotoSansSC-Regular.otf"),
        ("zh-CN", "NotoSansSC-Regular.otf"),
        ("zh-TW", "NotoSansTC-Regular.otf"),
        ("zh-Hant-HK", "NotoSansTC-Regular.otf"),
        ("ja-JP", "NotoSansJP-Regular.otf"),
        ("ko", "NotoSansKR-Regular.otf"),
    ] {
        assert_eq!(fallback_font_file('字', language), Some(file), "{language}");
    }
}

#[test]
fn uncovered_code_points_request_nothing() {
    assert_eq!(fallback_font_file('\u{E000}', "en"), None);
    assert_eq!(fallback_font_file('\u{FE0F}', "en"), None);
    assert_eq!(fallback_font_file('\u{A000}', "en"), None);
}

#[test]
fn every_requested_file_is_listed_once() {
    let mut files = FALLBACK_FONT_FILES.to_vec();
    let count = files.len();
    files.sort_unstable();
    files.dedup();
    assert_eq!(files.len(), count);
    for code in (0..0x3_0000).filter_map(char::from_u32) {
        if let Some(file) = fallback_font_file(code, "en") {
            assert!(FALLBACK_FONT_FILES.contains(&file), "{file}");
        }
    }
}

#[test]
fn browser_routes_start_at_the_root_off_the_browser() {
    let provider = BrowserRouteInformationProvider::with_strategy(UrlStrategy::Path);
    assert_eq!(provider.strategy(), UrlStrategy::Path);
    assert_eq!(provider.value().location(), "/");
    provider
        .router_reports_new_route_information(
            RouteInformation::new("/settings"),
            RouteInformationReportingType::Navigate,
        )
        .expect("memory reports succeed");
    assert_eq!(provider.value().location(), "/settings");
    provider
        .router_reports_new_route_information(
            RouteInformation::new("/ignored"),
            RouteInformationReportingType::None,
        )
        .expect("unreported routes succeed");
    assert_eq!(provider.value().location(), "/settings");
}
