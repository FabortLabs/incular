use std::{fs, path::PathBuf, process::Command};

use incular_devtools::preview_codegen::{
    DiscoveredPreview, PreviewCodegenError as Error, PreviewHelper, PreviewImport,
    PreviewImportAllocator, PreviewNamespace as Ns, generate_preview_registry,
};

fn package(path: &str) -> Ns {
    Ns::Package(path.to_owned())
}

fn sdk(path: &str) -> Ns {
    Ns::Sdk(path.to_owned())
}

fn file(path: &str) -> Ns {
    Ns::LocalFile(PathBuf::from(path))
}

fn prefix(namespace: Ns, prefix: u32) -> PreviewImport {
    PreviewImport { namespace, prefix }
}

#[test]
fn seeds_filter_unimportable_files_and_reserve_the_highest_alias() {
    let mut allocator = PreviewImportAllocator::default();
    let retained = [
        prefix(Ns::Helper(PreviewHelper::WidgetPreview), 1),
        prefix(Ns::Helper(PreviewHelper::Utils), 2),
        prefix(package("foo::foo"), 3),
        prefix(package("incular::widgets"), 5),
        prefix(sdk("std::time"), 6),
    ];
    allocator
        .seed([
            retained[0].clone(),
            retained[1].clone(),
            retained[2].clone(),
            prefix(file("preview.rs"), 4),
            retained[3].clone(),
            retained[4].clone(),
            prefix(file("root.rs"), 7),
        ])
        .unwrap();
    assert_eq!(allocator.imports(), retained);
    let bar = package("bar::bar");
    assert_eq!(allocator.qualify(Some(&bar), "Bar").unwrap(), "_i8::Bar");
    assert_eq!(
        allocator.qualify(Some(&bar), "build").unwrap(),
        "_i8::build"
    );
    assert_eq!(
        allocator
            .qualify(Some(&retained[2].namespace), "Foo")
            .unwrap(),
        "_i3::Foo"
    );
    assert_eq!(allocator.imports().last().unwrap().prefix, 8);
}

#[test]
fn an_existing_allocation_blocks_late_seed_without_changing_it() {
    let mut allocator = PreviewImportAllocator::default();
    assert_eq!(
        allocator
            .qualify(Some(&package("foo::foo")), "Foo")
            .unwrap(),
        "_i1::Foo"
    );
    assert_eq!(
        allocator.seed([prefix(sdk("std::time"), 1)]),
        Err(Error::AlreadyAllocated)
    );
    assert_eq!(allocator.imports(), [prefix(package("foo::foo"), 1)]);
    assert_eq!(
        allocator
            .qualify(Some(&package("bar::bar")), "Bar")
            .unwrap(),
        "_i2::Bar"
    );
}

fn preview(
    package_name: Option<&str>,
    library: Ns,
    function: &str,
    name: &str,
    uri: &str,
) -> DiscoveredPreview {
    DiscoveredPreview {
        package: package_name.map(str::to_owned),
        library,
        function: function.to_owned(),
        source_uri: uri.to_owned(),
        line: 4,
        column: 1,
        name: Some(name.to_owned()),
    }
}

#[test]
fn generating_from_analysis_ignores_root_previews_without_a_cargo_package() {
    let dependencies = [
        prefix(Ns::Helper(PreviewHelper::WidgetPreview), 1),
        prefix(Ns::Helper(PreviewHelper::Utils), 2),
        prefix(package("foo_project::preview"), 3),
        prefix(file("preview.rs"), 4),
        prefix(package("incular::widgets"), 5),
        prefix(sdk("std::time"), 6),
    ];
    let inputs = [
        preview(
            Some("foo_project"),
            package("foo_project::preview"),
            "valid_preview",
            "valid",
            "file:///user/incular_project/src/preview.rs",
        ),
        preview(
            None,
            file("preview.rs"),
            "root_preview",
            "root",
            "file:///user/incular_project/preview.rs",
        ),
    ];
    let generated = generate_preview_registry(dependencies, &inputs).unwrap();
    for expected in [
        "use crate::widget_preview as _i1;",
        "use crate::utils as _i2;",
        "use ::foo_project::preview as _i3;",
        "use ::incular::widgets as _i5;",
        "use ::std::time as _i6;",
        "package: \"foo_project\", source_uri: \"file:///user/incular_project/src/preview.rs\", line: 4, column: 1, name: Some(\"valid\"), build: _i3::valid_preview",
    ] {
        assert!(
            generated.contains(expected),
            "missing {expected:?} in {generated}"
        );
    }
    assert!(!generated.contains("_i4"));
    assert!(!generated.contains("root_preview"));
    assert!(!generated.contains("Some(\"root\")"));

    if let Some(rustc) = std::env::var_os("INCULAR_PREVIEW_RUSTC_SMOKE") {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("preview.rs");
        let binary = tmp.path().join(if cfg!(windows) {
            "preview.exe"
        } else {
            "preview"
        });
        let mut fixture = String::from(concat!(
            "extern crate self as incular;\n",
            "extern crate self as foo_project;\n",
            "pub mod widget_preview {}\n",
            "pub mod utils {}\n",
            "pub mod widgets { pub struct Widget(pub &'static str); }\n",
            "pub mod preview { pub fn valid_preview() -> crate::widgets::Widget { crate::widgets::Widget(\"selectable content\") } }\n",
        ));
        fixture.push_str(&generated);
        fixture.push_str("fn main() { assert_eq!(PREVIEWS.len(), 1); assert_eq!(PREVIEWS[0].name, Some(\"valid\")); assert_eq!((PREVIEWS[0].build)().0, \"selectable content\"); }\n");
        fs::write(&path, fixture).unwrap();
        let compile = Command::new(rustc)
            .arg("--edition=2024")
            .arg(&path)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&binary).output().unwrap();
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
}

#[test]
fn malformed_imports_and_aliases_do_not_modify_a_registry() {
    let mut allocator = PreviewImportAllocator::default();
    assert_eq!(allocator.qualify(None, "available").unwrap(), "available");
    let broken = package("foo; other");
    assert_eq!(
        allocator.seed([prefix(package("ok"), 1), prefix(broken.clone(), 2)]),
        Err(Error::InvalidPath("foo; other".to_owned()))
    );
    assert!(allocator.imports().is_empty());
    assert_eq!(
        allocator.seed([prefix(package("ok"), 1), prefix(file("other.rs"), 1)]),
        Err(Error::InvalidPrefix(1))
    );
    assert_eq!(
        allocator.qualify(Some(&package("ok")), "foo();"),
        Err(Error::InvalidSymbol("foo();".into()))
    );
    assert_eq!(
        allocator.qualify(Some(&package("ok")), "build").unwrap(),
        "_i1::build"
    );
}
