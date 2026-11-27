use incular_core::{FlutterVersion, flutter_version};

const VERSION: FlutterVersion = flutter_version!();

#[test]
fn framework_version_reads_the_compile_time_definition() {
    assert_eq!(VERSION.version, option_env!("FLUTTER_VERSION"));
}

#[test]
fn channel_reads_the_compile_time_definition() {
    assert_eq!(VERSION.channel, option_env!("FLUTTER_CHANNEL"));
}

#[test]
fn git_url_reads_the_compile_time_definition() {
    assert_eq!(VERSION.git_url, option_env!("FLUTTER_GIT_URL"));
}

#[test]
fn framework_revision_reads_the_compile_time_definition() {
    assert_eq!(
        VERSION.framework_revision,
        option_env!("FLUTTER_FRAMEWORK_REVISION")
    );
}

#[test]
fn engine_revision_reads_the_compile_time_definition() {
    assert_eq!(
        VERSION.engine_revision,
        option_env!("FLUTTER_ENGINE_REVISION")
    );
}

#[test]
fn dart_version_reads_the_compile_time_definition() {
    assert_eq!(VERSION.dart_version, option_env!("FLUTTER_DART_VERSION"));
}

#[test]
fn missing_and_empty_definitions_remain_distinct() {
    if option_env!("INCULAR_TEST_FLUTTER_VERSION_FIXTURE") == Some("fixture") {
        assert_eq!(VERSION.version, Some("3.50.0"));
        assert_eq!(VERSION.channel, Some(""));
        assert_eq!(VERSION.git_url, None);
        assert_eq!(VERSION.framework_revision, Some("ab2912035e"));
        assert_eq!(VERSION.engine_revision, Some("engine-fixture"));
        assert_eq!(VERSION.dart_version, Some("dart-fixture"));
    }
}
