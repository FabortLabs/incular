use std::{fs, path::Path, process::Command};

use incular_devtools::preview_connection_info::PreviewConnectionInfo;

#[test]
fn generated_connection_module_preserves_the_four_host_values() {
    let scaffold = tempfile::tempdir().unwrap();
    let root = Path::new("/user/incular_project");
    let info = PreviewConnectionInfo {
        uri: "ws://localhost:1234",
        service: "widget-preview-service",
        stream: "widget-preview-stream",
        project_root: root,
    };
    let path = scaffold.path().join(PreviewConnectionInfo::RELATIVE_PATH);
    assert!(!path.exists());
    assert_eq!(info.write(scaffold.path()).unwrap(), path);
    let source = fs::read_to_string(&path).unwrap();
    assert_eq!(
        source,
        concat!(
            "// Generated preview connection metadata.\n",
            "pub const DTD_URI: &str = \"ws://localhost:1234\";\n",
            "pub const WIDGET_PREVIEW_SERVICE: &str = \"widget-preview-service\";\n",
            "pub const WIDGET_PREVIEW_SCAFFOLD_STREAM: &str = \"widget-preview-stream\";\n",
            "pub const PROJECT_ROOT_PATH: &str = \"/user/incular_project\";\n",
        )
    );

    let moved = PreviewConnectionInfo {
        uri: "ws://localhost:4321",
        project_root: Path::new("/user/a project with \"quotes\" and a \\ backslash"),
        ..info
    };
    moved.write(scaffold.path()).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), moved.render().unwrap());
    assert!(!fs::read_to_string(&path).unwrap().contains(":1234"));
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);

    if let Some(rustc) = std::env::var_os("INCULAR_PREVIEW_RUSTC_SMOKE") {
        let driver = scaffold.path().join("run.rs");
        let binary = scaffold
            .path()
            .join(if cfg!(windows) { "run.exe" } else { "run" });
        fs::write(
            &driver,
            format!(
                "include!({:?}); fn main() {{ assert_eq!(DTD_URI, {:?}); assert_eq!(PROJECT_ROOT_PATH, {:?}); }}",
                PreviewConnectionInfo::RELATIVE_PATH,
                moved.uri,
                moved.project_root.to_str().unwrap(),
            ),
        ).unwrap();
        let compiled = Command::new(rustc)
            .arg("--edition=2024")
            .arg(&driver)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        assert!(Command::new(binary).status().unwrap().success());
    }
}

#[cfg(unix)]
#[test]
fn non_unicode_root_cannot_overwrite_a_valid_generated_file() {
    use incular_devtools::preview_connection_info::PreviewConnectionInfoError;
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let scaffold = tempfile::tempdir().unwrap();
    let info = PreviewConnectionInfo {
        uri: "ws://localhost:1234",
        service: "preview-service",
        stream: "preview-stream",
        project_root: Path::new("/valid/root"),
    };
    let expected = info.render().unwrap();
    let path = info.write(scaffold.path()).unwrap();
    let invalid = PreviewConnectionInfo {
        project_root: Path::new(OsStr::from_bytes(b"/invalid/\xff")),
        ..info
    };
    assert!(matches!(
        invalid.write(scaffold.path()),
        Err(PreviewConnectionInfoError::NonUnicodeRoot(_))
    ));
    assert_eq!(fs::read_to_string(path).unwrap(), expected);
}
