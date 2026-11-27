//! Idempotent migration for the generated preview scaffold's ignore rule.

use std::{
    fs,
    io::{self, Write},
    path::{Component, Path},
};

/// Outcome of checking a project's existing `.gitignore`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewIgnoreMigration {
    /// The project has no ignore file; do not create it.
    Missing,
    /// A matching ignore rule is already present; do not rewrite the file.
    Unchanged,
    /// Appended the scaffold rule to the project's ignore file.
    Updated,
}

/// Appends an ignore rule for a generated scaffold inside the project.
///
/// `scaffold` is relative to `project`, e.g. `.widget_preview`. The
/// project must already have a regular `.gitignore`. Existing bytes and
/// permissions are preserved, and updating is atomic.
pub fn migrate_preview_gitignore(
    project: &Path,
    scaffold: &Path,
) -> io::Result<PreviewIgnoreMigration> {
    use PreviewIgnoreMigration::{Missing, Unchanged, Updated};

    let path = project.join(".gitignore");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => return Err(invalid("the project's .gitignore is not a regular file")),
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(Missing),
        Err(source) => return Err(source),
    };
    let pattern = ignore_directory_pattern(scaffold)?;
    let mut output = fs::read(&path)?;
    let rule = pattern.as_bytes();

    // Ignore commented occurrences and account for later negation of the
    // literal rule. Do not normalize arbitrary user-supplied glob rules.
    let mut ignored = false;
    for line in output.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let negative = line.strip_prefix(b"!");
        let candidate = negative.unwrap_or(line);
        if candidate.strip_prefix(b"/").unwrap_or(candidate) == rule {
            ignored = negative.is_none();
        }
    }
    if ignored {
        return Ok(Unchanged);
    }

    let newline: &[u8] = if output.windows(2).any(|bytes| bytes == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    if !output.ends_with(b"\n") {
        output.extend_from_slice(newline);
    }
    output.extend_from_slice(b"# Widget Preview related");
    output.extend_from_slice(newline);
    output.extend_from_slice(rule);
    output.extend_from_slice(newline);
    let mut staged = tempfile::NamedTempFile::new_in(project)?;
    staged.write_all(&output)?;
    staged.as_file().set_permissions(metadata.permissions())?;
    staged.persist(&path).map_err(|source| source.error)?;
    Ok(Updated)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn ignore_directory_pattern(scaffold: &Path) -> io::Result<String> {
    let mut pattern = String::new();
    for component in scaffold.components() {
        let Component::Normal(segment) = component else {
            return Err(invalid(
                "the scaffold needs a relative path inside the project",
            ));
        };
        let segment = segment
            .to_str()
            .filter(|segment| !segment.contains(['\n', '\r']))
            .ok_or_else(|| invalid("the scaffold path cannot be represented as an ignore rule"))?;
        for ch in segment.chars() {
            if matches!(ch, '\\' | '*' | '?' | '[' | ']' | ' ' | '#' | '!') {
                pattern.push('\\');
            }
            pattern.push(ch);
        }
        pattern.push('/');
    }
    if pattern.is_empty() {
        return Err(invalid("the scaffold path cannot be empty"));
    }
    Ok(pattern)
}
