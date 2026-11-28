use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, PointerPhase, Size, time::Instant};
use incular_gestures::PointerEvent;
use incular_semantics::Role;
use incular_widgets::{
    Directionality, Text, Widget,
    internal::WidgetTree,
    widget_previews::{Preview, PreviewBuilder, PreviewGallery, PreviewGalleryEntry},
};

const DOCS: &str = "https://flutter.dev/to/widget-previews";

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(800., 600.)))
        .unwrap();
    tree.update_semantics();
}

#[test]
fn empty_gallery_exposes_help_and_its_link_until_previews_arrive() {
    let opened = Rc::new(RefCell::new(Vec::new()));
    let scene = |entries: Vec<PreviewGalleryEntry>| {
        let opened = opened.clone();
        Directionality::new(
            TextDirection::Ltr,
            PreviewGallery::new(entries, DOCS)
                .on_open_documentation(move |url| opened.borrow_mut().push(url.to_owned())),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(Vec::new())).unwrap();
    layout(&mut tree);
    let nodes = tree
        .semantics()
        .iter()
        .map(|(_, node)| node)
        .collect::<Vec<_>>();
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.label.as_deref() == Some("No previews detected"))
            .count(),
        1
    );
    assert!(!nodes.iter().any(|node| node.role == Role::Group));
    let link = nodes.iter().find(|node| node.role == Role::Link).unwrap();
    assert_eq!(link.label.as_deref(), Some(DOCS));
    let position = link.bounds.origin + Offset::new(2., 2.);
    let instant = Instant::now();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        tree.dispatch_gesture(PointerEvent {
            pointer: 1,
            position,
            phase,
            time: instant,
        });
    }
    assert_eq!(&*opened.borrow(), &[DOCS]);

    tree.update(
        root,
        scene(vec![PreviewGalleryEntry::new(
            Preview::default(),
            Text::new("Foo"),
        )]),
    )
    .unwrap();
    layout(&mut tree);
    let nodes = tree
        .semantics()
        .iter()
        .map(|(_, node)| node)
        .collect::<Vec<_>>();
    assert_eq!(
        nodes.iter().filter(|node| node.role == Role::Group).count(),
        1
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.label.as_deref() == Some("Foo"))
            .count(),
        1
    );
    assert!(!nodes.iter().any(
        |node| node.role == Role::Link || node.label.as_deref() == Some("No previews detected")
    ));

    tree.update(root, scene(Vec::new())).unwrap();
    layout(&mut tree);
    assert!(
        tree.semantics()
            .iter()
            .any(|(_, node)| node.role == Role::Link)
    );
    assert_eq!(&*opened.borrow(), &[DOCS]);
}

#[test]
fn group_order_and_named_headers_follow_supplied_preview_definitions() {
    let description = |group: &str, name: &str, content: &str| {
        let mut preview = PreviewBuilder::new();
        preview.group = Some(group.to_owned());
        preview.name = Some(name.to_owned());
        PreviewGalleryEntry::new(preview.build(), Text::new(content))
    };
    let entries = [
        description("b", "First", "first content"),
        description("a", "Next", "next content"),
        description("b", "Last", "last content"),
    ];
    let mut tree = WidgetTree::new();
    let gallery: Widget =
        Directionality::new(TextDirection::Ltr, PreviewGallery::new(entries, DOCS)).into();
    tree.mount(gallery).unwrap();
    layout(&mut tree);
    let semantic = tree.semantics();
    let groups = semantic
        .roots()
        .iter()
        .filter_map(|&id| semantic.node(id))
        .filter(|node| node.role == Role::Group)
        .collect::<Vec<_>>();
    assert_eq!(
        groups
            .iter()
            .map(|node| node.label.as_deref().unwrap())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
    for label in [
        "First",
        "Next",
        "Last",
        "first content",
        "next content",
        "last content",
    ] {
        assert!(
            semantic
                .iter()
                .any(|(_, node)| node.label.as_deref() == Some(label)),
            "{label}"
        );
    }
}

#[test]
fn gallery_renders_names_and_both_wrappers_of_each_transformed_preview() {
    use incular_config::Brightness;
    use incular_widgets::{
        SizedBox,
        widget_previews::{MultiPreview, PreviewTransform},
    };

    type Calls = Rc<RefCell<Vec<&'static str>>>;

    struct FixedSize(Calls);
    impl PreviewTransform for FixedSize {
        fn transform(&self) -> Preview {
            let mut fixed = PreviewBuilder::new();
            fixed.name = Some("Fixed Size".to_owned());
            let called = self.0.clone();
            fixed.add_wrapper(move |child| {
                called.borrow_mut().push("fixed");
                SizedBox::new().width(110.).height(40.).child(child).into()
            });
            fixed.build()
        }
    }

    struct BrightnessGroup {
        members: [FixedSize; 2],
        calls: Calls,
    }

    impl MultiPreview for BrightnessGroup {
        fn previews(&self) -> Vec<&dyn PreviewTransform> {
            self.members
                .iter()
                .map(|member| member as &dyn PreviewTransform)
                .collect()
        }

        fn transform(&self) -> Vec<Preview> {
            self.transform_members()
                .into_iter()
                .zip([("light", Brightness::Light), ("dark", Brightness::Dark)])
                .map(|(member, (label, brightness))| {
                    let mut options = member.to_builder();
                    options.name = Some(format!(
                        "MyPreview - {} - Brightness({label})",
                        member.name().unwrap()
                    ));
                    options.brightness = Some(brightness);
                    let called = self.calls.clone();
                    options.add_wrapper(move |child| {
                        called.borrow_mut().push("brightness");
                        child
                    });
                    options.build()
                })
                .collect()
        }
    }

    let calls = Calls::default();
    let group = BrightnessGroup {
        members: [FixedSize(calls.clone()), FixedSize(calls.clone())],
        calls: calls.clone(),
    };
    let entries = group
        .transform()
        .into_iter()
        .map(|preview| PreviewGalleryEntry::new(preview, Text::new("Foo")));
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, PreviewGallery::new(entries, DOCS)).into())
        .unwrap();
    layout(&mut tree);
    assert_eq!(
        &*calls.borrow(),
        &["fixed", "brightness", "fixed", "brightness"]
    );
    for label in [
        "MyPreview - Fixed Size - Brightness(light)",
        "MyPreview - Fixed Size - Brightness(dark)",
    ] {
        assert_eq!(
            tree.semantics()
                .iter()
                .filter(|(_, node)| node.label.as_deref() == Some(label))
                .count(),
            1
        );
    }
}
