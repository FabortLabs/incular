//! Flutter's `SharedAppData` (`widgets/shared_app_data.dart`) and
//! `InheritedModel` (`widgets/inherited_model.dart`): inherited values whose
//! dependents rebuild only for the keys or aspects they read.

use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
    rc::Rc,
};

use incular_core::reactivity::DependencySource;

use crate::tree::RetainedState;
use crate::{BuildContext, Widget};

/// A [`SharedAppData`] key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SharedAppDataKey(Rc<str>);

impl From<&str> for SharedAppDataKey {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for SharedAppDataKey {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

#[derive(Default)]
struct SharedEntry {
    value: Option<Rc<dyn Any>>,
    changes: DependencySource,
}

/// The values a [`SharedAppData`] holds. Clones share the same values.
#[derive(Clone, Default)]
pub struct SharedAppDataStore {
    entries: Rc<RefCell<HashMap<SharedAppDataKey, SharedEntry>>>,
}

impl fmt::Debug for SharedAppDataStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SharedAppDataStore")
            .field("keys", &self.entries.borrow().keys().collect::<Vec<_>>())
            .finish()
    }
}

impl SharedAppDataStore {
    /// `SharedAppData.getValue`: the value for `key`, stored from `init` the
    /// first time without a rebuild. Reading it inside a build rebuilds that
    /// build when [`Self::set_value`] changes this key.
    ///
    /// # Panics
    /// Panics when the stored value is not a `V`, where Flutter's cast
    /// throws.
    pub fn get_value<V: Clone + 'static>(
        &self,
        key: impl Into<SharedAppDataKey>,
        init: impl FnOnce() -> V,
    ) -> V {
        let key = key.into();
        let stored = {
            let mut entries = self.entries.borrow_mut();
            let entry = entries.entry(key.clone()).or_default();
            entry.changes.track();
            entry.value.clone()
        };
        // `init` runs without a borrow, so it may use the store itself.
        let value = stored.unwrap_or_else(|| {
            let value: Rc<dyn Any> = Rc::new(init());
            if let Some(entry) = self.entries.borrow_mut().get_mut(&key) {
                entry.value.get_or_insert_with(|| value.clone());
            }
            value
        });
        value
            .downcast_ref::<V>()
            .unwrap_or_else(|| panic!("SharedAppData value for {key:?} has another type"))
            .clone()
    }

    /// `SharedAppData.setValue`: stores `value` and rebuilds the builds that
    /// read `key`, unless it equals the stored value.
    pub fn set_value<V: PartialEq + 'static>(&self, key: impl Into<SharedAppDataKey>, value: V) {
        let changes = {
            let mut entries = self.entries.borrow_mut();
            let entry = entries.entry(key.into()).or_default();
            if entry
                .value
                .as_ref()
                .and_then(|current| current.downcast_ref::<V>())
                .is_some_and(|current| *current == value)
            {
                return;
            }
            entry.value = Some(Rc::new(value));
            entry.changes.clone()
        };
        changes.notify();
    }
}

/// Flutter's `SharedAppData`: application-wide values keyed by name. Each
/// build that reads a key rebuilds only when that key changes.
pub struct SharedAppData {
    child: Widget,
}

impl SharedAppData {
    #[must_use]
    pub fn new(child: impl Into<Widget>) -> Self {
        Self {
            child: child.into(),
        }
    }

    /// The nearest store, to read in a build or keep for a callback.
    #[must_use]
    pub fn of(context: &BuildContext<'_>) -> Option<SharedAppDataStore> {
        context
            .find_shared::<SharedAppDataStore>()
            .map(|store| (*store).clone())
    }

    /// `SharedAppData.getValue` through the nearest store.
    ///
    /// # Panics
    /// Panics without a `SharedAppData` ancestor, or for a value of another
    /// type, as Flutter does.
    pub fn get_value<V: Clone + 'static>(
        context: &BuildContext<'_>,
        key: impl Into<SharedAppDataKey>,
        init: impl FnOnce() -> V,
    ) -> V {
        Self::of(context)
            .expect("SharedAppData.getValue needs a SharedAppData ancestor")
            .get_value(key, init)
    }
}

