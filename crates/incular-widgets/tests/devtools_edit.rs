#![cfg(feature = "devtools")]

//! Typed DevTools property edits: every editable kind changes layout or paint
//! through the retained update path, survives an application rebuild, and
//! reverts cleanly.

use incular_config::{Constraints, MainAxisAlignment, MainAxisSize};
use incular_core::{Color, Size};
use incular_devtools_protocol::{DebugProperty, DebugValue, DevWidgetId, DevWindowId};
use incular_widgets::{
    Widget,
    devtools::DevEdit,
    internal::{ElementId, WidgetTree},
};

fn ltr_text(text: &str) -> Widget {
    incular_widgets::Text::new(text.to_owned())
        .text_direction(incular_widgets::TextDirection::Ltr)
        .into()
}

fn dev_id(tree: &WidgetTree, id: ElementId) -> DevWidgetId {
    tree.devtools_id_for_element(id).expect("devtools id")
}

fn properties(tree: &WidgetTree, id: ElementId) -> Vec<DebugProperty> {
    tree.devtools_node_details(id, dev_id(tree, id), DevWindowId::new(1, 0))
        .expect("details")
        .properties
}

fn property(tree: &WidgetTree, id: ElementId, name: &str) -> DebugProperty {
    properties(tree, id)
        .into_iter()
        .find(|property| property.name == name)
        .unwrap_or_else(|| panic!("missing property {name}"))
}

fn descendants(tree: &WidgetTree, root: ElementId) -> Vec<ElementId> {
    let mut out = vec![root];
    let mut index = 0;
    while index < out.len() {
        out.extend_from_slice(tree.children(out[index]).unwrap_or_default());
        index += 1;
    }
    out
}

fn find_with(tree: &WidgetTree, root: ElementId, name: &str) -> ElementId {
    descendants(tree, root)
        .into_iter()
        .find(|id| {
            properties(tree, *id)
                .iter()
                .any(|p| p.name == name && p.editable)
        })
        .unwrap_or_else(|| panic!("no editable {name}"))
}

fn edit(tree: &mut WidgetTree, id: ElementId, name: &str, value: DebugValue) -> DevEdit {
    let target = dev_id(tree, id);
    tree.devtools_edit_property(target, name, &value)
}

fn relayout(tree: &mut WidgetTree, size: Size) {
    tree.layout(Constraints::loose(size)).expect("layout");
}

#[test]
fn text_edits_change_layout_and_paint() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(ltr_text("hi")).expect("mount");
    relayout(&mut tree, Size::new(400., 200.));
    let before = tree.root_layout_size().unwrap();
    let painted = tree.paint();
    let size = property(&tree, root, "fontSize");
    assert!(size.editable && size.min.is_some() && size.max.is_some() && size.step.is_some());
    assert_eq!(
        edit(
            &mut tree,
            root,
            "text",
            DebugValue::Str("a much longer line".into())
        ),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(400., 200.));
    assert!(tree.root_layout_size().unwrap().width > before.width);
    assert_eq!(
        edit(&mut tree, root, "fontSize", DebugValue::Float(40.)),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(400., 200.));
    assert!(tree.root_layout_size().unwrap().height > before.height);
    assert_eq!(
        edit(&mut tree, root, "color", DebugValue::Color(255, 0, 0, 255)),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(400., 200.));
    assert!(tree.paint() != painted);
    assert_eq!(
        property(&tree, root, "color").value,
        DebugValue::Color(255, 0, 0, 255)
    );
    assert_eq!(
        edit(&mut tree, root, "fontSize", DebugValue::Str("x".into())),
        DevEdit::Rejected
    );
    assert_eq!(
        edit(&mut tree, root, "fontSize", DebugValue::Float(f64::NAN)),
        DevEdit::Rejected
    );
    assert_eq!(
        edit(&mut tree, root, "fontSize", DebugValue::Float(40.)),
        DevEdit::Unchanged
    );
}

