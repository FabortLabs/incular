//! Versioned cache metadata for a design-system-neutral widget preview
//! scaffold. The host supplies the location and Cargo workspace members.

use std::{
    collections::BTreeMap,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::preview_changes::PreviewChangeRouter;

/// Filesystem-backed metadata for an existing or future preview scaffold.
/// This manages invalidation only; it does not generate or launch a host.
#[derive(Clone, Debug)]
pub struct PreviewManifest {
    scaffold: PathBuf,
    sdk_version: String,
    root_manifest: PathBuf,
    members: Vec<PathBuf>,
}

/// Work the host must perform before opening an up-to-date preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewRegeneration {
    /// Scaffold and source dependency metadata are both current.
    None,
    /// Keep the scaffold but regenerate its dependency metadata.
    ProjectMetadata,
    /// Regenerate the scaffold itself.
    Scaffold(PreviewScaffoldInvalidation),
}

/// The durable metadata reason for rebuilding a preview scaffold.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PreviewScaffoldInvalidation {
    /// The generated directory does not exist yet.
    #[error("the widget preview scaffold is missing")]
    MissingScaffold,
    /// The manifest is absent, incomplete, or not valid JSON.
    #[error("the widget preview manifest is missing or invalid")]
    InvalidManifest,
    /// The on-disk manifest was written by a different schema.
    #[error("the widget preview manifest format changed")]
    ManifestVersionChanged,
    /// The scaffold was generated for a different SDK identity.
    #[error("the widget preview scaffold was generated with SDK {generated}, not {current}")]
    SdkVersionChanged {
        /// SDK identity originally recorded.
        generated: String,
        /// SDK identity in use now.
        current: String,
    },
}

/// A file or Cargo manifest error that cannot be repaired by treating
/// the preview cache as stale.
#[derive(Debug, thiserror::Error)]
pub enum PreviewManifestError {
    /// A manifest read or atomic write failed.
    #[error("preview metadata I/O at {path}: {source}")]
    Io {
        /// The affected file or scaffold directory.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// A Cargo manifest was present but contained invalid TOML.
    #[error("invalid Cargo preview input manifest at {path}: {source}")]
    Syntax {
        /// Project manifest with the invalid syntax.
        path: PathBuf,
        /// Underlying TOML parser error.
        #[source]
        source: toml::de::Error,
    },
}

impl PreviewManifest {
    /// Name of the generated file in the scaffold's directory.
    pub const FILE_NAME: &str = "preview_manifest.json";
    /// Incular's format. This stores Cargo files, not Flutter pubspecs.
    pub const VERSION: &str = "1";
    const PROJECT_HASHES: &str = "project-manifest-hashes";

    /// Binds the host's scaffold to an SDK identity and root Cargo.toml.
    #[must_use]
    pub fn new(
        scaffold: impl Into<PathBuf>,
        sdk_version: impl Into<String>,
        root_manifest: impl Into<PathBuf>,
    ) -> Self {
        Self {
            scaffold: scaffold.into(),
            sdk_version: sdk_version.into(),
            root_manifest: root_manifest.into(),
            members: Vec::new(),
        }
    }

    /// Sets the current workspace member manifests (Cargo.toml files).
    /// Missing members are ignored; the root always has an identity.
    #[must_use]
    pub fn workspace_members(mut self, members: impl IntoIterator<Item = PathBuf>) -> Self {
        self.members = members.into_iter().collect();
        self
    }

    /// Creates a change router using the same workspace manifests and
    /// generated scaffold path as this cache. It can be configured with
    /// additional Cargo target paths before a host starts watching.
    pub fn change_router(&self) -> io::Result<PreviewChangeRouter> {
        PreviewChangeRouter::for_workspace(&self.scaffold, &self.root_manifest, &self.members)
    }

    /// The metadata filename within this host's scaffold.
    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.scaffold.join(Self::FILE_NAME)
    }

