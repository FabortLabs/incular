//! Deterministic Rust source generation from portable preview descriptions.

use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
};

/// A library namespace reported by preview analysis.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PreviewNamespace {
    /// One of the scaffold-owned helper modules.
    Helper(PreviewHelper),
    /// A public path through a dependency (for example "my_crate::previews").
    Package(String),
    /// A standard SDK path rooted at std, core, or alloc.
    Sdk(String),
    /// A discovered module that the independent scaffold cannot import.
    LocalFile(PathBuf),
}

/// Scaffold-owned import namespaces.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PreviewHelper {
    /// Preview descriptor helpers.
    WidgetPreview,
    /// Shared scaffold utilities.
    Utils,
}

impl PreviewNamespace {
    fn import_path(&self) -> Result<Option<&str>, PreviewCodegenError> {
        match self {
            Self::LocalFile(_) => Ok(None),
            Self::Helper(PreviewHelper::WidgetPreview) => Ok(Some("crate::widget_preview")),
            Self::Helper(PreviewHelper::Utils) => Ok(Some("crate::utils")),
            Self::Package(path) if valid_path(path) => Ok(Some(path)),
            Self::Sdk(path)
                if valid_path(path)
                    && path
                        .split("::")
                        .next()
                        .is_some_and(|root| matches!(root, "std" | "core" | "alloc")) =>
            {
                Ok(Some(path))
            }
            Self::Package(path) | Self::Sdk(path) => {
                Err(PreviewCodegenError::InvalidPath(path.clone()))
            }
        }
    }
}

/// Invalid namespace, symbol, or prior compiler allocation.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PreviewCodegenError {
    /// Seeding cannot replace a live allocation table.
    #[error("preview import prefixes have already been allocated")]
    AlreadyAllocated,
    /// An import cannot be written as a supported Rust path.
    #[error("invalid preview import path {0:?}")]
    InvalidPath(String),
    /// A preview factory was not a supported Rust identifier.
    #[error("invalid Rust preview symbol {0:?}")]
    InvalidSymbol(String),
    /// Seed aliases must be unique and greater than zero.
    #[error("preview import prefix {0} is not unique and positive")]
    InvalidPrefix(u32),
    /// All representable numeric aliases have been reserved.
    #[error("preview import prefixes exhausted")]
    PrefixExhausted,
    /// The preview build lives outside an importable library.
    #[error("the scaffold cannot import the preview source {0:?}")]
    Unimportable(PreviewNamespace),
}

/// One retained use declaration in the generated source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewImport {
    /// Origin of the imported Rust path.
    pub namespace: PreviewNamespace,
    /// Numeric suffix of its generated Rust alias.
    pub prefix: u32,
}

/// Allocates stable Rust aliases across a regeneration. Filtered imports
/// reserve their old prefix numbers without emitting use declarations.
pub struct PreviewImportAllocator {
    imports: Vec<PreviewImport>,
    by_namespace: HashMap<PreviewNamespace, u32>,
    next: u64,
}

impl Default for PreviewImportAllocator {
    fn default() -> Self {
        Self {
            imports: Vec::new(),
            by_namespace: HashMap::new(),
            next: 1,
        }
    }
}

impl PreviewImportAllocator {
    /// Restores prior import allocations transactionally.
    pub fn seed(
        &mut self,
        imports: impl IntoIterator<Item = PreviewImport>,
    ) -> Result<(), PreviewCodegenError> {
        if self.next != 1 {
            return Err(PreviewCodegenError::AlreadyAllocated);
        }
        let mut seeded = Self::default();
        let mut aliases = HashSet::new();
        for import in imports {
            let prefix = import.prefix;
            if prefix == 0 || !aliases.insert(prefix) {
                return Err(PreviewCodegenError::InvalidPrefix(prefix));
            }
            seeded.next = seeded.next.max(u64::from(prefix) + 1);
            if import.namespace.import_path()?.is_some() {
                seeded.by_namespace.insert(import.namespace.clone(), prefix);
                seeded.imports.push(import);
            }
        }
        *self = seeded;
        Ok(())
    }

    /// The importable subset, in seed or first use order.
    #[must_use]
    pub fn imports(&self) -> &[PreviewImport] {
        &self.imports
    }

