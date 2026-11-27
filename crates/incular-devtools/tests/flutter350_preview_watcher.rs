//! Exercises the native backend against on-disk preview files.
#![cfg(feature = "preview-watch")]

use std::{fs, path::Path, time::Duration};

use incular_devtools::{
    preview_changes::PreviewChange,
    preview_manifest::{PreviewManifest, PreviewRegeneration},
    preview_watcher::{PreviewWatchUpdate, PreviewWatcher},
};

fn manifest(root: &Path) -> PreviewManifest {
    let cargo = root.join("Cargo.toml");
    fs::write(&cargo, "[package]\nname='preview'\nversion='0.1.0'\n").unwrap();
    PreviewManifest::new(root.join("src/generated"), "pinned", cargo)
}

async fn next(watcher: &mut PreviewWatcher) -> PreviewWatchUpdate {
    tokio::time::timeout(Duration::from_secs(5), watcher.next_update())
        .await
        .expect("no native notification")
        .unwrap()
        .expect("watcher closed")
}

#[tokio::test]
async fn native_watch_excludes_generated_and_unknown_sources() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let manifest = manifest(root);
    for folder in ["src/generated/src", "src/nested", "target", "unknown"] {
        fs::create_dir_all(root.join(folder)).unwrap();
    }
    let mut watcher = PreviewWatcher::start(manifest.change_router().unwrap()).unwrap();
    for path in [
        "src/generated/src/lib.rs",
        "src/generated/Cargo.toml",
        "target/a.rs",
        "unknown/a.rs",
        "src/notes.txt",
    ] {
        fs::write(root.join(path), "not a preview source").unwrap();
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(150), watcher.next_update())
            .await
            .is_err()
    );
    let source = root.join("src/nested/lib.rs");
    fs::write(&source, "fn example() {}").unwrap();
    assert_eq!(
        next(&mut watcher).await,
        PreviewWatchUpdate::Change(PreviewChange::Source(source))
    );
}

#[tokio::test]
async fn native_watch_detects_removed_sources() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let manifest = manifest(root);
    fs::create_dir(root.join("src")).unwrap();
    let source = root.join("src/lib.rs");
    fs::write(&source, "fn example() {}").unwrap();
    let mut watcher = PreviewWatcher::start(manifest.change_router().unwrap()).unwrap();
    fs::remove_file(&source).unwrap();
    assert_eq!(
        next(&mut watcher).await,
        PreviewWatchUpdate::Change(PreviewChange::Source(source))
    );
}

#[tokio::test]
async fn native_watch_adds_a_new_source_directory_and_requests_rescan() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let manifest = manifest(root);
    let mut watcher = PreviewWatcher::start(manifest.change_router().unwrap()).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    assert_eq!(next(&mut watcher).await, PreviewWatchUpdate::RescanRequired);
    let source = root.join("src/lib.rs");
    fs::write(&source, "fn example() {}").unwrap();
    assert_eq!(
        next(&mut watcher).await,
        PreviewWatchUpdate::Change(PreviewChange::Source(source))
    );
}

#[tokio::test]
async fn native_watch_routes_atomic_manifest_replacement_to_dependency_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let manifest = manifest(root);
    manifest.generate().unwrap();
    let mut watcher = PreviewWatcher::start(manifest.change_router().unwrap()).unwrap();
    let staged = root.join("manifest.new");
    let cargo = root.join("Cargo.toml");
    fs::write(&staged, "[package]\nname='preview'\nversion='0.2.0'\n").unwrap();
    fs::rename(&staged, &cargo).unwrap();
    assert_eq!(
        next(&mut watcher).await,
        PreviewWatchUpdate::Change(PreviewChange::Manifest(cargo))
    );
    assert_eq!(
        manifest.regeneration().unwrap(),
        PreviewRegeneration::ProjectMetadata
    );
}

#[tokio::test]
async fn populated_subtree_moved_into_or_out_of_a_recursive_watch_requests_rescan() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let manifest = manifest(root);
    fs::create_dir_all(root.join("src/regular")).unwrap();
    fs::create_dir_all(root.join("staged/nested")).unwrap();
    fs::write(root.join("staged/nested/lib.rs"), "fn preview() {}").unwrap();
    let mut watcher = PreviewWatcher::start(manifest.change_router().unwrap()).unwrap();

    let source = root.join("src/regular/preview");
    fs::rename(root.join("staged"), &source).unwrap();
    assert_eq!(next(&mut watcher).await, PreviewWatchUpdate::RescanRequired);
    assert_eq!(
        fs::read_to_string(source.join("nested/lib.rs")).unwrap(),
        "fn preview() {}"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(150), watcher.next_update())
            .await
            .is_err(),
        "compound rename notifications must not request duplicate rescans"
    );

    fs::rename(&source, root.join("removed")).unwrap();
    assert_eq!(next(&mut watcher).await, PreviewWatchUpdate::RescanRequired);
    assert!(!source.exists());
    assert!(
        tokio::time::timeout(Duration::from_millis(150), watcher.next_update())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn rename_of_non_source_file_does_not_request_a_rescan() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let manifest = manifest(root);
    fs::create_dir_all(root.join("src/regular")).unwrap();
    let before = root.join("src/regular/README.txt");
    let after = root.join("src/regular/README.md");
    fs::write(&before, "not a preview source").unwrap();
    let mut watcher = PreviewWatcher::start(manifest.change_router().unwrap()).unwrap();
    fs::rename(&before, &after).unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(150), watcher.next_update())
            .await
            .is_err()
    );
}
