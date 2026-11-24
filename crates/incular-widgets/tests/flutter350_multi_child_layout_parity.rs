//! Flutter 3.50.0 CustomMultiChildLayout delegate and child contracts.
use incular_config::Constraints;
use incular_core::Size;
use incular_widgets::{
    Center, CustomMultiChildLayout, LayoutId, MultiChildLayoutContext, MultiChildLayoutDelegate,
    SizedBox, Widget,
    internal::{Key, WidgetTree},
};
use std::{cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

const PARENT: Size = Size {
    width: 200.,
    height: 300.,
};
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

#[derive(Clone, Copy)]
enum Mode {
    Regular,
    Duplicate,
}
#[derive(Default)]
struct Observations {
    constraints: Vec<Constraints>,
    laid_out: Vec<(u32, Size, Vec<Size>, bool)>,
    asked: Vec<(u32, u32)>,
}
struct Delegate {
    id: u32,
    relayout: bool,
    mode: Mode,
    log: Rc<RefCell<Observations>>,
}
impl MultiChildLayoutDelegate for Delegate {
    fn get_size(&self, constraints: Constraints) -> Size {
        self.log.borrow_mut().constraints.push(constraints);
        PARENT
    }
    fn perform_layout(&self, size: Size, ctx: &mut MultiChildLayoutContext<'_>) {
        let bounds = Constraints::loose(size);
        let a = ctx.layout_child(0usize, bounds);
        let b = match self.mode {
            Mode::Regular => ctx.layout_child(1usize, bounds),
            Mode::Duplicate => ctx.layout_child(0usize, Constraints::tight(Size::new(100., 90.))),
        };
        self.log
            .borrow_mut()
            .laid_out
            .push((self.id, size, vec![a, b], ctx.has_child("fred")));
    }
    fn should_relayout(&self, old: &Self) -> bool {
        self.log.borrow_mut().asked.push((self.id, old.id));
        self.relayout
    }
}

fn scene(
    log: &Rc<RefCell<Observations>>,
    id: u32,
    relayout: bool,
    count: usize,
    mode: Mode,
) -> Widget {
    let children = (0..count).map(|i| {
        let (width, height) = if i == 0 { (150., 100.) } else { (100., 200.) };
        Widget::from(LayoutId::new(
            i,
            SizedBox::new().width(width).height(height),
        ))
        .with_key(Key::Value(i as u64))
    });
    Center::new(CustomMultiChildLayout::new(
        Delegate {
            id,
            relayout,
            mode,
            log: log.clone(),
        },
        children,
    ))
    .into()
}

#[test]
fn delegate_and_child_constraints_match_exact_control() {
    let log = Rc::new(RefCell::new(Observations::default()));
    let mut tree = WidgetTree::new();
    tree.mount(scene(&log, 0, false, 2, Mode::Regular)).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let view = log.borrow();
    assert_eq!(view.constraints.last(), Some(&Constraints::loose(VIEW)));
    assert_eq!(
        view.laid_out.last(),
        Some(&(
            0,
            PARENT,
            vec![Size::new(150., 100.), Size::new(100., 200.)],
            false
        ))
    );
}

#[test]
fn the_replacement_delegate_decides_if_layout_runs() {
    let log = Rc::new(RefCell::new(Observations::default()));
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(&log, 1, false, 2, Mode::Regular)).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert!(log.borrow().asked.is_empty());
    assert!(log.borrow().laid_out.iter().any(|(id, ..)| *id == 1));
    for (id, needs_layout) in [(2, false), (3, true)] {
        tree.update(root, scene(&log, id, needs_layout, 2, Mode::Regular))
            .unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
    }
    let facts = log.borrow();
    let observed = [(2, 1), (3, 2)].map(|(new, old)| {
        (
            facts.asked.contains(&(new, old)),
            facts.laid_out.iter().any(|(id, ..)| *id == new),
        )
    });
    assert_eq!(
        observed,
        [(true, false), (true, true)],
        "all receivers: {:?}",
        facts.asked
    );
}

