//! Flutter's `RawAutocomplete` (widgets/autocomplete.dart): options built
//! from the field value, shown while the field has focus, arrow-key
//! highlight, submit selects the highlighted option, Escape hides.

use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::{Code, KeyboardEvent, KeyboardKey, NamedKey, Size};
use incular_text::TextEditingController;
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::{
    AutocompleteHighlightedOption, Directionality, FocusNode, RawAutocomplete, SizedBox,
};

type Seen = Rc<RefCell<Vec<(Vec<String>, usize)>>>;
type SubmitSlot = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

struct Fixture {
    tree: WidgetTree,
    root: ElementId,
    controller: TextEditingController,
    focus: FocusNode,
    seen: Seen,
    selected: Rc<RefCell<Vec<String>>>,
    select: Rc<RefCell<Option<incular_widgets::AutocompleteOnSelected<String>>>>,
    submit: SubmitSlot,
}

fn fixture() -> Fixture {
    let controller = TextEditingController::new();
    let focus = FocusNode::new();
    let seen: Seen = Rc::default();
    let selected = Rc::new(RefCell::new(Vec::new()));
    let select = Rc::new(RefCell::new(None));
    let submit: SubmitSlot = Rc::new(RefCell::new(None));
    let autocomplete = RawAutocomplete::new(
        ["apple", "apricot", "avocado", "banana"].map(String::from),
        |option: &String| option.clone(),
    )
    .text_editing_controller(controller.clone())
    .focus_node(focus.clone())
    .options_builder(|value| {
        ["apple", "apricot", "avocado", "banana"]
            .into_iter()
            .filter(|option| !value.text.is_empty() && option.starts_with(value.text.as_str()))
            .map(String::from)
            .collect()
    })
    .options_view_builder({
        let seen = seen.clone();
        let select = select.clone();
        move |context, on_selected, options| {
            seen.borrow_mut()
                .push((options.to_vec(), AutocompleteHighlightedOption::of(context)));
            *select.borrow_mut() = Some(on_selected);
            SizedBox::new().width(100.).height(40.).into()
        }
    })
    .field_view_builder({
        let submit = submit.clone();
        move |_, _, _, on_submitted| {
            *submit.borrow_mut() = Some(on_submitted);
            SizedBox::new().width(200.).height(30.).into()
        }
    })
    .on_selected({
        let selected = selected.clone();
        move |option| selected.borrow_mut().push(option.clone())
    });
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(Directionality::new(TextDirection::Ltr, autocomplete).into())
        .expect("mount");
    let mut fixture = Fixture {
        tree,
        root,
        controller,
        focus,
        seen,
        selected,
        select,
        submit,
    };
    fixture.pump();
    fixture
}

impl Fixture {
    fn pump(&mut self) {
        self.tree
            .layout(Constraints::loose(Size::new(400., 400.)))
            .expect("layout");
    }

    fn deepest(&self) -> ElementId {
        let mut id = self.root;
        while let Some(&child) = self.tree.children(id).and_then(|children| children.first()) {
            id = child;
        }
        id
    }

    fn key(&mut self, code: Code, named: NamedKey) -> bool {
        let event = KeyboardEvent::key_down(KeyboardKey::Named(named), code);
        let consumed = self.tree.dispatch_keyboard(Some(self.deepest()), event);
        self.pump();
        consumed
    }

    fn last_view(&self) -> Option<(Vec<String>, usize)> {
        self.seen.borrow().last().cloned()
    }
}

#[test]
fn typing_filters_options_shown_only_while_focused() {
    let mut f = fixture();
    f.controller.insert_text("ap");
    f.pump();
    // Not focused: `_canShowOptionsView` is false.
    assert!(f.last_view().is_none());

    f.focus.apply_host_focus(true);
    f.pump();
    let (options, highlighted) = f.last_view().expect("options view built");
    assert_eq!(options, ["apple", "apricot"]);
    assert_eq!(highlighted, 0);

    f.controller.insert_text("r");
    f.pump();
    assert_eq!(f.last_view().expect("view").0, ["apricot"]);
}

#[test]
fn arrows_move_a_clamped_highlight_and_submit_selects_it() {
    let mut f = fixture();
    f.focus.apply_host_focus(true);
    f.controller.insert_text("a");
    f.pump();
    assert_eq!(f.last_view().expect("view").0.len(), 3);

    assert!(f.key(Code::ArrowDown, NamedKey::ArrowDown));
    assert_eq!(f.last_view().expect("view").1, 1);
    assert!(f.key(Code::ArrowDown, NamedKey::ArrowDown));
    assert!(f.key(Code::ArrowDown, NamedKey::ArrowDown));
    // `_updateHighlight` clamps at the last option.
    assert_eq!(f.last_view().expect("view").1, 2);
    assert!(f.key(Code::ArrowUp, NamedKey::ArrowUp));
    assert_eq!(f.last_view().expect("view").1, 1);

    let submit = f.submit.borrow().clone().expect("submit callback");
    submit();
    f.pump();
    assert_eq!(*f.selected.borrow(), ["apricot"]);
    assert_eq!(f.controller.text(), "apricot");
    assert_eq!(
        f.controller.selection_if_set(),
        Some(incular_text::TextSelection::collapsed("apricot".len()))
    );
    // The view closed: arrows now fall through only while options can
    // show, and the next arrow re-opens it (`_highlightOption`).
    let built = f.seen.borrow().len();
    assert!(f.key(Code::ArrowDown, NamedKey::ArrowDown));
    assert!(f.seen.borrow().len() > built);
}

#[test]
fn escape_hides_the_options_and_selecting_from_the_view_sets_the_text() {
    let mut f = fixture();
    f.focus.apply_host_focus(true);
    f.controller.insert_text("b");
    f.pump();
    assert_eq!(f.last_view().expect("view").0, ["banana"]);
    assert!(f.key(Code::Escape, NamedKey::Escape));
    // A hidden view lets Escape continue to enclosing handlers.
    assert!(!f.key(Code::Escape, NamedKey::Escape));

    f.controller.insert_text("a");
    f.pump();
    let select = f.select.borrow().clone().expect("select callback");
    select("banana".to_owned());
    f.pump();
    assert_eq!(f.controller.text(), "banana");
    assert_eq!(*f.selected.borrow(), ["banana"]);
}
