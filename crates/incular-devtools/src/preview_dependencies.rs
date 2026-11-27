//! Local dependencies of a generated widget preview Cargo scaffold.

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use toml::{Table, Value};

/// A preview scaffold dependency manifest could not be generated.
#[derive(Debug, thiserror::Error)]
pub enum PreviewDependenciesError {
    /// A project could not be read, or the scaffold could not be written.
    #[error("preview dependency manifest I/O: {0}")]
    Io(#[from] io::Error),
    /// A TOML document could not be parsed.
    #[error("invalid preview Cargo manifest {path}: {source}")]
    Syntax {
        /// Path of the affected input, or output path for the template.
        path: PathBuf,
        /// The underlying TOML error.
        #[source]
        source: toml::de::Error,
    },
    /// A scaffold section was not a table.
    #[error("preview Cargo section {0} must be a table")]
    InvalidSection(&'static str),
    /// Two different local packages claimed the same Cargo name.
    #[error("multiple preview packages are named {0}")]
    DuplicatePackage(String),
    /// A package directory could not be encoded in a Cargo manifest.
    #[error("preview package path is not UTF-8: {0}")]
    NonUtf8Path(PathBuf),
    /// The generated table could not be serialized.
    #[error("could not write the preview Cargo manifest: {0}")]
    Serialize(#[from] toml::ser::Error),
}

fn parse(text: &str, path: &Path) -> Result<Table, PreviewDependenciesError> {
    text.parse::<Table>()
        .map_err(|source| PreviewDependenciesError::Syntax {
            path: path.to_owned(),
            source,
        })
}

fn section<'a>(
    parent: &'a mut Table,
    name: &'static str,
) -> Result<&'a mut Table, PreviewDependenciesError> {
    parent
        .entry(name)
        .or_insert_with(|| Value::Table(Table::new()))
        .as_table_mut()
        .ok_or(PreviewDependenciesError::InvalidSection(name))
}

/// Builds a scaffold from its unmodified host template. Each named local
/// package appears as a direct path dependency and as a crates.io patch so
/// version-based transitive references to workspace peers resolve locally.
///
/// Every call starts from the supplied template, so members removed from the
/// workspace cannot linger as dependencies. Unrelated template entries survive.
pub(crate) fn write_from_template<'a>(
    output: &Path,
    template: &str,
    projects: impl IntoIterator<Item = &'a Path>,
) -> Result<(), PreviewDependenciesError> {
    let mut packages = BTreeMap::new();
    for project in projects {
        let path = std::path::absolute(project)?;
        let root = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "expected a project Cargo.toml")
        })?;
        let manifest = parse(&fs::read_to_string(&path)?, &path)?;
        let Some(name) = manifest
            .get("package")
            .and_then(Value::as_table)
            .and_then(|package| package.get("name"))
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
        else {
            continue; // A virtual workspace has no directly importable package.
        };
        let raw_path = root
            .to_str()
            .ok_or_else(|| PreviewDependenciesError::NonUtf8Path(root.to_owned()))?;
        let directory = if cfg!(windows) {
            raw_path.replace('\\', "/")
        } else {
            raw_path.to_owned()
        };
        if packages
            .insert(name.to_owned(), directory.clone())
            .is_some_and(|old| old != directory)
        {
            return Err(PreviewDependenciesError::DuplicatePackage(name.to_owned()));
        }
    }

    let mut scaffold = parse(template, output)?;
    let declarations = packages
        .into_iter()
        .map(|(name, path)| {
            let dependency =
                Value::Table(Table::from_iter([("path".to_owned(), Value::String(path))]));
            (name, dependency)
        })
        .collect::<Table>();
    section(&mut scaffold, "dependencies")?.extend(declarations.clone());
    section(section(&mut scaffold, "patch")?, "crates-io")?.extend(declarations);
    let body = toml::to_string(&scaffold)?;

    let directory = output.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected a scaffold Cargo.toml",
        )
    })?;
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(body.as_bytes())?;
    file.persist(output).map_err(|error| error.error)?;
    Ok(())
}
