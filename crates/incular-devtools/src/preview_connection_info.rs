//! Local connection metadata for the generated widget preview scaffold.

use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

/// Host-owned metadata for the scaffold's already-authorized local
/// protocol connection. Generating it does not connect to a service.
/// Keep generated values in the local scaffold cache, not in source control.
pub struct PreviewConnectionInfo<'a> {
    /// Connection URI supplied by the current tool session.
    pub uri: &'a str,
    /// Name of the service the preview client calls.
    pub service: &'a str,
    /// Event stream name for scaffold updates.
    pub stream: &'a str,
    /// Root path of the project the scaffold is previewing.
    pub project_root: &'a Path,
}

/// The supplied root or generated file could not be stored as Rust source.
#[derive(Debug, thiserror::Error)]
pub enum PreviewConnectionInfoError {
    /// The project path cannot be represented in the generated Rust string.
    #[error("cannot encode a non-Unicode preview project root: {0:?}")]
    NonUnicodeRoot(PathBuf),
    /// A scaffold directory or generated file could not be written.
    #[error("cannot write preview connection metadata at {path}: {source}")]
    Io {
        /// Generated source path.
        path: PathBuf,
        /// Filesystem error from preparation or publication.
        #[source]
        source: io::Error,
    },
}

impl PreviewConnectionInfo<'_> {
    /// Generated module path within a native preview scaffold.
    pub const RELATIVE_PATH: &str = "src/dtd/dtd_connection_info.rs";

    /// Renders the constants as escaped Rust literals.
    pub fn render(&self) -> Result<String, PreviewConnectionInfoError> {
        let root = self
            .project_root
            .to_str()
            .ok_or_else(|| PreviewConnectionInfoError::NonUnicodeRoot(self.project_root.into()))?;
        let Self {
            uri,
            service,
            stream,
            ..
        } = self;
        Ok(format!(
            concat!(
                "// Generated preview connection metadata.\n",
                "pub const DTD_URI: &str = {:?};\n",
                "pub const WIDGET_PREVIEW_SERVICE: &str = {:?};\n",
                "pub const WIDGET_PREVIEW_SCAFFOLD_STREAM: &str = {:?};\n",
                "pub const PROJECT_ROOT_PATH: &str = {:?};\n",
            ),
            uri, service, stream, root,
        ))
    }

    /// Writes the complete generated module under this scaffold. Readers
    /// never observe a partially written source file when settings change.
    pub fn write(&self, scaffold: &Path) -> Result<PathBuf, PreviewConnectionInfoError> {
        let source = self.render()?;
        let path = scaffold.join(Self::RELATIVE_PATH);
        let parent = path.parent().expect("generated path has a parent");
        let failed = |source| PreviewConnectionInfoError::Io {
            path: path.clone(),
            source,
        };
        fs::create_dir_all(parent).map_err(&failed)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(&failed)?;
        temporary.write_all(source.as_bytes()).map_err(&failed)?;
        temporary.persist(&path).map_err(|e| failed(e.error))?;
        Ok(path)
    }
}
