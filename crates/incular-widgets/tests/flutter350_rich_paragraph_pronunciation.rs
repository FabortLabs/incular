use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_semantics::{Role, StringAttribute};
use incular_text::{RichText, TextSpan};
use incular_widgets::{Directionality, Widget, internal::WidgetTree};

fn semantics(span: TextSpan) -> (String, Vec<StringAttribute>, Role) {
    let widget: Widget = RichText::new(span).into();
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, widget).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(300., 100.)))
        .unwrap();
    tree.update_semantics();
    let labels: Vec<_> = tree
        .semantics()
        .iter()
        .filter_map(|(_, node)| {
            Some((
                node.label.as_ref()?.clone(),
                node.state
                    .text_attributes
                    .as_ref()
                    .map_or_else(Vec::new, |attrs| attrs.label.clone()),
                node.role,
            ))
        })
        .collect();
    let [text] = labels.as_slice() else {
        panic!("one spoken paragraph, found {labels:?}")
    };
    text.clone()
}

#[test]
fn mounted_rich_text_publishes_the_spoken_label_with_utf16_pronunciation() {
    let span = TextSpan::new("a").spell_out(true).child(
        TextSpan::new("b")
            .semantics_label("🦀")
            .locale("es-MX".parse().unwrap()),
    );
    assert_eq!(span.plain_text(), "ab");
    let (label, attributes, role) = semantics(span);
    assert_eq!(label, "a🦀");
    assert_eq!(role, Role::Text);
    assert_eq!(
        attributes,
        [
            StringAttribute::spell_out(0..1),
            StringAttribute::spell_out(1..3),
            StringAttribute::locale(1..3, "es-MX"),
        ]
    );
}

#[test]
fn regular_paragraphs_do_not_create_extra_semantics_or_synthetic_annotations() {
    let span = TextSpan::new("ab").child(TextSpan::new("c").semantics_label("see"));
    let (label, attributes, role) = semantics(span);
    assert_eq!(label, "absee");
    assert!(attributes.is_empty());
    assert_eq!(role, Role::Text);
}

#[test]
fn switching_locale_or_clearing_ranges_keeps_the_mounted_text_semantics_id() {
    let widget = |locale: Option<&str>| -> Widget {
        let mut span = TextSpan::new("hello");
        if let Some(locale) = locale {
            span = span.locale(locale.parse().unwrap());
        }
        Directionality::new(TextDirection::Ltr, RichText::new(span)).into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget(Some("es-MX"))).unwrap();
    let constraints = Constraints::loose(Size::new(300., 100.));
    tree.layout(constraints).unwrap();
    tree.update_semantics();
    let initial = tree
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("hello"))
        .unwrap()
        .0;

    for locale in [Some("fr-FR"), None] {
        tree.update(root, widget(locale)).unwrap();
        tree.layout(constraints).unwrap();
        tree.update_semantics();
        let (id, node) = tree
            .semantics()
            .iter()
            .find(|(_, node)| node.label.as_deref() == Some("hello"))
            .unwrap();
        assert_eq!(id, initial);
        if let Some(locale) = locale {
            assert_eq!(
                node.state.text_attributes.as_ref().unwrap().label,
                [StringAttribute::locale(0..5, locale)]
            );
        } else {
            assert!(node.state.text_attributes.is_none());
        }
    }
}

#[test]
fn an_empty_spell_out_span_never_publishes_a_zero_length_pronunciation_range() {
    let mut tree = WidgetTree::new();
    let text = RichText::new(TextSpan::empty().spell_out(true));
    tree.mount(Directionality::new(TextDirection::Rtl, text).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(200., 100.)))
        .unwrap();
    tree.update_semantics();
    let graph = tree.semantics();
    let [root] = graph.roots() else {
        panic!("expected the empty paragraph")
    };
    let paragraph = graph.node(*root).unwrap();
    assert_eq!(paragraph.label.as_deref().unwrap_or_default(), "");
    assert!(
        paragraph
            .state
            .text_attributes
            .as_ref()
            .is_none_or(|attrs| attrs.label.is_empty())
    );
    assert!(paragraph.children.is_empty());
}
