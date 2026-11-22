//! Running external tools with readable failures.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Runs `command`, streaming its output, and fails with its exit status.
pub(crate) fn run(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("cannot start {}: {error}", describe(command)))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with {status}", describe(command)))
    }
}

/// Runs `command` and returns its captured output, failing on a bad status.
pub(crate) fn output(command: &mut Command) -> Result<Output, String> {
    let output = command
        .output()
        .map_err(|error| format!("cannot start {}: {error}", describe(command)))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "{} failed with {}:\n{}",
            describe(command),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn describe(command: &Command) -> String {
    let mut text = command.get_program().to_string_lossy().into_owned();
    for argument in command.get_args() {
        text.push(' ');
        text.push_str(&argument.to_string_lossy());
    }
    text
}

/// The Cargo workspace or package metadata, including dependencies.
///
/// Resolves with every feature enabled: apps usually make the mobile shell
/// a non-default feature (`mobile = ["incular/mobile"]` beside a default
/// `desktop`), and the shell's package must still be found.
pub(crate) fn cargo_metadata() -> Result<serde_json::Value, String> {
    let output = output(Command::new("cargo").args([
        "metadata",
        "--format-version",
        "1",
        "--all-features",
    ]))?;
    serde_json::from_slice(&output.stdout).map_err(|error| format!("bad cargo metadata: {error}"))
}

/// The directory holding `package`'s `Cargo.toml`, found through Cargo so
/// registry and path dependencies both work.
pub(crate) fn package_dir(metadata: &serde_json::Value, package: &str) -> Result<PathBuf, String> {
    metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|candidate| candidate["name"] == package)
        .and_then(|candidate| candidate["manifest_path"].as_str())
        .and_then(|manifest| Path::new(manifest).parent().map(Path::to_path_buf))
        .ok_or_else(|| {
            format!("{package} is not in the dependency graph; enable incular's `mobile` feature")
        })
}

/// The resolved version of `package`, when it is in the dependency graph.
pub(crate) fn package_version(metadata: &serde_json::Value, package: &str) -> Option<String> {
    metadata["packages"]
        .as_array()?
        .iter()
        .find(|candidate| candidate["name"] == package)?["version"]
        .as_str()
        .map(str::to_owned)
}

/// The Cargo target directory.
pub(crate) fn target_dir(metadata: &serde_json::Value) -> PathBuf {
    metadata["target_directory"]
        .as_str()
        .map_or_else(|| PathBuf::from("target"), PathBuf::from)
}

/// Writes `contents` to `path`, creating parent directories.
pub(crate) fn write(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, contents)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Forward slashes, which Gradle scripts and plists accept on every host.
pub(crate) fn portable(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// A reverse-DNS-safe identifier segment.
pub(crate) fn identifier(name: &str) -> String {
    let mut segment: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if segment.starts_with(|character: char| character.is_ascii_digit()) {
        segment.insert(0, 'x');
    }
    segment
}
