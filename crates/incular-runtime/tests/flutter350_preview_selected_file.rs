use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

use incular_config::{Constraints, TextDirection};
use incular_controls::widget_previews::preview_file_filter_toggle;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Column, Directionality, Text, Widget,
    widget_previews::file_filter::{PreviewFileEntry, PreviewFileFilter, PreviewPathStyle},
};

const TOGGLE: &str = "preview.filter.selected-file";
const PREF: &str = PreviewFileFilter::<&str>::PREFERENCE_KEY;

struct Fixture {
    runtime: Runtime,
    filter: PreviewFileFilter<&'static str>,
    prefs: Rc<RefCell<BTreeMap<String, bool>>>,
    fail: Rc<Cell<bool>>,
    errors: Rc<Cell<usize>>,
}

impl Fixture {
    fn new(style: PreviewPathStyle, available: bool, files: [&str; 2]) -> Self {
        let filter = PreviewFileFilter::new(
            style,
            available,
            None,
            files
                .into_iter()
                .zip(["widget1", "widget2"])
                .map(|(file, value)| PreviewFileEntry {
                    source_uri: file.to_owned(),
                    group: "group".to_owned(),
                    value,
                }),
        );
        let prefs = Rc::new(RefCell::new(BTreeMap::new()));
        let fail = Rc::new(Cell::new(false));
        let errors = Rc::new(Cell::new(0));
        let toggle = preview_file_filter_toggle(
            filter.clone(),
            {
                let prefs = prefs.clone();
                let fail = fail.clone();
                move |key, enabled| {
                    if fail.get() {
                        return Err(());
                    }
                    prefs.borrow_mut().insert(key.to_owned(), enabled);
                    Ok(())
                }
            },
            {
                let errors = errors.clone();
                move |_| errors.set(errors.get() + 1)
            },
        );
        let panel = Builder::new({
            let filter = filter.clone();
            move |_| {
                let groups = filter.visible_groups();
                Column::new(groups.iter().flat_map(|group| {
                    group
                        .previews
                        .iter()
                        .map(|entry| Widget::from(Text::new(entry.value)))
                }))
                .into()
            }
        });
        let mut runtime = Runtime::new(
            Directionality::new(TextDirection::Ltr, Column::new([toggle, panel.into()])).into(),
        )
        .unwrap();
        runtime.set_platform_semantics_enabled(true);
        let mut scene = Self {
            runtime,
            filter,
            prefs,
            fail,
            errors,
        };
        scene.frame();
        scene
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame(Constraints::tight(Size::new(480., 240.)))
            .unwrap();
    }

    fn has_toggle(&self) -> bool {
        self.runtime
            .tree()
            .element_with_key(&TOGGLE.into())
            .is_some()
    }

    fn toggle(&mut self) {
        let tree = self.runtime.tree();
        let id = tree.element_with_key(&TOGGLE.into()).unwrap();
        let bounds = tree.element_bounds(id).unwrap();
        let position = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = self
                .runtime
                .handle_input(InputEvent::Pointer { position, phase });
        }
        self.frame();
    }

    fn shows(&self, expected: &[&str]) {
        let groups = self.filter.visible_groups();
        for group in groups.iter() {
            assert_eq!(group.name, "group");
        }
        assert_eq!(
            groups
                .iter()
                .flat_map(|group| group.previews.iter().map(|entry| entry.value))
                .collect::<Vec<_>>(),
            expected
        );
        let mut rendered = self
            .runtime
            .tree()
            .semantics()
            .iter()
            .filter_map(|(_, node)| {
                node.label
                    .as_deref()
                    .filter(|label| label.starts_with("widget"))
                    .map(|label| (node.bounds.origin.y, label))
            })
            .collect::<Vec<_>>();
        rendered.sort_by(|(y, _), (other, _)| y.total_cmp(other));
        assert_eq!(
            rendered.iter().map(|(_, label)| *label).collect::<Vec<_>>(),
            expected
        );
    }
}

fn selected_file(style: PreviewPathStyle, sources: [&str; 2], editor: [&str; 2]) {
    let mut scene = Fixture::new(style, true, sources);
    assert!(scene.filter.enabled());
    scene.shows(&[]);
    scene.filter.set_selected_source(Some(editor[0]));
    scene.frame();
    scene.shows(&["widget1"]);
    scene.filter.set_selected_source(None);
    scene.frame();
    scene.shows(&["widget1"]);
    scene.filter.set_selected_source(Some(editor[1]));
    scene.frame();
    scene.shows(&["widget2"]);
    scene.toggle();
    assert!(!scene.filter.enabled());
    scene.shows(&["widget1", "widget2"]);
    scene.toggle();
    scene.shows(&["widget2"]);
}

#[test]
fn posix_uri_selection_and_focus_back_to_the_previewer_keep_the_active_group() {
    let files = ["file:///script1", "file:///script2"];
    selected_file(PreviewPathStyle::Posix, files, files);
}

#[test]
fn windows_drive_case_and_percent_encoded_colons_do_not_hide_the_previews() {
    selected_file(
        PreviewPathStyle::Windows,
        ["file:///c:/script1", "file:///C:/script2"],
        ["file:///C%3A/script1", "file:///c%3A/script2"],
    );
}

#[test]
fn the_toggle_and_visible_groups_follow_live_editor_service_availability() {
    let mut scene = Fixture::new(
        PreviewPathStyle::Posix,
        false,
        ["file:///script1", "file:///script2"],
    );
    assert!(!scene.has_toggle());
    scene.shows(&["widget1", "widget2"]);
    scene.filter.set_editor_available(true);
    scene.frame();
    assert!(scene.has_toggle());
    assert!(scene.filter.enabled());
    scene.shows(&[]);
    scene.filter.set_editor_available(false);
    scene.frame();
    assert!(!scene.has_toggle());
    scene.shows(&["widget1", "widget2"]);
}

#[test]
fn defaults_dont_write_preferences_and_failed_storage_doesnt_flip_the_toggle() {
    let mut scene = Fixture::new(
        PreviewPathStyle::Posix,
        true,
        ["file:///script1", "file:///script2"],
    );
    assert!(scene.filter.enabled());
    assert_eq!(scene.prefs.borrow().get(PREF), None);
    scene.toggle();
    assert!(!scene.filter.enabled());
    assert_eq!(scene.prefs.borrow().get(PREF), Some(&false));
    scene.fail.set(true);
    scene.toggle();
    assert_eq!(scene.errors.get(), 1);
    assert!(!scene.filter.enabled());
    assert_eq!(scene.prefs.borrow().get(PREF), Some(&false));
    scene.fail.set(false);
    scene.toggle();
    assert!(scene.filter.enabled());
    assert_eq!(scene.prefs.borrow().get(PREF), Some(&true));
}
