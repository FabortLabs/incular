//! Flutter's `SharedAppData` and `InheritedModel`: dependents rebuild only
//! for the keys or aspects they read.

use std::{cell::Cell, collections::HashSet, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    Builder, Column, InheritedModel, InheritedModelData, SharedAppData, SharedAppDataStore, Widget,
};

fn layout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(500., 500.)))
        .expect("layout");
}

/// A builder that counts its builds and shows `width` as a box.
fn counted(
    builds: &Rc<Cell<u32>>,
    width: impl Fn(&incular_widgets::BuildContext<'_>) -> f32 + 'static,
) -> Widget {
    let builds = builds.clone();
    Builder::new(move |context| {
        builds.set(builds.get() + 1);
        Widget::box_(Size::new(width(context), 10.), Color::WHITE)
    })
    .into()
}

#[test]
fn shared_app_data_rebuilds_only_the_readers_of_a_changed_key() {
    let store: Rc<Cell<Option<SharedAppDataStore>>> = Rc::new(Cell::new(None));
    let (a_builds, b_builds) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let a = {
        let store = store.clone();
        counted(&a_builds, move |context| {
            store.set(SharedAppData::of(context));
            SharedAppData::get_value(context, "a", || 1_f32)
        })
    };
    let b = counted(&b_builds, |context| {
        SharedAppData::get_value(context, "b", || 2_f32)
    });
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(SharedAppData::new(Column::new([a, b])).into())
        .expect("mount");
    layout(&mut tree);
    assert_eq!((a_builds.get(), b_builds.get()), (1, 1));

    let store = store.take().expect("store");
    // Setting the value it already has rebuilds nothing.
    store.set_value("a", 1_f32);
    assert!(!tree.has_dirty_dependencies());
    store.set_value("a", 30_f32);
    assert!(tree.has_dirty_dependencies());
    layout(&mut tree);
    assert_eq!((a_builds.get(), b_builds.get()), (2, 1));
    let a_element = tree.children(tree.children(root).unwrap()[0]).unwrap()[0];
    assert_eq!(tree.element_bounds(a_element).unwrap().size.width, 30.);
    assert_eq!(store.get_value("b", || 0_f32), 2.);
}

#[derive(Clone, Copy, PartialEq)]
struct Pair {
    first: f32,
    second: f32,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Part {
    First,
    Second,
}

impl InheritedModelData for Pair {
    type Aspect = Part;

    fn update_should_notify(&self, old: &Self) -> bool {
        self != old
    }

    fn update_should_notify_dependent(&self, old: &Self, aspects: &HashSet<Part>) -> bool {
        (aspects.contains(&Part::First) && self.first != old.first)
            || (aspects.contains(&Part::Second) && self.second != old.second)
    }
}

#[test]
fn inherited_model_rebuilds_dependents_by_aspect() {
    let builds: Vec<Rc<Cell<u32>>> = (0..3).map(|_| Rc::new(Cell::new(0))).collect();
    // The same child widget each time, so only the model's notifications
    // can rebuild the readers.
    let children: Widget = Column::new([
        counted(&builds[0], |context| {
            InheritedModel::<Pair>::inherit_from(context, Some(Part::First))
                .map_or(0., |pair| pair.first)
        }),
        counted(&builds[1], |context| {
            InheritedModel::<Pair>::inherit_from(context, Some(Part::Second))
                .map_or(0., |pair| pair.second)
        }),
        counted(&builds[2], |context| {
            InheritedModel::<Pair>::inherit_from(context, None).map_or(0., |pair| pair.first)
        }),
    ])
    .into();
    let model = |first, second| -> Widget {
        InheritedModel::new(Pair { first, second }, children.clone()).into()
    };
    let counts = || builds.iter().map(|b| b.get()).collect::<Vec<_>>();
    let mut tree = WidgetTree::new();
    let root = tree.mount(model(1., 2.)).expect("mount");
    layout(&mut tree);
    assert_eq!(counts(), [1, 1, 1]);

    // An equal model notifies nobody.
    tree.update(root, model(1., 2.)).expect("update");
    layout(&mut tree);
    assert_eq!(counts(), [1, 1, 1]);

    // The first part changes: its reader and the aspect-less reader rebuild.
    tree.update(root, model(5., 2.)).expect("update");
    layout(&mut tree);
    assert_eq!(counts(), [2, 1, 2]);

    // The second part changes: only its reader and the aspect-less one.
    tree.update(root, model(5., 7.)).expect("update");
    layout(&mut tree);
    assert_eq!(counts(), [2, 2, 3]);
}