    /// Determines whether the scaffold needs a full rebuild, only a
    /// dependency-metadata refresh, or neither.
    pub fn regeneration(&self) -> Result<PreviewRegeneration, PreviewManifestError> {
        use PreviewRegeneration::{None as UpToDate, ProjectMetadata, Scaffold};
        use PreviewScaffoldInvalidation::{
            InvalidManifest, ManifestVersionChanged, MissingScaffold, SdkVersionChanged,
        };
        let scaffold_exists = self
            .scaffold
            .try_exists()
            .map_err(|source| Self::io(&self.scaffold, source))?;
        if !scaffold_exists {
            return Ok(Scaffold(MissingScaffold));
        }
        let Some(manifest) = self.read_manifest()? else {
            return Ok(Scaffold(InvalidManifest));
        };
        let (Some(version), Some(sdk)) = (
            manifest.get("version").and_then(Value::as_str),
            manifest.get("sdk-version").and_then(Value::as_str),
        ) else {
            return Ok(Scaffold(InvalidManifest));
        };
        if version != Self::VERSION {
            return Ok(Scaffold(ManifestVersionChanged));
        }
        if sdk != self.sdk_version {
            return Ok(Scaffold(SdkVersionChanged {
                generated: sdk.into(),
                current: self.sdk_version.clone(),
            }));
        }
        let hashes = json!(self.project_hashes()?);
        Ok(if manifest.get(Self::PROJECT_HASHES) == Some(&hashes) {
            UpToDate
        } else {
            ProjectMetadata
        })
    }

    /// Records a scaffold generation after the host has rendered its files.
    pub fn generate(&self) -> Result<(), PreviewManifestError> {
        let manifest = json!({
            "version": Self::VERSION,
            "sdk-version": self.sdk_version,
            Self::PROJECT_HASHES: self.project_hashes()?,
        });
        self.write_manifest(&manifest)
    }

    /// Records a dependency-only refresh without hiding an SDK mismatch.
    /// Returns false if there is no compatible scaffold to refresh.
    pub fn update_project_hashes(&self) -> Result<bool, PreviewManifestError> {
        let Some(mut manifest) = self.read_manifest()? else {
            return Ok(false);
        };
        if manifest.get("version").and_then(Value::as_str) != Some(Self::VERSION)
            || manifest.get("sdk-version").and_then(Value::as_str) != Some(&self.sdk_version)
        {
            return Ok(false);
        }
        manifest[Self::PROJECT_HASHES] = json!(self.project_hashes()?);
        self.write_manifest(&manifest)?;
        Ok(true)
    }

    fn read_manifest(&self) -> Result<Option<Value>, PreviewManifestError> {
        let path = self.path();
        match fs::read(&path) {
            Ok(bytes) => Ok(serde_json::from_slice::<Value>(&bytes)
                .ok()
                .filter(Value::is_object)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(Self::io(&path, source)),
        }
    }

    fn write_manifest(&self, value: &Value) -> Result<(), PreviewManifestError> {
        fs::create_dir_all(&self.scaffold).map_err(|source| Self::io(&self.scaffold, source))?;
        let path = self.path();
        let mut output = tempfile::NamedTempFile::new_in(&self.scaffold)
            .map_err(|source| Self::io(&path, source))?;
        serde_json::to_writer(output.as_file_mut(), value)
            .map_err(|error| Self::io(&path, io::Error::other(error)))?;
        output
            .flush()
            .and_then(|()| output.as_file().sync_all())
            .map_err(|source| Self::io(&path, source))?;
        output
            .persist(&path)
            .map_err(|error| Self::io(&path, error.error))?;
        Ok(())
    }

    fn project_hashes(&self) -> Result<BTreeMap<String, String>, PreviewManifestError> {
        let mut hashes = BTreeMap::new();
        for (root, path) in std::iter::once((true, &self.root_manifest))
            .chain(self.members.iter().map(|path| (false, path)))
        {
            let source = match fs::read_to_string(path) {
                Ok(source) => source,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if root {
                        String::new()
                    } else {
                        continue;
                    }
                }
                Err(source) => return Err(Self::io(path, source)),
            };
            let manifest: toml::Table =
                source
                    .parse()
                    .map_err(|source| PreviewManifestError::Syntax {
                        path: path.clone(),
                        source,
                    })?;
            let path = std::path::absolute(path).map_err(|source| Self::io(path, source))?;
            let key = path.to_str().ok_or_else(|| {
                Self::io(
                    &path,
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "preview manifest path is not UTF-8",
                    ),
                )
            })?;
            // Re-serialize parsed TOML to ignore comments and formatting.
            // FNV-1a is a stable versioned cache fingerprint, not an
            // identity proof or a security digest.
            let hash = manifest
                .to_string()
                .bytes()
                .fold(0xCBF2_9CE4_8422_2325u64, |hash, byte| {
                    (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01B3)
                });
            hashes.insert(key.to_owned(), format!("{hash:016x}"));
        }
        Ok(hashes)
    }

    fn io(path: &Path, source: io::Error) -> PreviewManifestError {
        PreviewManifestError::Io {
            path: path.to_owned(),
            source,
        }
    }
}
