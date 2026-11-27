use std::{fs, path::Path};

use incular_devtools::preview_gitignore::{
    PreviewIgnoreMigration as Outcome, migrate_preview_gitignore,
};

fn migrate(project: &Path) -> Outcome {
    migrate_preview_gitignore(project, Path::new(".widget_preview")).unwrap()
}

#[test]
fn a_missing_ignore_file_is_not_created() {
    let project = tempfile::tempdir().unwrap();
    assert_eq!(migrate(project.path()), Outcome::Missing);
    assert!(!project.path().join(".gitignore").exists());
}

#[test]
fn an_existing_ignore_rule_is_not_rewritten() {
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join(".gitignore");
    let text = ".DS_Store\n.widget_preview/\n";
    fs::write(&path, text).unwrap();
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(migrate(project.path()), Outcome::Unchanged);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
    assert_eq!(fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn a_missing_rule_is_added_below_existing_rules() {
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join(".gitignore");
    fs::write(&path, ".DS_Store\n.atom/\n").unwrap();
    assert_eq!(migrate(project.path()), Outcome::Updated);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        ".DS_Store\n.atom/\n# Widget Preview related\n.widget_preview/\n"
    );
    assert_eq!(migrate(project.path()), Outcome::Unchanged);
}

#[test]
fn preserves_crlf_and_appends_past_an_explicit_negation() {
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join(".gitignore");
    let original = b"# .widget_preview/\r\n.widget_preview/\r\n!.widget_preview/";
    fs::write(&path, original).unwrap();
    assert_eq!(migrate(project.path()), Outcome::Updated);
    let mut expected = original.to_vec();
    expected.extend_from_slice(b"\r\n# Widget Preview related\r\n.widget_preview/\r\n");
    assert_eq!(fs::read(&path).unwrap(), expected);
    assert_eq!(migrate(project.path()), Outcome::Unchanged);
}

#[test]
fn never_installs_a_rule_for_a_parent_directory() {
    let project = tempfile::tempdir().unwrap();
    let path = project.path().join(".gitignore");
    fs::write(&path, ".DS_Store\n").unwrap();
    let error = migrate_preview_gitignore(project.path(), Path::new("../different")).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(fs::read_to_string(&path).unwrap(), ".DS_Store\n");
}

#[cfg(unix)]
#[test]
fn does_not_modify_an_external_ignore_file_via_a_symlink() {
    let project = tempfile::tempdir().unwrap();
    let other = tempfile::NamedTempFile::new().unwrap();
    fs::write(other.path(), "outside\n").unwrap();
    let ignore = project.path().join(".gitignore");
    std::os::unix::fs::symlink(other.path(), &ignore).unwrap();
    let error =
        migrate_preview_gitignore(project.path(), Path::new(".widget_preview")).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(fs::symlink_metadata(&ignore).unwrap().is_symlink());
    assert_eq!(fs::read_to_string(other.path()).unwrap(), "outside\n");
}
