use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use incular_devtools::preview_manifest::PreviewManifest;

const TEMPLATE: &str = r#"
[package]
name = "preview-host"
version = "0.1.0"
edition = "2024"

[package.metadata.incular-preview]
host = "retained"

[workspace]
"#;

fn package(directory: &Path, name: &str, dependencies: &str) -> PathBuf {
    fs::create_dir_all(directory.join("src")).unwrap();
    fs::write(directory.join("src/lib.rs"), "").unwrap();
    let manifest = directory.join("Cargo.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{dependencies}"
        ),
    )
    .unwrap();
    manifest
}

#[test]
fn preview_host_links_workspace_packages_and_resolves_version_based_peer_references() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Cargo.toml");
    fs::write(
        &root,
        "[workspace]\nmembers = [\"abcd\", \"example\"]\nresolver = \"3\"\n",
    )
    .unwrap();
    let library = package(&temp.path().join("abcd"), "abcd", "");
    let example = package(
        &temp.path().join("example"),
        "example",
        "[dependencies]\nabcd = \"0.1\"\n",
    );
    let scaffold = temp.path().join("scaffold");
    let manifest = PreviewManifest::new(&scaffold, "sdk", &root)
        .workspace_members([library.clone(), example.clone()]);
    manifest.write_scaffold_dependencies(TEMPLATE).unwrap();
    let generated = scaffold.join("Cargo.toml");
    let contents: toml::Table = fs::read_to_string(&generated).unwrap().parse().unwrap();
    for (name, member) in [("abcd", &library), ("example", &example)] {
        let path = member.parent().unwrap();
        let as_text = path.to_str().unwrap();
        let expected = if cfg!(windows) {
            as_text.replace('\\', "/")
        } else {
            as_text.to_owned()
        };
        assert_eq!(
            contents["dependencies"][name]["path"].as_str(),
            Some(expected.as_str())
        );
        assert_eq!(
            contents["patch"]["crates-io"][name]["path"].as_str(),
            Some(expected.as_str())
        );
    }

    fs::create_dir_all(scaffold.join("src")).unwrap();
    fs::write(scaffold.join("src/lib.rs"), "").unwrap();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let result = Command::new(cargo)
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&generated)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let packages = metadata["packages"].as_array().unwrap();
    let abcd = packages
        .iter()
        .find(|package| package["name"] == "abcd")
        .unwrap();
    let example = packages
        .iter()
        .find(|package| package["name"] == "example")
        .unwrap();
    assert!(abcd["source"].is_null());
    let peer = metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == example["id"])
        .unwrap();
    assert!(
        peer["deps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|dep| dep["pkg"] == abcd["id"])
    );

    PreviewManifest::new(&scaffold, "sdk", root)
        .workspace_members([library])
        .write_scaffold_dependencies(TEMPLATE)
        .unwrap();
    let refreshed: toml::Table = fs::read_to_string(generated).unwrap().parse().unwrap();
    assert!(refreshed["dependencies"].get("example").is_none());
    assert!(refreshed["patch"]["crates-io"].get("example").is_none());
    assert_eq!(
        refreshed["package"]["metadata"]["incular-preview"]["host"].as_str(),
        Some("retained")
    );
}

#[test]
fn duplicate_local_names_do_not_overwrite_an_existing_scaffold() {
    let temp = tempfile::tempdir().unwrap();
    let root = package(&temp.path().join("root"), "same", "");
    let member = package(&temp.path().join("member"), "same", "");
    let scaffold = temp.path().join("scaffold");
    fs::create_dir_all(&scaffold).unwrap();
    let generated = scaffold.join("Cargo.toml");
    fs::write(&generated, TEMPLATE).unwrap();
    let result = PreviewManifest::new(scaffold, "sdk", root)
        .workspace_members([member])
        .write_scaffold_dependencies(TEMPLATE);
    assert!(matches!(
        result,
        Err(incular_devtools::preview_dependencies::PreviewDependenciesError::DuplicatePackage(name))
            if name == "same"
    ));
    assert_eq!(fs::read_to_string(generated).unwrap(), TEMPLATE);
}