fn invalid_layout(count: usize, mode: Mode) {
    let log = Rc::new(RefCell::new(Observations::default()));
    let mut tree = WidgetTree::new();
    tree.mount(scene(&log, 1, false, count, mode)).unwrap();
    let result =
        std::panic::catch_unwind(AssertUnwindSafe(|| tree.layout(Constraints::tight(VIEW))));
    if matches!(result, Ok(Ok(_))) {
        let sizes: Vec<_> = (0..count)
            .map(|i| {
                let element = tree.element_with_key(&Key::Value(i as u64)).unwrap();
                tree.element_bounds(element).unwrap().size
            })
            .collect();
        panic!(
            "invalid layout must be rejected; accepted child sizes={sizes:?}, delegate observations={:?}",
            log.borrow().laid_out
        );
    }
}

#[test]
fn child_cannot_be_laid_out_twice() {
    invalid_layout(1, Mode::Duplicate);
}
#[test]
fn forgetting_one_child_rejects_layout() {
    invalid_layout(3, Mode::Regular);
}
#[test]
fn forgetting_two_children_rejects_layout() {
    invalid_layout(4, Mode::Regular);
}

struct Pref(Rc<std::cell::Cell<Size>>);
impl MultiChildLayoutDelegate for Pref {
    fn get_size(&self, _: Constraints) -> Size {
        self.0.get()
    }
    fn perform_layout(&self, _: Size, _: &mut MultiChildLayoutContext<'_>) {}
    fn should_relayout(&self, old: &Self) -> bool {
        self.0.get() != old.0.get()
    }
}
fn preferred(size: Rc<std::cell::Cell<Size>>) -> Widget {
    Center::new(
        Widget::from(CustomMultiChildLayout::new(
            Pref(size),
            Vec::<Widget>::new(),
        ))
        .with_key(Key::Value(50)),
    )
    .into()
}
fn preferred_size(tree: &WidgetTree) -> Size {
    let id = tree.element_with_key(&Key::Value(50)).unwrap();
    tree.element_bounds(id).unwrap().size
}

#[test]
fn preferred_size_can_change_under_loose_constraints() {
    use std::cell::Cell;
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(preferred(Rc::new(Cell::new(Size::new(300., 200.)))))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(preferred_size(&tree), Size::new(300., 200.));
    tree.update(root, preferred(Rc::new(Cell::new(Size::new(350., 250.)))))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(preferred_size(&tree), Size::new(350., 250.));
}

#[test]
fn existing_delegate_can_trigger_relayout_without_replacement() {
    use std::cell::Cell;
    let mut tree = WidgetTree::new();
    let state = Rc::new(Cell::new(Size::new(100., 200.)));
    let root = tree.mount(preferred(state.clone())).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(preferred_size(&tree), Size::new(100., 200.));
    state.set(Size::new(150., 240.));
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let observed = preferred_size(&tree);
    // Preserve the state-before-change observation for the replacement's
    // delegate diff, then install a genuinely new preferred size.
    state.set(Size::new(100., 200.));
    tree.update(root, preferred(Rc::new(Cell::new(Size::new(150., 240.)))))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(
        preferred_size(&tree),
        Size::new(150., 240.),
        "new delegate control"
    );
    assert_eq!(
        observed,
        Size::new(150., 240.),
        "same retained delegate relayout"
    );
}

#[test]
fn nested_multichild_layout_has_independent_slots_with_same_ids() {
    let log = Rc::new(RefCell::new(Observations::default()));
    let layout = |id| Delegate {
        id,
        relayout: false,
        mode: Mode::Regular,
        log: log.clone(),
    };
    let inner = CustomMultiChildLayout::new(
        layout(1),
        [
            LayoutId::new(0usize, SizedBox::new().width(150.).height(100.)),
            LayoutId::new(1usize, SizedBox::new().width(100.).height(200.)),
        ],
    );
    let outer = Center::new(CustomMultiChildLayout::new(
        layout(2),
        [
            LayoutId::new(0usize, inner),
            LayoutId::new(1usize, SizedBox::new().width(100.).height(200.)),
        ],
    ));
    let mut tree = WidgetTree::new();
    tree.mount(outer.into()).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let log = log.borrow();
    assert!(log.laid_out.iter().any(|(id, ..)| *id == 1));
    assert_eq!(
        log.laid_out.last(),
        Some(&(2, PARENT, vec![PARENT, Size::new(100., 200.)], false))
    );
}

#[test]
fn zero_size_parent_clamps_even_nonzero_preferred_geometry() {
    let mut tree = WidgetTree::new();
    tree.mount(preferred(Rc::new(std::cell::Cell::new(Size::new(
        100., 200.,
    )))))
    .unwrap();
    tree.layout(Constraints::tight(Size::ZERO)).unwrap();
    let _ = tree.paint();
    assert_eq!(preferred_size(&tree), Size::ZERO);
}

#[derive(Clone, Copy)]
enum IdCheck {
    Layout,
    Position,
    BadConstraints,
}
struct IdProbe(IdCheck);
impl MultiChildLayoutDelegate for IdProbe {
    fn get_size(&self, _: Constraints) -> Size {
        PARENT
    }
    fn should_relayout(&self, _: &Self) -> bool {
        true
    }
    fn perform_layout(&self, size: Size, ctx: &mut MultiChildLayoutContext<'_>) {
        match self.0 {
            IdCheck::Layout => {
                ctx.layout_child(0usize, Constraints::loose(size));
                ctx.layout_child(1usize, Constraints::loose(size));
            }
            IdCheck::Position => {
                ctx.position_child(0usize, incular_core::Offset::ZERO);
                ctx.position_child(1usize, incular_core::Offset::ZERO);
            }
            IdCheck::BadConstraints => {
                ctx.layout_child(0usize, Constraints::new(0., -1., 0., -1.));
            }
        }
    }
}

fn id_scene(tree: &mut WidgetTree, case: IdCheck) {
    let child = Widget::from(LayoutId::new(
        0usize,
        SizedBox::new().width(100.).height(100.),
    ))
    .with_key(Key::Value(77));
    tree.mount(Center::new(CustomMultiChildLayout::new(IdProbe(case), [child])).into())
        .unwrap();
}

fn missing_id(case: IdCheck) {
    let mut tree = WidgetTree::new();
    id_scene(&mut tree, case);
    let panic =
        std::panic::catch_unwind(AssertUnwindSafe(|| tree.layout(Constraints::tight(VIEW))))
            .expect_err("nonexistent id must be rejected");
    let error = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(
        error.contains("MissingLayoutChild") && error.contains("1"),
        "{error}"
    );
}

#[test]
fn cannot_layout_a_missing_id() {
    missing_id(IdCheck::Layout);
}
#[test]
fn cannot_position_a_missing_id() {
    missing_id(IdCheck::Position);
}

#[test]
fn negative_child_maximum_is_not_silently_accepted() {
    let mut tree = WidgetTree::new();
    id_scene(&mut tree, IdCheck::BadConstraints);
    let result =
        std::panic::catch_unwind(AssertUnwindSafe(|| tree.layout(Constraints::tight(VIEW))));
    if matches!(result, Ok(Ok(_))) {
        let id = tree.element_with_key(&Key::Value(77)).unwrap();
        panic!(
            "negative maxima were accepted and laid out as {:?}",
            tree.element_bounds(id).unwrap().size
        );
    }
}

#[test]
fn a_child_without_any_layout_id_is_rejected() {
    let mut tree = WidgetTree::new();
    let child = Widget::from(SizedBox::new().width(100.)).with_key(Key::Value(90));
    let pref = Pref(Rc::new(std::cell::Cell::new(Size::new(10., 10.))));
    tree.mount(Center::new(CustomMultiChildLayout::new(pref, [child])).into())
        .unwrap();
    let result =
        std::panic::catch_unwind(AssertUnwindSafe(|| tree.layout(Constraints::tight(VIEW))));
    if matches!(result, Ok(Ok(_))) {
        let id = tree.element_with_key(&Key::Value(90)).unwrap();
        panic!(
            "missing LayoutId was accepted with {:?}",
            tree.element_bounds(id).unwrap().size
        );
    }
}
