#![cfg(feature = "material")]

#[test]
fn neutral_licenses_do_not_shadow_the_existing_material_prelude() {
    use incular::material_prelude::*;
    use incular::prelude::*;

    let existing: LicenseEntry = LicenseEntry::with_line_breaks(["test-material"], "Legacy");
    LicenseRegistry::reset();
    LicenseRegistry::add_license(existing);
    assert_eq!(LicenseRegistry::packages(), ["test-material"]);
    LicenseRegistry::reset();

    let neutral: incular::core::licenses::LicenseEntry =
        incular::licenses::LicenseEntry::with_line_breaks(["test-core"], "Core");
    assert_eq!(neutral.paragraphs().next().unwrap().text, "Core");
    assert_eq!(Size::ZERO.width, 0.0);
}