#[test]
fn padding_and_sized_box_edits_resize_the_subtree() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            incular_widgets::Padding::all(
                8.,
                incular_widgets::SizedBox::from_dimensions(
                    Some(30.),
                    Some(20.),
                    Some(Widget::box_(Size::new(5., 5.), Color::WHITE)),
                ),
            )
            .into(),
        )
        .expect("mount");
    relayout(&mut tree, Size::new(400., 400.));
    assert_eq!(tree.root_layout_size(), Some(Size::new(46., 36.)));
    let padding = property(&tree, root, "padding");
    assert!(padding.editable && matches!(padding.value, DebugValue::Insets(_)));
    assert_eq!(
        edit(
            &mut tree,
            root,
            "padding",
            DebugValue::Insets([20., 4., 20., 4.])
        ),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(400., 400.));
    assert_eq!(tree.root_layout_size(), Some(Size::new(70., 28.)));
    assert_eq!(
        edit(
            &mut tree,
            root,
            "padding",
            DebugValue::Insets([-1., 0., 0., 0.])
        ),
        DevEdit::Rejected
    );
    let constrained = find_with(&tree, root, "width");
    assert_eq!(
        edit(&mut tree, constrained, "width", DebugValue::Float(100.)),
        DevEdit::Applied
    );
    assert_eq!(
        edit(&mut tree, constrained, "height", DebugValue::Float(50.)),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(400., 400.));
    assert_eq!(tree.root_layout_size(), Some(Size::new(140., 58.)));
}

#[test]
fn flex_alignment_edits_move_children() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            incular_widgets::Row::new(vec![
                Widget::box_(Size::new(20., 10.), Color::WHITE),
                Widget::box_(Size::new(20., 30.), Color::WHITE),
            ])
            .text_direction(incular_widgets::TextDirection::Ltr)
            .main_axis_alignment(MainAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Max)
            .into(),
        )
        .expect("mount");
    relayout(&mut tree, Size::new(200., 100.));
    let first = tree.children(root).unwrap()[0];
    assert_eq!(tree.element_bounds(first).unwrap().origin.x, 0.);
    let main = property(&tree, root, "mainAxisAlignment");
    assert!(main.editable && main.options.contains(&"SpaceBetween".to_owned()));
    assert_eq!(
        edit(
            &mut tree,
            root,
            "mainAxisAlignment",
            DebugValue::Enum("End".into())
        ),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(200., 100.));
    assert_eq!(tree.element_bounds(first).unwrap().origin.x, 160.);
    assert_eq!(
        edit(
            &mut tree,
            root,
            "mainAxisSize",
            DebugValue::Enum("Min".into())
        ),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(200., 100.));
    assert_eq!(tree.root_layout_size().unwrap().width, 40.);
    assert_eq!(
        edit(
            &mut tree,
            root,
            "mainAxisAlignment",
            DebugValue::Enum("Nope".into())
        ),
        DevEdit::Rejected
    );
    assert_eq!(
        edit(
            &mut tree,
            root,
            "crossAxisAlignment",
            DebugValue::Enum("End".into())
        ),
        DevEdit::Applied
    );
    assert_eq!(
        edit(&mut tree, root, "spacing", DebugValue::Float(10.)),
        DevEdit::Applied
    );
    relayout(&mut tree, Size::new(200., 100.));
    assert_eq!(tree.root_layout_size().unwrap().width, 50.);
}

