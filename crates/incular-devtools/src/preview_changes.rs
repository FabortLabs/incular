//! Routes native watcher events to preview source or dependency invalidation.

use std::{
    collections::BTreeSet,
    env, io,
    path::{Component, Path, PathBuf},
};

/// One preview invalidation delivered by a host file watcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewChange {
    /// Invalidate the registered Rust source context at this file.
    Source(PathBuf),
    /// Refresh dependency metadata from this workspace manifest.
    Manifest(PathBuf),
}

/// Scope of files that may affect a preview. This handles both new paths and
/// deletions without reading the changed file. A host supplies actual watch
/// events; constructing a router does not start a watcher.
#[derive(Clone, Debug)]
pub struct PreviewChangeRouter {
    root: PathBuf,
    manifests: BTreeSet<PathBuf>,
    sources: BTreeSet<PathBuf>,
    source_dirs: BTreeSet<PathBuf>,
    excluded: BTreeSet<PathBuf>,
}

impl PreviewChangeRouter {
    pub(crate) fn for_workspace(
        scaffold: &Path,
        root_manifest: &Path,
        members: &[PathBuf],
    ) -> io::Result<Self> {
        let cwd = env::current_dir()?;
        let root_manifest = Self::absolute(&cwd, root_manifest)?;
        let root = root_manifest
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "manifest has no parent"))?
            .to_owned();
        let mut router = Self {
            root,
            manifests: BTreeSet::new(),
            sources: BTreeSet::new(),
            source_dirs: BTreeSet::new(),
            excluded: BTreeSet::from([Self::absolute(&cwd, scaffold)?]),
        };
        for manifest in std::iter::once(Ok(root_manifest))
            .chain(members.iter().map(|path| Self::absolute(&cwd, path)))
        {
            router.add_package(manifest?)?;
        }
        Ok(router)
    }

    fn add_package(&mut self, manifest: PathBuf) -> io::Result<()> {
        let root = manifest
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "manifest has no parent"))?;
        self.source_dirs.extend(
            ["src", "examples", "tests", "benches"]
                .into_iter()
                .map(|dir| root.join(dir)),
        );
        self.sources.insert(root.join("build.rs"));
        self.excluded
            .extend([root.join("target"), root.join(".git")]);
        self.manifests.insert(manifest);
        Ok(())
    }

    /// Registers a source context for an explicitly configured Cargo target
    /// directory outside the conventional package source directories.
    pub fn include_source_directory(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        self.source_dirs
            .insert(Self::absolute(&self.root, path.as_ref())?);
        Ok(())
    }

    /// Registers an individual source (for example a custom build script).
    pub fn include_source_file(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        self.sources
            .insert(Self::absolute(&self.root, path.as_ref())?);
        Ok(())
    }

    /// Excludes another generated directory, such as a custom target dir.
    /// Exclusions take priority even within a registered source context.
    pub fn exclude_directory(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        self.excluded
            .insert(Self::absolute(&self.root, path.as_ref())?);
        Ok(())
    }

    /// Maps an event into the corresponding invalidation. Relative watcher
    /// paths are interpreted from the workspace root, not the caller's
    /// current directory. The router and watcher should use the same path
    /// namespace: paths are normalized lexically, not resolved through
    /// symlinks on every event.
    #[must_use]
    pub fn route(&self, path: impl AsRef<Path>) -> Option<PreviewChange> {
        let path = Self::absolute(&self.root, path.as_ref()).ok()?;
        if self.excluded.iter().any(|dir| path.starts_with(dir)) {
            return None;
        }
        if self.manifests.contains(&path) {
            return Some(PreviewChange::Manifest(path));
        }
        if self.sources.contains(&path)
            || path.extension().is_some_and(|extension| extension == "rs")
                && self.source_dirs.iter().any(|dir| path.starts_with(dir))
        {
            return Some(PreviewChange::Source(path));
        }
        None
    }

    /// Delivers only an applicable source or manifest event.
    pub fn dispatch(&self, path: impl AsRef<Path>, on_change: impl FnOnce(PreviewChange)) -> bool {
        let Some(event) = self.route(path) else {
            return false;
        };
        on_change(event);
        true
    }

    #[cfg(feature = "preview-watch")]
    pub(crate) fn is_source_directory(&self, path: &Path) -> bool {
        self.source_dirs.iter().any(|dir| path.starts_with(dir))
            && !self.excluded.iter().any(|dir| path.starts_with(dir))
    }

    #[cfg(feature = "preview-watch")]
    pub(crate) fn watch_plan(&self) -> io::Result<std::collections::BTreeMap<PathBuf, bool>> {
        use std::{collections::BTreeMap, fs};

        fn register_parent(plan: &mut BTreeMap<PathBuf, bool>, path: &Path) {
            if let Some(parent) = path
                .ancestors()
                .skip(1)
                .find(|candidate| candidate.is_dir())
            {
                plan.entry(parent.to_owned()).or_insert(false);
            }
        }

        let mut plan = BTreeMap::new();
        for file in self.manifests.iter().chain(&self.sources) {
            if !self
                .excluded
                .iter()
                .any(|excluded| file.starts_with(excluded))
            {
                // Watch the containing directory to survive atomic replacement.
                register_parent(&mut plan, file);
            }
        }
        for root in &self.source_dirs {
            let mut directories = vec![root.clone()];
            while let Some(path) = directories.pop() {
                if self
                    .excluded
                    .iter()
                    .any(|excluded| path.starts_with(excluded))
                {
                    continue;
                }
                match fs::symlink_metadata(&path) {
                    Ok(metadata) if !metadata.is_dir() => continue,
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        register_parent(&mut plan, &path);
                        continue;
                    }
                    Err(error) => return Err(error),
                }
                if self
                    .excluded
                    .iter()
                    .any(|excluded| excluded.starts_with(&path))
                {
                    // A recursive watch cannot exclude any descendants.
                    // Only branch where necessary to avoid generated trees.
                    plan.entry(path.clone()).or_insert(false);
                    for entry in fs::read_dir(path)? {
                        let entry = entry?;
                        if entry.file_type()?.is_dir() {
                            directories.push(entry.path());
                        }
                    }
                } else {
                    plan.insert(path, true);
                }
            }
        }
        let recursive: Vec<_> = plan
            .iter()
            .filter_map(|(path, recursive)| recursive.then_some(path.clone()))
            .collect();
        plan.retain(|path, _| {
            !recursive
                .iter()
                .any(|parent| parent != path && path.starts_with(parent))
        });
        Ok(plan)
    }

    fn absolute(base: &Path, raw: &Path) -> io::Result<PathBuf> {
        let requested = base.join(raw);
        let mut normalized = PathBuf::new();
        for component in requested.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if normalized.file_name().is_none() || !normalized.pop() {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "preview path escapes its filesystem root",
                        ));
                    }
                }
                component => normalized.push(component.as_os_str()),
            }
        }
        if normalized.is_absolute() {
            Ok(normalized)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "preview path is not absolute",
            ))
        }
    }
}
