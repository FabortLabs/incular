use incular_core::{AppBuildVersion, app_build_version};

const EMBEDDED: AppBuildVersion = app_build_version!();
const MANIFEST: AppBuildVersion = app_build_version!("1.4.2-rc.1+45");

#[test]
fn application_name_and_number_follow_the_compile_time_declarations() {
    assert_eq!(EMBEDDED.name, option_env!("INCULAR_BUILD_NAME"));
    assert_eq!(EMBEDDED.number, option_env!("INCULAR_BUILD_NUMBER"));
    assert_eq!(
        MANIFEST.name,
        option_env!("INCULAR_BUILD_NAME").or(Some("1.4.2-rc.1"))
    );
    assert_eq!(
        MANIFEST.number,
        option_env!("INCULAR_BUILD_NUMBER").or(Some("45"))
    );
}

#[test]
fn overrides_are_independent_and_empty_is_distinct_from_missing() {
    for (manifest, name, number, expected) in [
        (None, None, None, (None, None)),
        (Some("1.2.3"), None, None, (Some("1.2.3"), None)),
        (Some("1.2.3+7"), None, None, (Some("1.2.3"), Some("7"))),
        (None, Some("x"), None, (Some("x"), None)),
        (None, None, Some("3"), (None, Some("3"))),
        (Some("1.2.3+7"), Some("x"), None, (Some("x"), Some("7"))),
        (Some("1.2.3+7"), None, Some(""), (Some("1.2.3"), Some(""))),
        (
            Some("1.2.3+7"),
            Some(""),
            Some("0001"),
            (Some(""), Some("0001")),
        ),
    ] {
        let version = AppBuildVersion::resolve(manifest, name, number);
        assert_eq!((version.name, version.number), expected);
    }
}
