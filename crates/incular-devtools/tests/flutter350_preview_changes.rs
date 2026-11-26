//! Preview invalidation at the host watcher and manifest boundary.
use std::{fs, path::Path};

use incular_devtools::{
    preview_changes::PreviewChange,
    preview_manifest::{PreviewManifest, PreviewRegeneration},
};

#[test]
fn generated_or_unknown_context_events_cannot_invalidate_previews() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("workspace");
    let scaffold = root.join("src/generated-scaffold");
    let manifest = PreviewManifest::new(&scaffold, "pinned", root.join("Cargo.toml"))
        .workspace_members([root.join("packages/member/Cargo.toml")]);
    let mut changed = manifest.change_router().unwrap();
    changed.exclude_directory("src/generated-target").unwrap();
    let mut seen = Vec::new();
    for path in [
        root.join("target/debug/build/example/out/generated.rs"),
        root.join("packages/member/target/debug/generated.rs"),
        scaffold.join("src/main.rs"),
        scaffold.join("Cargo.toml"),
        root.join("src/generated-target/deep/lib.rs"),
        root.join(".git/internal.rs"),
        root.join("unknown/no_analysis_context.rs"),
        dir.path().join("foreign/src/lib.rs"),
        root.join("Cargo.toml.bak"),
        root.join("src/notes.txt"),
        root.join("src/temp/../../unknown/file.rs"),
    ] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"fn preview_only() {}").unwrap();
        assert!(
            !changed.dispatch(&path, |event| seen.push(event)),
            "{path:?}"
        );
    }
    assert!(seen.is_empty());
}

#[test]
fn registered_source_updates_and_deletions_both_reach_the_preview_host() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    let member = project.join("crates/member");
    let scaffold = project.join(".incular/scaffold");
    let manifest = PreviewManifest::new(scaffold, "pinned", project.join("Cargo.toml"))
        .workspace_members([member.join("Cargo.toml")]);
    let mut changes = manifest.change_router().unwrap();
    changes.include_source_directory("components/ui").unwrap();
    changes.include_source_file("scripts/previews.rs").unwrap();
    let mut seen = Vec::new();
    for relative in [
        "src/lib.rs",
        "crates/member/src/button.rs",
        "crates/member/examples/gallery.rs",
        "components/ui/shell.rs",
        "scripts/previews.rs",
        "build.rs",
        "src/target/theme.rs",
    ] {
        let path = project.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"fn preview_only() {}").unwrap();
        assert!(changes.dispatch(relative, |event| seen.push(event)));
        fs::remove_file(&path).unwrap();
        assert!(changes.dispatch(relative, |event| seen.push(event)));
        let expected = PreviewChange::Source(path);
        assert_eq!(&seen[seen.len() - 2..], &[expected.clone(), expected]);
    }
    assert_eq!(seen.len(), 14);
    assert_eq!(
        changes.route(Path::new("components/ui/../ui/./shell.rs")),
        changes.route("components/ui/shell.rs"),
    );
}

#[test]
fn manifest_events_refresh_dependency_metadata_not_the_scaffold() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let root_manifest = root.join("Cargo.toml");
    let member = root.join("member/Cargo.toml");
    fs::create_dir_all(member.parent().unwrap()).unwrap();
    fs::write(&root_manifest, "[workspace]\nmembers = ['member']").unwrap();
    fs::write(&member, "[package]\nname = 'member'\nversion = '0.1.0'").unwrap();
    let manifest = PreviewManifest::new(root.join("scaffold"), "pinned", &root_manifest)
        .workspace_members([member.clone()]);
    manifest.generate().unwrap();
    let changes = manifest.change_router().unwrap();
    assert_eq!(
        changes.route("Cargo.toml"),
        Some(PreviewChange::Manifest(root_manifest))
    );

    fs::write(&member, "[package]\nname = 'member'\nversion = '0.2.0'").unwrap();
    assert!(changes.dispatch("member/Cargo.toml", |event| {
        assert_eq!(event, PreviewChange::Manifest(member.clone()));
        assert_eq!(
            manifest.regeneration().unwrap(),
            PreviewRegeneration::ProjectMetadata
        );
        assert!(manifest.update_project_hashes().unwrap());
    }));
    assert_eq!(manifest.regeneration().unwrap(), PreviewRegeneration::None);

    fs::remove_file(&member).unwrap();
    assert_eq!(
        changes.route("member/Cargo.toml"),
        Some(PreviewChange::Manifest(member))
    );
    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::ProjectMetadata
    );
}