#[test]
fn align_and_decoration_edits_apply() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            incular_widgets::Align::new(
                incular_config::Alignment::TOP_LEFT,
                incular_widgets::DecoratedBox::new(Widget::box_(Size::new(20., 20.), Color::WHITE))
                    .background(Color::rgba(10, 20, 30, 255)),
            )
            .into(),
        )
        .expect("mount");
    relayout(&mut tree, Size::new(100., 100.));
    let tight = Constraints::tight(Size::new(100., 100.));
    tree.layout(tight).expect("layout");
    let decorated = find_with(&tree, root, "radius");
    assert_eq!(tree.element_bounds(decorated).unwrap().origin.x, 0.);
    let alignment = property(&tree, root, "alignment");
    assert_eq!(alignment.value, DebugValue::Enum("TopLeft".into()));
    assert_eq!(alignment.options.len(), 9);
    assert_eq!(
        edit(
            &mut tree,
            root,
            "alignment",
            DebugValue::Enum("BottomRight".into())
        ),
        DevEdit::Applied
    );
    tree.layout(tight).expect("layout");
    let bounds = tree.element_bounds(decorated).unwrap();
    assert_eq!((bounds.origin.x, bounds.origin.y), (80., 80.));

    let painted = tree.paint();
    assert_eq!(
        edit(
            &mut tree,
            decorated,
            "background",
            DebugValue::Color(200, 0, 0, 255)
        ),
        DevEdit::Applied
    );
    tree.layout(tight).expect("layout");
    let recolored = tree.paint();
    assert!(recolored != painted);
    assert_eq!(
        edit(&mut tree, decorated, "radius", DebugValue::Float(6.)),
        DevEdit::Applied
    );
    assert_eq!(
        edit(&mut tree, decorated, "borderWidth", DebugValue::Float(2.)),
        DevEdit::Applied
    );
    assert_eq!(
        edit(
            &mut tree,
            decorated,
            "borderColor",
            DebugValue::Color(0, 255, 0, 255)
        ),
        DevEdit::Applied
    );
    tree.layout(tight).expect("layout");
    assert!(tree.paint() != recolored);
    assert_eq!(
        property(&tree, decorated, "radius").value,
        DebugValue::Float(6.)
    );
    assert_eq!(
        property(&tree, decorated, "borderColor").value,
        DebugValue::Color(0, 255, 0, 255)
    );
}

#[test]
fn opacity_edit_is_clamped_and_stays_compositor_only() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(
            incular_widgets::Opacity::new(1., Widget::box_(Size::new(10., 10.), Color::WHITE))
                .into(),
        )
        .expect("mount");
    relayout(&mut tree, Size::new(50., 50.));
    let opacity = property(&tree, root, "opacity");
    assert_eq!((opacity.min, opacity.max), (Some(0.), Some(1.)));
    assert_eq!(
        edit(&mut tree, root, "opacity", DebugValue::Float(7.)),
        DevEdit::Unchanged
    );
    assert_eq!(
        edit(&mut tree, root, "opacity", DebugValue::Float(0.5)),
        DevEdit::Applied
    );
    assert_eq!(
        property(&tree, root, "opacity").value,
        DebugValue::Float(0.5)
    );
}

#[test]
fn overrides_survive_application_rebuilds_until_reverted() {
    let build = || -> Widget {
        incular_widgets::Padding::all(4., Widget::box_(Size::new(10., 10.), Color::WHITE)).into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(build()).expect("mount");
    relayout(&mut tree, Size::new(100., 100.));
    assert_eq!(tree.root_layout_size(), Some(Size::new(18., 18.)));
    assert_eq!(
        edit(
            &mut tree,
            root,
            "padding",
            DebugValue::Insets([10., 10., 10., 10.])
        ),
        DevEdit::Applied
    );
    // The application rebuilds with its own (original) configuration.
    tree.update(root, build()).expect("rebuild");
    relayout(&mut tree, Size::new(100., 100.));
    assert_eq!(tree.root_layout_size(), Some(Size::new(30., 30.)));
    assert_eq!(
        property(&tree, root, "padding").value,
        DebugValue::Insets([10., 10., 10., 10.])
    );
    let target = dev_id(&tree, root);
    assert_eq!(
        tree.devtools_revert_property(target, "padding", &DebugValue::Insets([4., 4., 4., 4.])),
        DevEdit::Applied
    );
    tree.update(root, build()).expect("rebuild");
    relayout(&mut tree, Size::new(100., 100.));
    assert_eq!(tree.root_layout_size(), Some(Size::new(18., 18.)));
}

#[test]
fn stale_ids_and_read_only_properties_are_rejected() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(ltr_text("hello")).expect("mount");
    relayout(&mut tree, Size::new(100., 100.));
    assert!(!property(&tree, root, "softWrap").editable);
    assert_eq!(
        edit(&mut tree, root, "softWrap", DebugValue::Bool(false)),
        DevEdit::Rejected
    );
    let stale = DevWidgetId::new(9_999, 3);
    assert_eq!(
        tree.devtools_edit_property(stale, "text", &DebugValue::Str("x".into())),
        DevEdit::Rejected
    );
}
