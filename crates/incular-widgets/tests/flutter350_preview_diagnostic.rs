use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, PointerPhase, Size, ValueNotifier, time::Instant};
use incular_gestures::PointerEvent;
use incular_semantics::Role;
use incular_widgets::{
    Directionality, Text,
    internal::WidgetTree,
    widget_previews::{
        Preview, PreviewBuilder, PreviewDiagnostic, PreviewDiagnosticFrame, PreviewGallery,
        PreviewGalleryEntry, PreviewSourceLocation,
    },
};

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(800., 600.)))
        .unwrap();
    tree.update_semantics();
}

fn node(tree: &WidgetTree, label: &str) -> (Role, Offset, Option<String>) {
    let (_, node) = tree
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("{label}: {}", tree.semantics_debug_dump()));
    (
        node.role,
        node.bounds.origin + Offset::new(2., 2.),
        node.state.link_url.clone(),
    )
}

fn tap(tree: &mut WidgetTree, point: Offset, pointer: u64) {
    let time = Instant::now();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        tree.dispatch_gesture(PointerEvent {
            pointer,
            position: point,
            phase,
            time,
        });
    }
}

#[test]
fn failed_factory_keeps_preview_name_and_only_live_editor_links_open_sources() {
    const FILE: &str = "test/error_widget_test.dart 46:23";
    const SDK: &str = "dart:async 12:1";
    const PACKAGE: &str = "package:gallery/src/host.dart 13:5";
    const UNPARSED: &str = "<anonymous>";
    let file = PreviewSourceLocation {
        uri: "file:///preview/test/error_widget_test.dart".into(),
        line: Some(46),
        column: Some(23),
    };
    let package = PreviewSourceLocation {
        uri: "package:gallery/src/host.dart".into(),
        line: Some(13),
        column: Some(5),
    };
    let editor = ValueNotifier::new(true);
    let navigation = Rc::new(RefCell::new(Vec::new()));
    let diagnostic = PreviewDiagnostic::new(
        "Error!",
        [
            PreviewDiagnosticFrame {
                text: FILE.into(),
                source: Some(file.clone()),
            },
            PreviewDiagnosticFrame {
                text: SDK.into(),
                source: Some(PreviewSourceLocation {
                    uri: "dart:async".into(),
                    line: Some(12),
                    column: Some(1),
                }),
            },
            PreviewDiagnosticFrame {
                text: PACKAGE.into(),
                source: Some(package.clone()),
            },
            PreviewDiagnosticFrame {
                text: UNPARSED.into(),
                source: None,
            },
        ],
        180.,
    )
    .editor(editor.clone(), {
        let navigation = navigation.clone();
        move |location| navigation.borrow_mut().push(location.clone())
    });

    let mut preview = PreviewBuilder::new();
    preview.name = Some("Broken preview".into());
    let failed = PreviewDiagnostic::build_preview(move || Err(diagnostic.clone()));
    let mut tree = WidgetTree::new();
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            PreviewGallery::new(
                [
                    PreviewGalleryEntry::new(preview.build(), failed),
                    PreviewGalleryEntry::new(Preview::default(), Text::new("Healthy preview")),
                ],
                "https://flutter.dev/to/widget-previews",
            ),
        )
        .into(),
    )
    .unwrap();
    layout(&mut tree);
    for title in [
        "Failed to initialize widget tree: Error!",
        "Broken preview",
        "Healthy preview",
        "Stacktrace:",
        UNPARSED,
    ] {
        assert_ne!(node(&tree, title).0, Role::Link);
    }

    let (role, file_point, link) = node(&tree, FILE);
    assert_eq!(role, Role::Link);
    assert_eq!(link, Some(file.uri.clone()));
    let (role, sdk_point, link) = node(&tree, SDK);
    assert_ne!(role, Role::Link);
    assert_eq!(link, None);
    let (role, package_point, link) = node(&tree, PACKAGE);
    assert_eq!(role, Role::Link);
    assert_eq!(link, Some(package.uri.clone()));
    for (pointer, point) in [file_point, sdk_point, package_point]
        .into_iter()
        .enumerate()
    {
        tap(&mut tree, point, pointer as u64 + 1);
    }
    assert_eq!(*navigation.borrow(), [file.clone(), package]);

    // Toggling availability disables even a tap before its rebuild is drawn.
    assert!(editor.set_value(false));
    tap(&mut tree, file_point, 4);
    assert_eq!(navigation.borrow().len(), 2);
    layout(&mut tree);
    for (pointer, label) in [FILE, SDK, PACKAGE, UNPARSED].into_iter().enumerate() {
        let (role, point, link) = node(&tree, label);
        assert_ne!(role, Role::Link);
        assert_eq!(link, None);
        tap(&mut tree, point, pointer as u64 + 5);
    }
    assert_eq!(navigation.borrow().len(), 2);

    assert!(editor.set_value(true));
    layout(&mut tree);
    let (role, point, link) = node(&tree, FILE);
    assert_eq!(role, Role::Link);
    assert_eq!(link, Some(file.uri.clone()));
    tap(&mut tree, point, 9);
    assert_eq!(navigation.borrow().last(), Some(&file));
}
