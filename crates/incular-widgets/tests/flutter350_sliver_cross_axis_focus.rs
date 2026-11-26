//! A real editor must remain mounted when its lane scrolls offscreen.
use std::time::Instant;

use incular_config::{Constraints, TextDirection};
use incular_core::{Offset, Size};
use incular_scroll::ScrollController;
use incular_text::TextEditingController;
use incular_widgets::{
    CustomScrollView, Directionality, EditableText, GlobalKey, SizedBox, Sliver,
    SliverConstrainedCrossAxis, SliverCrossAxisGroup, SliverFixedExtentList, internal::WidgetTree,
};

#[test]
fn scrolled_out_grouped_editor_is_kept_alive_only_while_focused() {
    for constrain in [false, true] {
        let key = GlobalKey::new();
        let edit = TextEditingController::with_text("draft");
        let edit_for_child = edit.clone();
        let scroll = ScrollController::new();
        let first = SliverFixedExtentList::new(60, 40., move |index| {
            if index == 0 {
                incular_widgets::Widget::from(EditableText::new(edit_for_child.clone()))
                    .with_key(key)
            } else {
                SizedBox::new().height(40.).into()
            }
        });
        let lane: Box<dyn Sliver> = if constrain {
            Box::new(SliverConstrainedCrossAxis::new(100., first))
        } else {
            Box::new(first)
        };
        let second = SliverFixedExtentList::new(60, 40., |_| SizedBox::new().height(40.));
        let group = SliverCrossAxisGroup::new(vec![lane, Box::new(second) as Box<dyn Sliver>]);
        let mut tree = WidgetTree::new();
        tree.mount(
            Directionality::new(
                TextDirection::Ltr,
                CustomScrollView::new(vec![Box::new(group) as Box<dyn Sliver>])
                    .controller(scroll.clone()),
            )
            .into(),
        )
        .unwrap();
        let layout = |tree: &mut WidgetTree| {
            tree.layout(Constraints::tight(Size::new(300., 300.)))
                .unwrap();
            tree.update_compositor(Instant::now()).unwrap();
        };
        layout(&mut tree);
        let editor = tree.element_with_key(&key.into()).unwrap();
        assert_eq!(tree.text_field_at(Offset::new(20., 20.)), Some(editor));
        let client = tree.text_field_client_id(editor).unwrap();
        tree.set_focused(editor, true, Instant::now()).unwrap();
        assert!(scroll.jump_to(1200.));
        layout(&mut tree);
        assert_eq!(scroll.offset(), 1200.);
        assert_eq!(
            tree.element_with_key(&key.into()),
            Some(editor),
            "constrained wrapper={constrain}"
        );
        assert_eq!(tree.text_field_client_id(editor), Some(client));
        assert_eq!(tree.text_field_at(Offset::new(20., 20.)), None);

        tree.set_focused(editor, false, Instant::now()).unwrap();
        layout(&mut tree);
        assert!(tree.element_with_key(&key.into()).is_none());
        assert!(scroll.jump_to(0.));
        layout(&mut tree);
        let remounted = tree.element_with_key(&key.into()).unwrap();
        assert_ne!(tree.text_field_client_id(remounted), Some(client));
        assert_eq!(edit.value().text, "draft");
    }
}
