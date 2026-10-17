use incular::prelude::{Code, Color, Modifiers};
use incular_devtools_ui::views::{
    Shortcut, ToolView, shortcut_for,
    theme::{self, DARK, LIGHT, Palette, Theme, contrast_ratio},
};

fn assert_contrast(name: &str, theme: &str, foreground: Color, background: Color, minimum: f64) {
    let ratio = contrast_ratio(foreground, background);
    assert!(
        ratio >= minimum,
        "{name} on its background in the {theme} theme has contrast {ratio:.2}, below {minimum}"
    );
}

fn check_palette(label: &str, palette: &Palette) {
    for surface in [palette.background, palette.surface, palette.surface_raised] {
        assert_contrast("text_primary", label, palette.text_primary, surface, 4.5);
        assert_contrast("text_muted", label, palette.text_muted, surface, 4.5);
        for (name, color) in [
            ("primary", palette.primary),
            ("danger", palette.danger),
            ("warning", palette.warning),
            ("violet", palette.violet),
            ("accent_text", palette.accent_text),
        ] {
            assert_contrast(name, label, color, surface, 3.0);
        }
    }
    for fill in [palette.control, palette.control_active] {
        assert_contrast("text_primary", label, palette.text_primary, fill, 4.5);
        assert_contrast("primary", label, palette.primary, fill, 3.0);
    }
    for fill in [palette.primary, palette.danger] {
        assert_contrast("on_accent", label, palette.on_accent, fill, 4.5);
    }
    // Chart phases must be visible against the plot surface and distinct.
    let phases = [
        palette.phase_build,
        palette.phase_layout,
        palette.phase_paint,
        palette.phase_semantics,
        palette.phase_composite,
        palette.phase_other,
        palette.gpu_color,
    ];
    for phase in phases {
        assert_contrast("phase colour", label, phase, palette.surface, 2.2);
    }
    for (index, a) in phases.iter().enumerate() {
        for b in &phases[index + 1..] {
            assert_ne!(a, b, "chart colours must be distinct in the {label} theme");
        }
    }
}

#[test]
fn both_palettes_meet_text_and_accent_contrast() {
    check_palette("dark", &DARK);
    check_palette("light", &LIGHT);
}

#[test]
fn light_palette_is_not_an_inversion_of_dark() {
    assert!(LIGHT.background.red > 200 && DARK.background.red < 60);
    assert_ne!(LIGHT.primary, DARK.primary);
    assert_eq!(LIGHT.surface, Color::rgba(255, 255, 255, 255));
}

#[test]
fn current_theme_selects_the_palette_used_by_accessors() {
    theme::set_current(Theme::Light);
    assert_eq!(theme::current(), Theme::Light);
    assert_eq!(theme::background(), LIGHT.background);
    assert_eq!(theme::text_primary(), LIGHT.text_primary);
    theme::set_current(Theme::Dark);
    assert_eq!(theme::background(), DARK.background);
    assert_eq!(Theme::Dark.palette(), &DARK);
    assert_eq!(Theme::Light.palette(), &LIGHT);
}

#[test]
fn theme_parsing_toggle_and_labels() {
    assert_eq!(Theme::parse("Light"), Some(Theme::Light));
    assert_eq!(Theme::parse(" dark "), Some(Theme::Dark));
    assert_eq!(Theme::parse("sepia"), None);
    assert_eq!(Theme::Dark.toggled(), Theme::Light);
    assert_eq!(Theme::Dark.switch_label(), "Light theme");
    assert_eq!(Theme::Light.switch_label(), "Dark theme");
}

#[test]
fn control_digits_select_views_in_tab_order() {
    let digits = [
        Code::Digit1,
        Code::Digit2,
        Code::Digit3,
        Code::Digit4,
        Code::Digit5,
        Code::Digit6,
        Code::Digit7,
        Code::Digit8,
        Code::Digit9,
        Code::Digit0,
    ];
    for (code, view) in digits.into_iter().zip(ToolView::ALL) {
        assert_eq!(
            shortcut_for(code, Modifiers::CONTROL),
            Some(Shortcut::View(view))
        );
    }
    assert_eq!(ToolView::Overview.shortcut_label(), "Ctrl+1");
    assert_eq!(ToolView::Application.shortcut_label(), "Ctrl+0");
}

#[test]
fn commands_require_control_and_ignore_plain_typing() {
    for code in [Code::Digit1, Code::KeyR, Code::KeyE, Code::PageDown] {
        assert_eq!(shortcut_for(code, Modifiers::empty()), None);
        assert_eq!(shortcut_for(code, Modifiers::SHIFT), None);
        assert_eq!(shortcut_for(code, Modifiers::ALT), None);
    }
    assert_eq!(
        shortcut_for(Code::KeyR, Modifiers::CONTROL),
        Some(Shortcut::Refresh)
    );
    assert_eq!(
        shortcut_for(Code::KeyE, Modifiers::CONTROL),
        Some(Shortcut::Export)
    );
    assert_eq!(
        shortcut_for(Code::KeyC, Modifiers::CONTROL | Modifiers::SHIFT),
        Some(Shortcut::PickWidget)
    );
    assert_eq!(shortcut_for(Code::KeyC, Modifiers::CONTROL), None);
    assert_eq!(
        shortcut_for(Code::PageDown, Modifiers::CONTROL),
        Some(Shortcut::NextView)
    );
    assert_eq!(
        shortcut_for(Code::PageUp, Modifiers::CONTROL),
        Some(Shortcut::PreviousView)
    );
}

#[test]
fn view_cycling_wraps_in_both_directions() {
    assert_eq!(ToolView::Application.cycled(1), ToolView::Overview);
    assert_eq!(ToolView::Overview.cycled(-1), ToolView::Application);
    assert_eq!(ToolView::Widgets.cycled(1), ToolView::Performance);
}
