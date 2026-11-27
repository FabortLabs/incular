use incular_config::Brightness;
use incular_core::Color;
use incular_widgets::widget_previews::ide_theme::{IdeTheme, IdeThemeQueryParams};

fn params(values: &[(&str, Option<&str>)]) -> IdeThemeQueryParams {
    IdeThemeQueryParams::new(
        values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.map(str::to_owned)))
            .collect(),
    )
}

#[test]
fn parses_background_foreground_and_explicit_dark_mode() {
    let query = params(&[
        ("backgroundColor", Some("#112233")),
        ("foregroundColor", Some("#112244")),
        ("theme", Some("dark")),
    ]);
    assert!(!query.params().is_empty());
    assert_eq!(
        query.background_color(),
        Some(Color::rgba(0x11, 0x22, 0x33, 255))
    );
    assert_eq!(
        query.foreground_color(),
        Some(Color::rgba(0x11, 0x22, 0x44, 255))
    );
    assert!(query.dark_mode());
    assert!(query.to_theme().ide_specified_theme());
}

#[test]
fn malformed_colors_do_not_change_the_chosen_brightness() {
    let query = params(&[
        ("backgroundColor", Some("badcolor")),
        ("foregroundColor", Some("badcolor")),
        ("theme", Some("dark")),
    ]);
    assert!(!query.params().is_empty());
    assert_eq!(query.background_color(), None);
    assert_eq!(query.foreground_color(), None);
    assert!(query.dark_mode());
}

#[test]
fn unknown_params_do_not_affect_the_preview() {
    let query = params(&[("fontSize", Some("50")), ("theme", Some("dark"))]);
    assert!(query.dark_mode());
    assert_eq!(query.background_color(), None);
    assert_eq!(query.foreground_color(), None);
    assert_eq!(
        query.params().get("fontSize").unwrap().as_deref(),
        Some("50")
    );
}

#[test]
fn empty_or_null_params_keep_the_dark_default_and_no_color_override() {
    let query = params(&[]);
    assert!(query.params().is_empty());
    assert_eq!(query.background_color(), None);
    assert_eq!(query.foreground_color(), None);
    assert!(query.dark_mode());
    let nulls = params(&[("theme", None), ("backgroundColor", None)]);
    assert!(nulls.dark_mode());
    assert_eq!(nulls.background_color(), None);
    assert_eq!(IdeTheme::default().brightness(), Brightness::Dark);
    assert!(!IdeTheme::default().ide_specified_theme());
}

#[test]
fn without_an_override_both_modes_keep_the_supplied_surface() {
    let ide = IdeTheme::default();
    for (mode, fallback) in [
        (Brightness::Dark, Color::rgba(0x15, 0x15, 0x16, 255)),
        (Brightness::Light, Color::rgba(0xfa, 0xfa, 0xfe, 255)),
    ] {
        assert_eq!(ide.background_for(mode, fallback), fallback);
    }
}

#[test]
fn a_white_or_black_override_keeps_its_alpha_when_it_matches_the_mode() {
    let white = Color::rgba(255, 255, 255, 179);
    for (mode, color) in [(Brightness::Light, white), (Brightness::Dark, Color::BLACK)] {
        let ide = IdeTheme {
            background_color: Some(color),
            ..IdeTheme::default()
        };
        assert_eq!(ide.background_for(mode, Color::TRANSPARENT), color);
    }
}

#[test]
fn an_intermediate_background_is_rejected_by_both_modes() {
    let ide = IdeTheme {
        background_color: Some(Color::rgba(0xff, 0x98, 0, 255)),
        ..IdeTheme::default()
    };
    assert_eq!(
        ide.background_for(Brightness::Dark, Color::BLACK),
        Color::BLACK
    );
    assert_eq!(
        ide.background_for(Brightness::Light, Color::WHITE),
        Color::WHITE
    );
}

#[test]
fn css_alpha_and_exact_luminance_boundaries_match_the_preview_host() {
    let query = params(&[
        ("backgroundColor", Some("%23AbC4")),
        ("foregroundColor", Some("#A0b1c280")),
        ("theme", Some("light")),
    ]);
    assert_eq!(
        query.background_color(),
        Some(Color::rgba(0xaa, 0xbb, 0xcc, 0x44))
    );
    assert_eq!(
        query.foreground_color(),
        Some(Color::rgba(0xa0, 0xb1, 0xc2, 0x80))
    );
    assert_eq!(query.to_theme().brightness(), Brightness::Light);
    assert!(params(&[("theme", Some("Light"))]).dark_mode());

    for (mode, channel, accepted) in [
        (Brightness::Dark, 97, true),
        (Brightness::Dark, 98, false),
        (Brightness::Light, 241, false),
        (Brightness::Light, 242, true),
    ] {
        let color = Color::rgba(channel, channel, channel, 127);
        let theme = IdeTheme {
            background_color: Some(color),
            ..IdeTheme::default()
        };
        let chosen = theme.background_for(mode, Color::TRANSPARENT);
        assert_eq!(chosen, if accepted { color } else { Color::TRANSPARENT });
    }
}
