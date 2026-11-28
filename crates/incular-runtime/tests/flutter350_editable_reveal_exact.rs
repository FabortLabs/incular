use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::{Rect, Size};
use incular_platform::{PlatformEvent, TextInputState};
use incular_runtime::Runtime;
use incular_scroll::ScrollPhysics;
use incular_text::{TextEditingController, TextStyle};
use incular_widgets::{EditableText, FocusNode, ListView, ScrollController, SizedBox, Widget};

const KEY: u64 = 902;
const VIEWPORT_HEIGHT: f32 = 300.;

struct Fixture {
    runtime: Runtime,
    scroll: ScrollController,
    focus: FocusNode,
    text: TextEditingController,
}

impl Fixture {
    fn mount(
        leading: f32,
        trailing: f32,
        initial: f32,
        padding: Option<f32>,
        implicit: bool,
    ) -> Self {
        let text = TextEditingController::new();
        let focus = FocusNode::new();
        let scroll = ScrollController::new().with_initial_scroll_offset(initial);
        let mut field = EditableText::new(text.clone())
            .focus_node(focus.clone())
            .style(TextStyle::new().font_size(16.).line_height_multiplier(1.))
            .text_direction(TextDirection::Ltr);
        if let Some(padding) = padding {
            field = field.scroll_padding(EdgeInsets::all(padding));
        }
        let mut rows = vec![
            Widget::from(field).with_key(KEY),
            SizedBox::new().height(trailing).into(),
        ];
        if leading > 0. {
            rows.insert(0, SizedBox::new().height(leading).into());
        }
        let list = ListView::new(rows)
            .controller(scroll.clone())
            .physics(ScrollPhysics::CLAMPING.with_implicit_scrolling(implicit));
        let runtime = Runtime::new(list.into()).unwrap();
        let mut fixture = Self {
            runtime,
            scroll,
            focus,
            text,
        };
        fixture.layout();
        fixture
    }

    fn layout(&mut self) {
        for _ in 0..4 {
            self.runtime
                .run_frame(Constraints::tight(Size::new(800., VIEWPORT_HEIGHT)))
                .unwrap();
        }
    }

    fn editor(&self) -> incular_widgets::internal::ElementId {
        self.runtime.tree().element_with_key(&KEY.into()).unwrap()
    }

    fn bounds(&self) -> Rect {
        self.runtime.tree().element_bounds(self.editor()).unwrap()
    }

    fn open_keyboard(&mut self) {
        assert!(self.runtime.request_text_input_focus_for(self.editor()));
        self.layout();
        assert!(self.focus.has_primary_focus());
    }

    fn scroll_to(&mut self, offset: f32) {
        let _ = self.scroll.jump_to(offset);
        self.layout();
        assert_eq!(self.scroll.offset(), offset);
    }

    fn input(&mut self, text: &str) {
        self.runtime
            .handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
                text: text.into(),
                selection_start: text.len(),
                selection_end: text.len(),
                composing: None,
            }));
        self.layout();
        assert_eq!(self.text.text(), text);
    }
}

#[test]
fn opening_the_keyboard_reveals_the_editor_from_half_above_the_list() {
    let mut fixture = Fixture::mount(0., 350., 0., None, true);
    let half = 0.5 * fixture.bounds().size.height;
    fixture.scroll_to(half);
    fixture.open_keyboard();
    assert_eq!(fixture.scroll.offset(), 0.);
}

#[test]
fn opening_the_keyboard_honors_the_custom_scroll_inset() {
    let mut fixture = Fixture::mount(200., 850., 0., Some(50.), true);
    let half = 0.5 * fixture.bounds().size.height;
    fixture.scroll_to(200. + half);
    fixture.open_keyboard();
    assert!((fixture.scroll.offset() - 150.).abs() < 5.);
}

fn typing_when_editor_was_scrolled_away(implicit: bool) {
    let mut fixture = Fixture::mount(350., 350., 100., None, implicit);
    fixture.open_keyboard();
    let retained = fixture.editor();
    fixture.scroll_to(0.);
    assert!(fixture.bounds().origin.y >= VIEWPORT_HEIGHT);
    assert!(fixture.focus.has_primary_focus());
    fixture.input("Hello");
    assert_eq!(fixture.editor(), retained);
    if implicit {
        assert!(fixture.scroll.offset() > 0.);
        assert!(fixture.bounds().origin.y < VIEWPORT_HEIGHT);
    } else {
        assert_eq!(fixture.scroll.offset(), 0.);
        assert!(fixture.bounds().origin.y >= VIEWPORT_HEIGHT);
    }
}

#[test]
fn entering_text_reveals_the_focused_editor_after_user_scrolled_away() {
    typing_when_editor_was_scrolled_away(true);
}

#[test]
fn implicit_scroll_policy_can_leave_the_edited_field_off_screen() {
    typing_when_editor_was_scrolled_away(false);
}

