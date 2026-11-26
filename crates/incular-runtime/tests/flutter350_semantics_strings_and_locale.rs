//! Pinned Flutter semantics text direction, locale scopes and value conflicts.

use incular_config::{Constraints, Locale, TextDirection};
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{Column, Container, Directionality, Semantics, SizedBox, Widget};

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

fn mount(child: impl Into<Widget>) -> Runtime {
    let mut runtime = Runtime::new(child.into()).unwrap();
    frame(&mut runtime);
    runtime
}

// widgets/semantics_test.dart:188
#[test]
fn locale_only_annotations_emit_distinct_nested_scopes() {
    let locale = |tag: &str| tag.parse::<Locale>().unwrap();
    let build = |inner: Locale| -> Widget {
        Directionality::new(
            TextDirection::Ltr,
            Semantics::new(Semantics::new(SizedBox::square(10.)).locale_for_subtree(inner))
                .locale_for_subtree(locale("ab-CD")),
        )
        .into()
    };
    let mut runtime = mount(build(locale("de-FG")));
    let root = runtime.tree().root().unwrap();
    let tree = runtime.tree().semantics();
    let [outer] = tree.roots() else {
        panic!("one outer scope, got {:?}", tree.roots());
    };
    let outer = *outer;
    let outer_node = tree.node(outer).unwrap();
    let [inner] = outer_node.children.as_slice() else {
        panic!(
            "one independent inner locale annotation, got {:?}",
            outer_node.children
        );
    };
    let inner = *inner;
    for tag in ["de-FG", "he-IL", "de-FG"] {
        runtime.tree_mut().update(root, build(locale(tag))).unwrap();
        frame(&mut runtime);
        let tree = runtime.tree().semantics();
        let parent = tree.node(outer).unwrap();
        let child = tree.node(inner).unwrap();
        assert_eq!(tree.roots(), &[outer]);
        assert_eq!(parent.children, [inner]);
        assert_eq!(parent.state.locale.as_deref(), Some("ab-CD"));
        assert_eq!(child.state.locale.as_deref(), Some(tag));
        for node in [parent, child] {
            assert_eq!(node.state.text_direction, None);
            assert!(node.label.is_none());
            assert!(node.value.is_none());
        }
        assert!(child.children.is_empty());
    }
}

// widgets/semantics_test.dart:303
#[test]
fn label_hint_and_value_keep_text_and_ambient_direction_on_one_node() {
    let child = Semantics::new(Container::new())
        .label("label")
        .hint("hint")
        .value("value");
    let runtime = mount(Directionality::new(TextDirection::Ltr, child));
    let tree = runtime.tree().semantics();
    let [root] = tree.roots() else {
        panic!("one text annotation");
    };
    let node = tree.node(*root).unwrap();
    assert!(node.children.is_empty());
    assert_eq!(node.label.as_deref(), Some("label"));
    assert_eq!(node.description.as_deref(), Some("hint"));
    assert_eq!(node.value.as_deref(), Some("value"));
    assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
}

// widgets/semantics_test.dart:362
#[test]
fn distinct_values_never_absorb_each_other_or_lose_direction() {
    let content = Widget::from(
        Semantics::new(Column::new([
            Semantics::new(SizedBox::square(10.)).value("value one"),
            Semantics::new(SizedBox::square(10.)).value("value two"),
        ]))
        .container(true),
    );
    let build = |direction| -> Widget { Directionality::new(direction, content.clone()).into() };
    let mut runtime = mount(build(TextDirection::Ltr));
    let root = runtime.tree().root().unwrap();
    for direction in [TextDirection::Ltr, TextDirection::Rtl, TextDirection::Ltr] {
        runtime.tree_mut().update(root, build(direction)).unwrap();
        frame(&mut runtime);
        let tree = runtime.tree().semantics();
        let [scope] = tree.roots() else {
            panic!("one container");
        };
        let scope = tree.node(*scope).unwrap();
        assert!(scope.value.is_none());
        assert!(scope.state.text_direction.is_none());
        assert_eq!(scope.children.len(), 2);
        for (&id, text) in scope.children.iter().zip(["value one", "value two"]) {
            let node = tree.node(id).unwrap();
            assert_eq!(node.value.as_deref(), Some(text));
            assert_eq!(node.state.text_direction, Some(direction));
        }
    }
}

