//! Metadata for an app compiled with a Flutter toolchain.

/// Compile-time metadata for the originating Flutter toolchain. No value is
/// fabricated for Rust-only apps, forks, or custom embedders without metadata.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FlutterVersion {
    pub version: Option<&'static str>,
    pub channel: Option<&'static str>,
    pub git_url: Option<&'static str>,
    pub framework_revision: Option<&'static str>,
    pub engine_revision: Option<&'static str>,
    pub dart_version: Option<&'static str>,
}

/// Reads the six Flutter build definitions at the application call site.
/// A missing definition is None; an explicitly empty definition is Some("").
#[macro_export]
macro_rules! flutter_version {
    () => {
        $crate::FlutterVersion {
            version: option_env!("FLUTTER_VERSION"),
            channel: option_env!("FLUTTER_CHANNEL"),
            git_url: option_env!("FLUTTER_GIT_URL"),
            framework_revision: option_env!("FLUTTER_FRAMEWORK_REVISION"),
            engine_revision: option_env!("FLUTTER_ENGINE_REVISION"),
            dart_version: option_env!("FLUTTER_DART_VERSION"),
        }
    };
}
