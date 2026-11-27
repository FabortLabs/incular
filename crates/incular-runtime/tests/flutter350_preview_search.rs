use incular_config::{Constraints, TextDirection};
use incular_controls::widget_previews::PreviewSearchControls;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Column, Directionality, FocusNode, Text, TextSelection, Widget,
    widget_previews::search::{PreviewSearch, PreviewSearchField as Field},
};

const EDITOR: &str = "preview.search.editor";
const CLEAR: &str = "preview.search.clear";

struct Fixture {
    runtime: Runtime,
    search: PreviewSearch,
    focus: FocusNode,
}

impl Fixture {
    fn new() -> Self {
        let search = PreviewSearch::default();
        let focus = FocusNode::new();
        let toolbar = PreviewSearchControls::new(search.clone()).focus_node(focus.clone());
        let panel = Builder::new({
            let search = search.clone();
            move |_| {
                let matcher = search.matcher();
                Column::new(
                    ["My Awesome Widget", "Other Widget"]
                        .into_iter()
                        .filter_map(|name| {
                            matcher
                                .matches("group", Some(name), "file:///preview.rs", "sample")
                                .then(|| Widget::from(Text::new(name)))
                        }),
                )
                .into()
            }
        });
        let mut runtime = Runtime::new(
            Directionality::new(
                TextDirection::Ltr,
                Column::new([Widget::from(toolbar), panel.into()]),
            )
            .into(),
        )
        .unwrap();
        runtime.set_platform_semantics_enabled(true);
        let mut this = Self {
            runtime,
            search,
            focus,
        };
        this.frame();
        this
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame(Constraints::tight(Size::new(400., 140.)))
            .unwrap();
    }

    fn has_key(&self, key: &str) -> bool {
        self.runtime.tree().element_with_key(&key.into()).is_some()
    }

    fn tap(&mut self, key: &str) {
        let tree = self.runtime.tree();
        let id = tree.element_with_key(&key.into()).unwrap();
        let bounds = tree.element_bounds(id).unwrap();
        let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = self
                .runtime
                .handle_input(InputEvent::Pointer { position, phase });
        }
    }

    fn shows(&self, label: &str) -> bool {
        self.runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.label.as_deref() == Some(label))
    }
}

#[test]
fn typing_and_clearing_use_one_query_and_restore_results_without_losing_focus() {
    let mut scene = Fixture::new();
    assert!(scene.has_key(EDITOR));
    assert!(!scene.has_key(CLEAR));
    assert!(scene.shows("My Awesome Widget") && scene.shows("Other Widget"));
    scene.tap(EDITOR);
    assert!(scene.focus.has_focus());
    let _ = scene
        .runtime
        .handle_input(InputEvent::Text("My Awesome Widget".to_owned()));
    scene.frame();
    assert_eq!(scene.search.query(), "My Awesome Widget");
    assert!(scene.has_key(CLEAR));
    assert!(
        scene
            .runtime
            .tree()
            .semantics()
            .iter()
            .any(|(_, node)| node.state.tooltip.as_deref() == Some("Clear search"))
    );
    assert!(scene.shows("My Awesome Widget"));
    assert!(!scene.shows("Other Widget"));
    scene.tap(CLEAR);
    scene.frame();
    assert_eq!(scene.search.query(), "");
    assert!(!scene.has_key(CLEAR));
    assert!(scene.focus.has_focus());
    assert!(scene.shows("My Awesome Widget") && scene.shows("Other Widget"));
    let _ = scene
        .runtime
        .handle_input(InputEvent::Text("other".to_owned()));
    scene.frame();
    assert_eq!(scene.search.query(), "other");
    assert!(scene.shows("Other Widget") && !scene.shows("My Awesome Widget"));
}

#[test]
fn a_programmatic_query_updates_the_buffer_and_moves_the_selection_to_its_end() {
    let mut scene = Fixture::new();
    scene.tap(EDITOR);
    scene.search.set_query("  awesome  ");
    scene.frame();
    let editor = scene.search.editing_controller();
    assert_eq!(editor.text(), "  awesome  ");
    assert_eq!(editor.selection(), TextSelection::collapsed(11));
    assert!(scene.shows("My Awesome Widget") && !scene.shows("Other Widget"));
    editor.set_selection(TextSelection::collapsed(2));
    scene.search.set_query("  awesome  ");
    assert_eq!(editor.selection(), TextSelection::collapsed(2));
}

#[test]
fn search_checks_only_active_fields_and_never_disables_the_last_field() {
    let search = PreviewSearch::default();
    search.set_query("  PACKAGE  ");
    assert!(
        search
            .matcher()
            .matches("group", None, "file:///demo.rs", "demo_package")
    );
    assert!(search.toggle_field(Field::Package));
    assert!(
        !search
            .matcher()
            .matches("group", None, "file:///demo.rs", "demo_package")
    );
    assert!(search.toggle_field(Field::Group));
    assert!(search.toggle_field(Field::Script));
    assert!(!search.toggle_field(Field::Name));
    assert!(search.field_enabled(Field::Name));
    assert!(
        search
            .matcher()
            .matches("group", Some("Package view"), "", "")
    );
}
