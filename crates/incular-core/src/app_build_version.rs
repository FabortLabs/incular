//! Compile-time application version metadata for Flutter's services API.

/// Version of the application that is executing, not the framework version
/// or a version fetched from its server.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AppBuildVersion {
    pub name: Option<&'static str>,
    pub number: Option<&'static str>,
}

impl AppBuildVersion {
    /// Uses each explicit setting when present and otherwise uses that part
    /// of the manifest's `name+number`. An explicit empty string is distinct
    /// from an omitted setting.
    #[must_use]
    pub const fn resolve(
        manifest: Option<&'static str>,
        build_name: Option<&'static str>,
        build_number: Option<&'static str>,
    ) -> Self {
        let (manifest_name, manifest_number) = match manifest {
            Some(version) => split_version(version),
            None => (None, None),
        };
        Self {
            name: match build_name {
                None => manifest_name,
                explicit => explicit,
            },
            number: match build_number {
                None => manifest_number,
                explicit => explicit,
            },
        }
    }
}

const fn split_version(version: &'static str) -> (Option<&'static str>, Option<&'static str>) {
    let bytes = version.as_bytes();
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position] == b'+' {
            // An ASCII plus cannot lie inside a UTF-8 code unit sequence.
            let (name, plus_number) = version.split_at(position);
            let (_, number) = plus_number.split_at(1);
            return (Some(name), Some(number));
        }
        position += 1;
    }
    (Some(version), None)
}

/// Reads build metadata at the application call site, never at framework
/// compilation or runtime. `INCULAR_BUILD_NAME` and `INCULAR_BUILD_NUMBER`
/// override their respective parts. Without settings, both are absent.
///
/// An optional argument supplies a manifest version fallback. A Cargo app
/// can use `const VERSION: AppBuildVersion = app_build_version!(env!("CARGO_PKG_VERSION"));`
/// if it wants its Cargo package version to supply the defaults.
#[macro_export]
macro_rules! app_build_version {
    () => {
        $crate::AppBuildVersion::resolve(
            None,
            option_env!("INCULAR_BUILD_NAME"),
            option_env!("INCULAR_BUILD_NUMBER"),
        )
    };
    ($manifest_version:expr $(,)?) => {
        $crate::AppBuildVersion::resolve(
            Some($manifest_version),
            option_env!("INCULAR_BUILD_NAME"),
            option_env!("INCULAR_BUILD_NUMBER"),
        )
    };
}