    /// Resolves or allocates a Rust alias for a symbol. No namespace
    /// means a local name and leaves the aliases untouched.
    pub fn qualify(
        &mut self,
        namespace: Option<&PreviewNamespace>,
        symbol: &str,
    ) -> Result<String, PreviewCodegenError> {
        if !valid_identifier(symbol) {
            return Err(PreviewCodegenError::InvalidSymbol(symbol.to_owned()));
        }
        let Some(namespace) = namespace else {
            return Ok(symbol.to_owned());
        };
        if namespace.import_path()?.is_none() {
            return Err(PreviewCodegenError::Unimportable(namespace.clone()));
        }
        let prefix = if let Some(&existing) = self.by_namespace.get(namespace) {
            existing
        } else {
            let prefix =
                u32::try_from(self.next).map_err(|_| PreviewCodegenError::PrefixExhausted)?;
            self.next += 1;
            self.by_namespace.insert(namespace.clone(), prefix);
            self.imports.push(PreviewImport {
                namespace: namespace.clone(),
                prefix,
            });
            prefix
        };
        Ok(format!("_i{prefix}::{symbol}"))
    }

    fn write_imports(&self, source: &mut String) -> Result<(), PreviewCodegenError> {
        for import in self.imports() {
            let path = import.namespace.import_path()?.expect("retained import");
            let absolute = if path.starts_with("crate::") {
                ""
            } else {
                "::"
            };
            let _ = writeln!(source, "use {absolute}{path} as _i{};", import.prefix);
        }
        Ok(())
    }
}

/// Preview definition supplied by Rust build/IDE analysis. A discoverable
/// root-level file can exist without an importable Cargo package.
#[derive(Clone, Debug)]
pub struct DiscoveredPreview {
    /// Cargo package owning the preview; absent for root source files.
    pub package: Option<String>,
    /// Library exposing the factory.
    pub library: PreviewNamespace,
    /// Name of the exported, zero-argument widget factory.
    pub function: String,
    /// Source URI to display or open in connected tooling.
    pub source_uri: String,
    /// Analyzer-supplied source line.
    pub line: u32,
    /// Analyzer-supplied source column.
    pub column: u32,
    /// Override for the display name.
    pub name: Option<String>,
}

/// Generates a registry for native facade widgets, excluding previews that
/// have no package. This creates source only; it does not run generated code
/// or launch the preview host.
pub fn generate_preview_registry(
    imports: impl IntoIterator<Item = PreviewImport>,
    previews: &[DiscoveredPreview],
) -> Result<String, PreviewCodegenError> {
    let mut allocator = PreviewImportAllocator::default();
    allocator.seed(imports)?;
    let mut rows = String::new();
    for preview in previews {
        let Some(package) = &preview.package else {
            continue;
        };
        let factory = allocator.qualify(Some(&preview.library), &preview.function)?;
        let name = preview
            .name
            .as_ref()
            .map_or_else(|| "None".into(), |value| format!("Some({value:?})"));
        let _ = writeln!(
            rows,
            "    GeneratedPreview {{ package: {package:?}, source_uri: {:?}, line: {}, column: {}, name: {name}, build: {factory} }},",
            preview.source_uri, preview.line, preview.column,
        );
    }

    let mut source = String::from(
        "// Generated by Incular's widget preview tooling.\n#[allow(unused_imports)]\nmod preview_imports {\n",
    );
    // Wrap the imports and row table in one module so unused seeded aliases
    // are allowed, without changing lint behavior for the host.
    allocator.write_imports(&mut source)?;
    source.push_str(concat!(
        "\npub struct GeneratedPreview {\n",
        "    pub package: &'static str,\n",
        "    pub source_uri: &'static str,\n",
        "    pub line: u32,\n",
        "    pub column: u32,\n",
        "    pub name: Option<&'static str>,\n",
        "    pub build: fn() -> ::incular::widgets::Widget,\n",
        "}\n\n",
        "pub const PREVIEWS: &[GeneratedPreview] = &[\n",
    ));
    source.push_str(&rows);
    source.push_str("];\n}\npub use preview_imports::{GeneratedPreview, PREVIEWS};\n");
    Ok(source)
}

fn valid_path(value: &str) -> bool {
    value.split("::").all(valid_identifier)
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.bytes();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
        && !matches!(
            value,
            "_" | "Self"
                | "abstract"
                | "as"
                | "async"
                | "await"
                | "become"
                | "box"
                | "break"
                | "const"
                | "continue"
                | "crate"
                | "do"
                | "dyn"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "final"
                | "fn"
                | "for"
                | "gen"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "macro"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "override"
                | "priv"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "try"
                | "type"
                | "typeof"
                | "union"
                | "unsafe"
                | "unsized"
                | "use"
                | "virtual"
                | "where"
                | "while"
                | "yield"
        )
}