#[test]
fn bringing_an_unfocused_multiline_caret_into_view_scrolls_only_the_outer_viewport() {
    use incular_config::Alignment;
    use incular_core::Offset;
    use incular_text::TextAffinity;
    use incular_widgets::{Align, GlobalKey, SingleChildScrollView};

    let body =
        "Now is the time for\nall good people\nto come to the aid\nof their country.".repeat(20);
    let text = TextEditingController::with_text(&body);
    text.clear_selection();
    let focus = FocusNode::new();
    let key = GlobalKey::new();
    let editor = Widget::from(
        EditableText::new(text.clone())
            .focus_node(focus.clone())
            .text_direction(TextDirection::Ltr)
            .max_lines(None),
    )
    .with_key(key);
    let scrolling = ScrollController::new();
    let viewport = SingleChildScrollView::new(editor).controller(scrolling.clone());
    let scene = Align::new(Alignment::TOP_LEFT, SizedBox::square(200.).child(viewport));
    let mut runtime = Runtime::new(scene.into()).unwrap();
    let bounds = Constraints::tight(Size::new(800., 600.));
    runtime.run_frame(bounds).unwrap();
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    let renderer = runtime.tree().render_id(id);
    assert!(scrolling.max_offset() > 0.);
    assert_eq!(scrolling.offset(), 0.);
    assert_eq!(
        runtime.tree().text_field_scroll_offset(id),
        Some(Offset::ZERO)
    );
    assert!(!focus.has_focus());
    assert!(text.selection_if_set().is_none());

    assert!(runtime.tree_mut().text_field_bring_position_into_view(
        id,
        body.len(),
        TextAffinity::Downstream,
    ));
    for _ in 0..3 {
        runtime.run_frame(bounds).unwrap();
    }
    assert_eq!(scrolling.offset(), scrolling.max_offset());
    assert_eq!(
        runtime.tree().text_field_scroll_offset(id),
        Some(Offset::ZERO)
    );
    assert_eq!(runtime.tree().render_id(id), renderer);
    assert!(!focus.has_focus());
    assert!(text.selection_if_set().is_none());
    let caret_y = runtime
        .tree()
        .text_field_offset_rect(id, body.len(), TextAffinity::Downstream)
        .unwrap()
        .origin
        .y;
    assert!((0. ..200.).contains(&caret_y));
}

#[test]
fn retained_editor_can_disable_implicit_caret_scrolling_without_disabling_explicit_scroll() {
    use incular_config::Alignment;
    use incular_core::Offset;
    use incular_text::TextAffinity;
    use incular_widgets::{Align, GlobalKey};

    let body =
        "Now is the time for\nall good people\nto come to the aid\nof their country.".repeat(20);
    let text = TextEditingController::with_text(&body);
    text.clear_selection();
    let key = GlobalKey::new();
    let build = |allow: bool| {
        let mut editor = EditableText::new(text.clone())
            .text_direction(TextDirection::Ltr)
            .max_lines(None);
        if !allow {
            editor = editor.allow_implicit_caret_scrolling(false);
        }
        Widget::from(Align::new(
            Alignment::TOP_LEFT,
            SizedBox::square(200.).child(Widget::from(editor).with_key(key)),
        ))
    };
    let bounds = Constraints::tight(Size::new(800., 600.));
    let mut runtime = Runtime::new(build(true)).unwrap();
    runtime.run_frame(bounds).unwrap();
    let root = runtime.tree().root().unwrap();
    let id = runtime.tree().element_with_key(&key.into()).unwrap();
    let render = runtime.tree().render_id(id);
    assert_eq!(
        runtime.tree().text_field_scroll_offset(id),
        Some(Offset::ZERO)
    );
    let max = runtime.tree().editable_max_scroll_extent(id).unwrap();
    assert!(max > 0.);

    assert!(runtime.tree_mut().text_field_bring_position_into_view(
        id,
        body.len(),
        TextAffinity::Downstream
    ));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(runtime.tree().text_field_scroll_offset(id).unwrap().y, max);

    assert!(
        runtime
            .tree_mut()
            .text_field_scroll_to_document_boundary(id, false)
    );
    runtime.schedule_update(root, build(false)).unwrap();
    runtime.run_frame(bounds).unwrap();
    assert_eq!(runtime.tree().render_id(id), render);
    assert_eq!(
        runtime.tree().text_field_scroll_offset(id),
        Some(Offset::ZERO)
    );
    assert!(!runtime.tree_mut().text_field_bring_position_into_view(
        id,
        body.len(),
        TextAffinity::Downstream
    ));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(
        runtime.tree().text_field_scroll_offset(id),
        Some(Offset::ZERO)
    );

    text.set_selection(incular_text::TextSelection::collapsed(body.len()));
    runtime.run_frame(bounds).unwrap();
    assert_eq!(
        runtime.tree().text_field_scroll_offset(id),
        Some(Offset::ZERO)
    );
    assert!(
        runtime
            .tree_mut()
            .text_field_scroll_to_document_boundary(id, true)
    );
    runtime.run_frame(bounds).unwrap();
    assert_eq!(runtime.tree().text_field_scroll_offset(id).unwrap().y, max);
}
