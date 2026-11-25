//! Flutter 3.50 widget preview scaffold regeneration at a native host boundary.
use std::{fs, path::PathBuf};

use incular_devtools::preview_manifest::{
    PreviewManifest, PreviewManifestError, PreviewRegeneration, PreviewScaffoldInvalidation,
};
use serde_json::Value;

const SDK: &str = "2.1.0-dev.8.0.flutter-4312ae32";
const ROOT: &str = "[package]\nname = 'preview-root'\nversion = '0.1.0'\n";

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let scaffold = temp.path().join("scaffold");
    let input = temp.path().join("project").join("Cargo.toml");
    fs::create_dir_all(input.parent().unwrap()).unwrap();
    (temp, scaffold, input)
}

#[test]
fn generates_a_manifest_with_format_sdk_and_project_identity() {
    let (_dir, scaffold, path) = fixture();
    let manifest = PreviewManifest::new(scaffold.clone(), SDK, path.clone());
    assert!(!scaffold.exists());
    // Flutter generates a scaffold even for an empty test root.
    manifest.generate().unwrap();
    let data: Value = serde_json::from_slice(&fs::read(manifest.path()).unwrap()).unwrap();
    assert_eq!(data["version"], PreviewManifest::VERSION);
    assert_eq!(data["sdk-version"], SDK);
    let hashes = data["project-manifest-hashes"].as_object().unwrap();
    assert_eq!(hashes.len(), 1);
    assert!(hashes[path.to_str().unwrap()].as_str().is_some());
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);
}

#[test]
fn missing_scaffold_and_sdk_change_require_a_scaffold_rebuild() {
    let (_dir, scaffold, path) = fixture();
    let manifest = PreviewManifest::new(scaffold.clone(), SDK, path.clone());
    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::Scaffold(PreviewScaffoldInvalidation::MissingScaffold),
    );
    manifest.generate().unwrap();
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);

    let modified_sdk = format!("{SDK}foo");
    let modified = PreviewManifest::new(scaffold, &modified_sdk, path);
    let PreviewRegeneration::Scaffold(reason) = modified.regeneration().unwrap() else {
        panic!("expected SDK mismatch to require a new scaffold");
    };
    assert_eq!(
        reason,
        PreviewScaffoldInvalidation::SdkVersionChanged {
            generated: SDK.into(),
            current: modified_sdk.clone(),
        },
    );
    let detail = reason.to_string();
    assert!(detail.contains(SDK) && detail.contains(&modified_sdk));
    assert!(!modified.update_project_hashes().unwrap());
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);
}

#[test]
fn editing_current_project_refreshes_only_dependency_metadata() {
    let (_dir, scaffold, path) = fixture();
    let manifest = PreviewManifest::new(scaffold, SDK, path.clone());
    manifest.generate().unwrap();
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);
    fs::write(&path, ROOT).unwrap();
    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::ProjectMetadata
    );
    assert!(manifest.update_project_hashes().unwrap());
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);
    fs::write(&path, format!("# comment only\n{ROOT}\n")).unwrap();
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);
}

#[test]
fn deleted_workspace_member_and_corruption_are_distinct() {
    let (dir, scaffold, root) = fixture();
    fs::write(&root, ROOT).unwrap();
    let member = dir.path().join("member.toml");
    fs::write(&member, "[package]\nname = 'member'\n").unwrap();
    let manifest =
        PreviewManifest::new(scaffold, SDK, root.clone()).workspace_members([member.clone()]);
    manifest.generate().unwrap();
    fs::remove_file(&member).unwrap();
    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::ProjectMetadata
    );
    assert!(manifest.update_project_hashes().unwrap());
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);

    fs::write(manifest.path(), b"corrupt").unwrap();
    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::Scaffold(PreviewScaffoldInvalidation::InvalidManifest),
    );
    manifest.generate().unwrap();
    fs::write(&root, "[package\n").unwrap();
    assert!(
        matches!(manifest.regeneration(), Err(PreviewManifestError::Syntax { path, .. }) if path == root)
    );
}

#[test]
fn metadata_refresh_preserves_other_fields_and_does_not_mask_format_mismatch() {
    let (_dir, scaffold, root) = fixture();
    fs::write(&root, ROOT).unwrap();
    let manifest = PreviewManifest::new(scaffold, SDK, root);
    manifest.generate().unwrap();
    let path = manifest.path();
    let mut stored: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let fields = stored.as_object_mut().unwrap();
    fields.remove("project-manifest-hashes");
    fields.insert("host-extension".into(), Value::Bool(true));
    fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();

    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::ProjectMetadata
    );
    assert!(manifest.update_project_hashes().unwrap());
    let mut stored: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(stored["host-extension"], true);
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);

    stored["version"] = "0".into();
    fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::Scaffold(PreviewScaffoldInvalidation::ManifestVersionChanged),
    );
    assert!(!manifest.update_project_hashes().unwrap());
}
