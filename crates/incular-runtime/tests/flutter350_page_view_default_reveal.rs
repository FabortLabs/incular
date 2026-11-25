//! Portable ShowOnScreen uses the actual retained page and viewport geometry.
use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_semantics::{SemanticAction, SemanticActionKind};
use incular_widgets::{Directionality, PageController, PageView, Semantics, SizedBox, Widget};

#[test]
fn a_partially_visible_page_has_semantics_but_default_reveal_does_not_scroll() {
    let controller = PageController::new().with_viewport_fraction(0.625);
    let page = PageView::new([
        Widget::from(SizedBox::expand()),
        Widget::from(
            Semantics::new(SizedBox::expand())
                .label("Adjacent page")
                .button(true),
        ),
        Widget::from(SizedBox::expand()),
    ])
    .controller(controller.clone())
    .pad_ends(false);
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, page).into()).unwrap();
    let constraints = Constraints::tight(Size::new(400., 200.));
    runtime.run_frame(constraints).unwrap();

    let (id, node) = runtime
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Adjacent page"))
        .expect("the partly visible neighbor contributes semantics");
    let expected = Rect::from_origin_size(Offset::new(250., 0.), Size::new(250., 200.));
    assert_eq!(node.bounds, expected);
    assert!(node.actions.contains(&SemanticActionKind::ShowOnScreen));
    assert_eq!(controller.offset(), 0.);

    let _ = runtime.dispatch_semantic_action(id, SemanticAction::ShowOnScreen);
    runtime.run_frame(constraints).unwrap();
    assert_eq!(
        controller.offset(),
        0.,
        "default page view cannot reveal more of its neighbor"
    );
}

#[test]
fn a_default_page_view_passes_an_implicit_reveal_to_an_outer_scroll_view() {
    use incular_widgets::{Column, ScrollController, SingleChildScrollView};

    let inner = PageController::new().with_viewport_fraction(0.625);
    let outer = ScrollController::new();
    let page = PageView::new([
        Widget::from(SizedBox::expand()),
        Widget::from(
            Semantics::new(SizedBox::expand())
                .label("Nested adjacent page")
                .button(true),
        ),
        Widget::from(SizedBox::expand()),
    ])
    .controller(inner.clone())
    .pad_ends(false);
    let root = SingleChildScrollView::new(Column::new([
        Widget::from(SizedBox::new().height(100.)),
        Widget::from(SizedBox::new().height(200.).child(page)),
    ]))
    .controller(outer.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, root).into()).unwrap();
    let constraints = Constraints::tight(Size::new(400., 200.));
    runtime.run_frame(constraints).unwrap();
    let (id, node) = runtime
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.label.as_deref() == Some("Nested adjacent page"))
        .expect("partly visible nested neighbor");
    assert_eq!(
        node.bounds,
        Rect::from_origin_size(Offset::new(250., 100.), Size::new(250., 200.))
    );
    assert_eq!((inner.offset(), outer.offset()), (0., 0.));
    assert!(runtime.dispatch_semantic_action(id, SemanticAction::ShowOnScreen));
    runtime.run_frame(constraints).unwrap();
    assert_eq!((inner.offset(), outer.offset()), (0., 100.));
}
