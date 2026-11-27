use std::hash::{DefaultHasher, Hash, Hasher};

use incular_config::Constraints;
use incular_core::{Offset, Size, TextDirection};
use incular_text::{
    TextAffinity, TextEditingController, TextSelection, TextStyle, register_application_font_as,
};
use incular_widgets::{EditableText, TextSelectionPoint, Transform, Widget, internal::WidgetTree};

fn hash(point: TextSelectionPoint) -> u64 {
    let mut state = DefaultHasher::new();
    point.hash(&mut state);
    state.finish()
}

#[test]
fn text_selection_points_compare_position_and_direction() {
    let point = TextSelectionPoint::new(Offset::new(1., 2.), Some(TextDirection::Ltr));
    let same = TextSelectionPoint::new(Offset::new(1., 2.), Some(TextDirection::Ltr));
    let different = TextSelectionPoint::new(Offset::new(2., 2.), Some(TextDirection::Ltr));
    assert_eq!(point, same);
    assert_eq!(hash(point), hash(same));
    assert_ne!(point, different);
    assert_ne!(hash(point), hash(different));
    assert_ne!(
        point,
        TextSelectionPoint::new(point.point, Some(TextDirection::Rtl))
    );
    assert_ne!(point, TextSelectionPoint::new(point.point, None));
    let zero = TextSelectionPoint::new(Offset::ZERO, None);
    let negative_zero = TextSelectionPoint::new(Offset::new(-0., 0.), None);
    assert_eq!(zero, negative_zero);
    assert_eq!(hash(zero), hash(negative_zero));
}

#[test]
fn a_zero_width_directional_mark_yields_a_single_directionless_endpoint() {
    let controller = TextEditingController::with_text("\u{200e}");
    let mut tree = WidgetTree::new();
    let editable = EditableText::new(controller.clone()).text_direction(TextDirection::Ltr);
    let id = tree.mount(editable.into()).unwrap();
    let selection = TextSelection::new(0, "\u{200e}".len());
    assert_eq!(
        tree.editable_selection_endpoints(id, selection, TextAffinity::Downstream),
        None
    );
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .unwrap();
    let points = tree
        .editable_selection_endpoints(id, selection, TextAffinity::Downstream)
        .unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].point.x, 0.);
    assert!(points[0].point.y.is_finite());
    assert_eq!(points[0].direction, None);
    assert_eq!(
        tree.editable_selection_endpoints(id, TextSelection::new(0, 1), TextAffinity::Downstream),
        None
    );
    assert!(controller.selection_if_set().is_none());
}

#[test]
fn shaped_range_endpoints_include_the_viewport_offset_but_not_ancestor_translation() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let family = "Editable endpoint pinned Ahem";
    register_application_font_as(family, std::fs::read(path).unwrap());
    let controller = TextEditingController::with_text("ab");
    let child = Widget::from(
        EditableText::new(controller)
            .text_direction(TextDirection::Ltr)
            .style(
                TextStyle::new()
                    .font_family(family)
                    .font_size(10.)
                    .height(Some(1.)),
            )
            .fixed_scroll_offset(Some(5.)),
    )
    .with_key("editable endpoints");
    let mut tree = WidgetTree::new();
    tree.mount(Transform::translation(Offset::new(30., 25.), child).into())
        .unwrap();
    tree.layout(Constraints::loose(Size::new(100., 100.)))
        .unwrap();
    let id = tree.element_with_key(&"editable endpoints".into()).unwrap();
    let endpoints = |selection| {
        tree.editable_selection_endpoints(id, selection, TextAffinity::Downstream)
            .unwrap()
    };
    let range = [
        TextSelectionPoint::new(Offset::new(-5., 10.), Some(TextDirection::Ltr)),
        TextSelectionPoint::new(Offset::new(15., 10.), Some(TextDirection::Ltr)),
    ];
    assert_eq!(endpoints(TextSelection::new(0, 2)), range);
    assert_eq!(endpoints(TextSelection::new(2, 0)), range);
    assert_eq!(
        endpoints(TextSelection::collapsed(1)),
        [TextSelectionPoint::new(Offset::new(5., 10.), None)]
    );
}
