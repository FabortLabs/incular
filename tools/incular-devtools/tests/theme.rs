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
            ("success", palette.success),
            ("focus_ring", palette.focus_ring),
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

#[test]
fn accent_is_brand_violet_while_success_stays_green() {
    for (label, palette) in [("dark", &DARK), ("light", &LIGHT)] {
        let accent = palette.primary;
        assert!(
            accent.blue > accent.red && accent.red > accent.green,
            "{label} accent should lean violet: {accent:?}"
        );
        let success = palette.success;
        assert!(
            success.green > success.red && success.green > success.blue,
            "{label} success should stay green: {success:?}"
        );
    }
}

#[test]
fn interaction_states_are_ordered_and_visible_in_both_themes() {
    for (label, palette) in [("dark", &DARK), ("light", &LIGHT)] {
        // Pressed is darker (more opaque overlay) than hovered, and both are
        // translucent so they read on any fill.
        assert!(
            palette.hover_overlay.alpha > 0
                && palette.pressed_overlay.alpha > palette.hover_overlay.alpha
                && palette.pressed_overlay.alpha < 128,
            "{label} overlays"
        );
        // A disabled label is clearly more muted than a muted one yet differs
        // from the surface it sits on.
        assert_ne!(palette.text_disabled, palette.text_muted);
        assert!(
            contrast_ratio(palette.text_disabled, palette.surface)
                < contrast_ratio(palette.text_muted, palette.surface),
            "{label} disabled text must be quieter than muted text"
        );
        assert!(
            contrast_ratio(palette.text_disabled, palette.surface) > 1.5,
            "{label} disabled text must stay legible"
        );
        // The focus ring must be distinct from the selected fill it can overlap.
        assert!(contrast_ratio(palette.focus_ring, palette.control_active) >= 3.0);
        assert!(contrast_ratio(palette.focus_ring, palette.control) >= 3.0);
    }
}