#[test]
fn tooltip_hint_or_alternate_value_text_use_direction_without_a_label() {
    let hinted = Semantics::new(SizedBox::square(10.)).hint("info");
    let tooltip = Semantics::new(SizedBox::square(10.)).tooltip("preview");
    let increased = Semantics::new(SizedBox::square(10.)).increased_value("higher");
    let decreased = Semantics::new(SizedBox::square(10.)).decreased_value("lower");
    let runtime = mount(Directionality::new(
        TextDirection::Rtl,
        Column::new([hinted, tooltip, increased, decreased]),
    ));
    let tree = runtime.tree().semantics();
    assert_eq!(tree.roots().len(), 4);
    for id in tree.roots() {
        let node = tree.node(*id).unwrap();
        assert!(node.label.is_none());
        assert_eq!(node.state.text_direction, Some(TextDirection::Rtl));
    }
}

// widgets/semantics_test.dart:100
#[test]
fn a_childless_tooltip_keeps_its_explicit_direction() {
    let runtime = mount(
        Semantics::builder()
            .build()
            .tooltip("test1")
            .text_direction(TextDirection::Ltr),
    );
    let tree = runtime.tree().semantics();
    let [id] = tree.roots() else {
        panic!("one independent tooltip node");
    };
    let node = tree.node(*id).unwrap();
    assert_eq!(node.state.tooltip.as_deref(), Some("test1"));
    assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
    assert!(node.label.is_none() && node.description.is_none() && node.children.is_empty());
}

// widgets/semantics_test.dart:225,239,253,278
#[test]
fn labeled_annotations_inherit_direction_unless_explicitly_overridden() {
    for (ambient, explicit, expected) in [
        (TextDirection::Rtl, None, TextDirection::Rtl),
        (TextDirection::Ltr, None, TextDirection::Ltr),
        (
            TextDirection::Rtl,
            Some(TextDirection::Ltr),
            TextDirection::Ltr,
        ),
        (
            TextDirection::Ltr,
            Some(TextDirection::Rtl),
            TextDirection::Rtl,
        ),
    ] {
        let mut annotation = Semantics::new(Container::new()).label("test1");
        if let Some(direction) = explicit {
            annotation = annotation.text_direction(direction);
        }
        let runtime = mount(Directionality::new(ambient, annotation));
        let tree = runtime.tree().semantics();
        let [id] = tree.roots() else {
            panic!("one labeled annotation");
        };
        let node = tree.node(*id).unwrap();
        assert_eq!(node.label.as_deref(), Some("test1"));
        assert_eq!(node.state.text_direction, Some(expected));
        assert!(node.children.is_empty());
    }
}

// widgets/semantics_test.dart:1199
#[test]
fn current_and_alternate_values_with_adjust_actions_stay_on_the_same_node() {
    use incular_semantics::{SemanticAction, SemanticActionKind};
    use std::{cell::Cell, rc::Rc};

    let adjustment = Rc::new(Cell::new(0));
    let content = Semantics::builder()
        .build()
        .container(true)
        .value("10s")
        .increased_value("11s")
        .decreased_value("9s")
        .on_increase({
            let adjustment = adjustment.clone();
            move || adjustment.set(adjustment.get() + 1)
        })
        .on_decrease({
            let adjustment = adjustment.clone();
            move || adjustment.set(adjustment.get() - 1)
        });
    let mut runtime = mount(Directionality::new(TextDirection::Ltr, content));
    let tree = runtime.tree().semantics();
    let [id] = tree.roots() else {
        panic!("one adjustable semantic node");
    };
    let id = *id;
    let node = tree.node(id).unwrap();
    assert_eq!(node.value.as_deref(), Some("10s"));
    assert_eq!(node.state.increased_value.as_deref(), Some("11s"));
    assert_eq!(node.state.decreased_value.as_deref(), Some("9s"));
    assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
    assert_eq!(
        node.actions,
        [SemanticActionKind::Increment, SemanticActionKind::Decrement]
    );
    assert!(node.children.is_empty());
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Increment));
    assert_eq!(adjustment.get(), 1);
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::Decrement));
    assert_eq!(adjustment.get(), 0);
}