struct SharedAppDataState(Rc<SharedAppDataStore>);

impl RetainedState for SharedAppDataState {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl From<SharedAppData> for Widget {
    fn from(value: SharedAppData) -> Self {
        Widget::retained_builder(
            |_| SharedAppDataState(Rc::new(SharedAppDataStore::default())),
            move |state, _, _| {
                Widget::shared_environment_scope(state.0.clone(), value.child.clone())
            },
            false,
        )
    }
}

/// The data an [`InheritedModel`] provides: Flutter's `InheritedModel`
/// subclass, with its `updateShouldNotify` and
/// `updateShouldNotifyDependent`.
pub trait InheritedModelData: 'static {
    type Aspect: Hash + Eq + Clone + 'static;

    /// Whether any dependent might need to rebuild.
    fn update_should_notify(&self, old: &Self) -> bool;

    /// Whether a dependent on `aspects` rebuilds. Each aspect a dependent
    /// reads is asked about on its own, so a dependent rebuilds when any of
    /// its aspects needs it.
    fn update_should_notify_dependent(&self, old: &Self, aspects: &HashSet<Self::Aspect>) -> bool;
}

struct ModelCell<M: InheritedModelData> {
    model: RefCell<Option<Rc<M>>>,
    /// Dependents that read no aspect.
    all: DependencySource,
    aspects: RefCell<HashMap<M::Aspect, DependencySource>>,
}

impl<M: InheritedModelData> ModelCell<M> {
    fn update(&self, model: Rc<M>) {
        let Some(old) = self.model.replace(Some(model.clone())) else {
            return;
        };
        if Rc::ptr_eq(&old, &model) || !model.update_should_notify(&old) {
            return;
        }
        self.all.notify();
        let aspects: Vec<_> = self
            .aspects
            .borrow()
            .iter()
            .map(|(aspect, source)| (aspect.clone(), source.clone()))
            .collect();
        for (aspect, source) in aspects {
            if model.update_should_notify_dependent(&old, &HashSet::from([aspect])) {
                source.notify();
            }
        }
    }
}

/// The scope an [`InheritedModel`] installs; a new one each rebuild, so its
/// notifications reach dependents in the same layout.
struct ModelScope<M: InheritedModelData>(Rc<ModelCell<M>>);

struct ModelState<M: InheritedModelData>(Rc<ModelCell<M>>);

impl<M: InheritedModelData> RetainedState for ModelState<M> {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

/// Flutter's `InheritedModel`: provides `model` to descendants, which
/// rebuild only when the aspects they read change.
pub struct InheritedModel<M> {
    model: Rc<M>,
    child: Widget,
}

impl<M: InheritedModelData> InheritedModel<M> {
    #[must_use]
    pub fn new(model: M, child: impl Into<Widget>) -> Self {
        Self {
            model: Rc::new(model),
            child: child.into(),
        }
    }

    /// `InheritedModel.inheritFrom`: the nearest model, rebuilding this
    /// build when `aspect` changes, or on any notifying change without one.
    #[must_use]
    pub fn inherit_from(context: &BuildContext<'_>, aspect: Option<M::Aspect>) -> Option<Rc<M>> {
        let scope = context.find_shared::<ModelScope<M>>()?;
        let cell = &scope.0;
        match aspect {
            Some(aspect) => {
                cell.aspects.borrow_mut().entry(aspect).or_default().track();
            }
            None => {
                cell.all.track();
            }
        }
        cell.model.borrow().clone()
    }
}

impl<M: InheritedModelData> From<InheritedModel<M>> for Widget {
    fn from(value: InheritedModel<M>) -> Self {
        Widget::retained_builder(
            |_| {
                ModelState::<M>(Rc::new(ModelCell {
                    model: RefCell::new(None),
                    all: DependencySource::default(),
                    aspects: RefCell::new(HashMap::new()),
                }))
            },
            move |state, _, _| {
                state.0.update(value.model.clone());
                Widget::shared_environment_scope(
                    Rc::new(ModelScope(state.0.clone())),
                    value.child.clone(),
                )
            },
            false,
        )
    }
}
