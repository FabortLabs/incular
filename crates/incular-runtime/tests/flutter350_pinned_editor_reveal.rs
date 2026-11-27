use std::time::Duration;

use incular_config::{Alignment, Constraints, EdgeInsets, TextDirection};
use incular_core::{Size, time::Instant};
use incular_runtime::Runtime;
use incular_text::{TextAffinity, TextEditingController, TextStyle};
use incular_widgets::{
    Align, CustomScrollView, Directionality, EditableText, FocusNode, ListView,
    PersistentHeaderShowOnScreenConfiguration, ScrollController, Scrollable, ShowOnScreen,
    SizedBox, Sliver, SliverResizingHeader, SliverToBoxAdapter, Widget,
};

const EDITOR: u64 = 910;
const LEADING: u64 = 911;

fn editor(focus: &FocusNode) -> EditableText {
    EditableText::new(TextEditingController::new())
        .focus_node(focus.clone())
        .style(TextStyle::new().font_size(14.).line_height_multiplier(1.))
        .text_direction(TextDirection::Ltr)
}

#[test]
fn keyboard_reveal_honors_a_bottom_inset_as_tall_as_the_viewport() {
    let focus = FocusNode::new();
    let scroll = ScrollController::new();
    let leading = Widget::from(SizedBox::new().height(200.)).with_key(LEADING);
    let field = Widget::from(editor(&focus).scroll_padding(EdgeInsets::only(0., 0., 0., 300.)))
        .with_key(EDITOR);
    let list = ListView::new([leading, field, SizedBox::new().height(400.).into()])
        .controller(scroll.clone());
    let mut runtime = Runtime::new(list.into()).unwrap();
    let viewport = Constraints::tight(Size::new(800., 300.));
    runtime.run_frame(viewport).unwrap();
    assert_eq!(scroll.offset(), 0.);
    let field = runtime.tree().element_with_key(&EDITOR.into()).unwrap();
    assert!(runtime.request_text_input_focus_for(field));
    for _ in 0..3 {
        runtime.run_frame(viewport).unwrap();
    }
    assert!(focus.has_primary_focus());
    assert!(scroll.offset() > 0.);
    if let Some(bounds) = runtime
        .tree()
        .element_with_key(&LEADING.into())
        .and_then(|element| runtime.tree().element_bounds(element))
    {
        assert!(bounds.origin.y + bounds.size.height <= 0.);
    }
}

fn pinned_header(animated: bool) {
    let focus = FocusNode::new();
    let scroll = ScrollController::new();
    let field = Widget::from(editor(&focus)).with_key(EDITOR);
    let slivers = (0..50)
        .map(|index| -> Box<dyn Sliver> {
            if index == 10 {
                Box::new(
                    SliverResizingHeader::new(
                        50.,
                        50.,
                        Align::new(Alignment::TOP_CENTER, field.clone()),
                    )
                    .show_on_screen_configuration(
                        PersistentHeaderShowOnScreenConfiguration::default(),
                    ),
                )
            } else {
                Box::new(SliverToBoxAdapter::new(SizedBox::new().height(100.)))
            }
        })
        .collect::<Vec<_>>();
    let list = CustomScrollView::new(slivers).controller(scroll.clone());
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, list).into()).unwrap();
    let viewport = Constraints::tight(Size::new(600., 600.));
    let zero = Instant::now();
    runtime.run_frame_at(viewport, zero).unwrap();
    assert!(scroll.jump_to(1500.));
    runtime.run_frame_at(viewport, zero).unwrap();
    assert_eq!(scroll.offset(), 1500.);

    focus.request_focus();
    let (_, first) = runtime.run_frame_at(viewport, zero).unwrap();
    assert!(focus.has_primary_focus());
    assert_eq!(scroll.offset(), 1500.);
    assert_eq!(first.active_animations, 0);
    let field = runtime.focused_element().unwrap();
    let caret = runtime
        .tree()
        .editable_local_caret_rect(field, 0, TextAffinity::Downstream)
        .unwrap();
    let rect = EdgeInsets::all(20.).inflate_rect(caret).unwrap();
    let duration = if animated {
        Duration::from_millis(100)
    } else {
        Duration::ZERO
    };
    let request = ShowOnScreen::new().rect(rect).duration(duration);
    assert_eq!(
        Scrollable::show_on_screen(runtime.tree(), field, request),
        0
    );
    for elapsed in [50, 100, 150] {
        let (_, stats) = runtime
            .run_frame_at(viewport, zero + Duration::from_millis(elapsed))
            .unwrap();
        assert_eq!(scroll.offset(), 1500.);
        assert_eq!(stats.active_animations, 0);
    }
}

#[test]
fn focusing_a_pinned_headers_editor_does_not_start_a_viewport_animation() {
    pinned_header(true);
}

#[test]
fn focusing_a_pinned_headers_editor_does_not_jump_the_viewport() {
    pinned_header(false);
}