// widgets/semantics_test.dart:331,398
#[test]
fn compatible_hints_and_a_single_value_merge_into_the_container() {
    let hint = |text| Semantics::builder().build().hint(text);
    let value = |text| Semantics::builder().build().value(text);
    for (children, expected_hint, expected_value) in [
        (
            [hint("hint one"), hint("hint two")],
            "hint one\nhint two",
            None,
        ),
        ([hint("hint"), value("value")], "hint", Some("value")),
    ] {
        let runtime = mount(Directionality::new(
            TextDirection::Ltr,
            Semantics::new(Column::new(children)).container(true),
        ));
        let tree = runtime.tree().semantics();
        let [id] = tree.roots() else {
            panic!("one semantic container");
        };
        let node = tree.node(*id).unwrap();
        assert_eq!(node.description.as_deref(), Some(expected_hint));
        assert_eq!(node.value.as_deref(), expected_value);
        assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
        assert!(node.children.is_empty());
    }
}

#[test]
fn hint_absorption_tracks_new_value_conflicts_without_losing_boundaries() {
    let build = |second_value| -> Widget {
        let mut second = Semantics::new(SizedBox::square(10.)).hint("second hint");
        if second_value {
            second = second.value("second");
        }
        let first = Semantics::new(SizedBox::square(10.))
            .hint("first hint")
            .value("first");
        let boundary = Semantics::new(SizedBox::square(10.))
            .hint("separate")
            .container(true);
        Directionality::new(
            TextDirection::Ltr,
            Semantics::new(Column::new([first, boundary, second])).container(true),
        )
        .into()
    };
    let mut runtime = mount(build(false));
    let root = runtime.tree().root().unwrap();
    let id = runtime.tree().semantics().roots()[0];
    let [boundary] = runtime
        .tree()
        .semantics()
        .node(id)
        .unwrap()
        .children
        .as_slice()
    else {
        panic!("only explicit boundary when the other fragments are compatible");
    };
    let boundary = *boundary;
    for (two, hint, value) in [
        (false, Some("first hint\nsecond hint"), Some("first")),
        (true, None, None),
        (false, Some("first hint\nsecond hint"), Some("first")),
    ] {
        runtime.tree_mut().update(root, build(two)).unwrap();
        frame(&mut runtime);
        let semantics = runtime.tree().semantics();
        assert_eq!(semantics.roots(), &[id]);
        let owner = semantics.node(id).unwrap();
        assert_eq!(owner.description.as_deref(), hint);
        assert_eq!(owner.value.as_deref(), value);
        assert_eq!(
            semantics.node(boundary).unwrap().description.as_deref(),
            Some("separate")
        );
        assert_eq!(owner.children.len(), if two { 3 } else { 1 });
        if two {
            assert_eq!(owner.children[1], boundary);
            let first = semantics.node(owner.children[0]).unwrap();
            let second = semantics.node(owner.children[2]).unwrap();
            assert_eq!(first.value.as_deref(), Some("first"));
            assert_eq!(second.value.as_deref(), Some("second"));
            assert_eq!(first.description.as_deref(), Some("first hint"));
            assert_eq!(second.description.as_deref(), Some("second hint"));
        } else {
            assert_eq!(owner.children[0], boundary);
        }
    }
}

#[test]
fn merged_hints_rebase_utf16_attributes_and_keep_the_single_values_attributes() {
    use incular_semantics::{AttributedString, StringAttribute};

    let rtl_hint =
        AttributedString::try_with_attributes("אב", [StringAttribute::spell_out(0..2)]).unwrap();
    let value =
        AttributedString::try_with_attributes("five", [StringAttribute::spell_out(0..4)]).unwrap();
    let runtime = mount(Directionality::new(
        TextDirection::Ltr,
        Semantics::new(Column::new([
            Semantics::new(SizedBox::square(10.)).hint("😀"),
            Semantics::new(SizedBox::square(10.))
                .attributed_hint(rtl_hint)
                .text_direction(TextDirection::Rtl),
            Semantics::new(SizedBox::square(10.)).attributed_value(value),
        ]))
        .container(true),
    ));
    let semantics = runtime.tree().semantics();
    let [id] = semantics.roots() else {
        panic!("one retained text group");
    };
    let node = semantics.node(*id).unwrap();
    assert!(node.children.is_empty());
    assert_eq!(node.description.as_deref(), Some("😀\n\u{202b}אב\u{202c}"));
    assert_eq!(node.value.as_deref(), Some("five"));
    let attributes = node.state.text_attributes.as_deref().unwrap();
    assert_eq!(attributes.hint, [StringAttribute::spell_out(4..6)]);
    assert_eq!(attributes.value, [StringAttribute::spell_out(0..4)]);
    assert_eq!(node.state.text_direction, Some(TextDirection::Ltr));
}
