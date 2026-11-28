//! Mounted baseline propagation through transparent render proxies.
use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_rendering::GaussianBlur;
use incular_widgets::{BackdropFilter, Semantics, SizedBox, Text, Widget, internal::WidgetTree};

fn leaf(text: bool) -> Widget {
    if text {
        Text::new("Ag").text_direction(TextDirection::Ltr).into()
    } else {
        SizedBox::new().width(40.).height(20.).into()
    }
}

fn real_and_dry_baselines_follow_the_retained_child(scene: impl Fn(Option<Widget>) -> Widget) {
    let tight = Constraints::tight(Size::new(40., 20.));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(Some(leaf(false)))).unwrap();
    let render = tree.render_id(root).unwrap();
    for expected in [Some(false), Some(true), None, Some(false)] {
        tree.update(root, scene(expected.map(leaf))).unwrap();
        tree.layout(tight).unwrap();
        assert_eq!(tree.render_id(root), Some(render));
        assert_eq!(
            tree.dry_layout_size(render, tight),
            Some(Size::new(40., 20.))
        );
        let child = tree
            .children(root)
            .unwrap()
            .first()
            .and_then(|child| tree.render_id(*child));
        let actual = child.and_then(|child| tree.baseline(child));
        let dry = child.and_then(|child| tree.dry_alphabetic_baseline(child, tight));
        assert_eq!(actual.is_some(), expected == Some(true));
        assert_eq!(dry.is_some(), expected == Some(true));
        assert_eq!(tree.baseline(render), actual);
        assert_eq!(tree.dry_alphabetic_baseline(render, tight), dry);
    }
}

#[test]
fn semantics_annotation_forwards_dry_and_actual_baselines_without_inventing_one() {
    real_and_dry_baselines_follow_the_retained_child(|child| {
        Semantics::new(child.unwrap_or_else(|| SizedBox::shrink().into()))
            // Keep an actual annotation wrapper rather than merging the
            // semantics properties directly into the child's widget.
            .tag_for_children(None)
            .into()
    });
}

#[test]
fn a_childless_backdrop_proxy_has_no_dry_or_actual_baseline() {
    real_and_dry_baselines_follow_the_retained_child(|child| match child {
        Some(child) => BackdropFilter::blur(1., child).into(),
        None => BackdropFilter::without_child(GaussianBlur::uniform(1.)).into(),
    });
}
