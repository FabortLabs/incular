//! Restoration regression from Flutter 3.50's unregister lifecycle.
//! See `audits/flutter-3.50.0-parity/restoration-physics-comparisons.md`.

use incular_core::{RestorationBackend, RestorationKey, RestorationScope};
use incular_runtime::Restorable;
use serde_json::{Value, json};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Default)]
struct MemoryBackend(RefCell<HashMap<Vec<RestorationKey>, Value>>);

impl RestorationBackend for MemoryBackend {
    fn read_value(&self, path: &[RestorationKey]) -> Option<Value> {
        self.0.borrow().get(path).cloned()
    }

    fn write_value(&self, path: &[RestorationKey], value: Value) {
        self.0.borrow_mut().insert(path.to_vec(), value);
    }

    fn remove_value(&self, path: &[RestorationKey]) {
        self.0.borrow_mut().remove(path);
    }
}

#[test]
fn removed_restorable_does_not_reappear_on_later_edit() {
    let scope = RestorationScope::root(Rc::new(MemoryBackend::default()));
    let key = RestorationKey::new("draft").unwrap();
    let draft = Restorable::from_scope(scope.clone(), key.clone(), String::from("default"));

    assert!(draft.set("before".into()));
    assert_eq!(scope.get_json(&key), Some(json!("before")));
    draft.remove();
    assert_eq!(scope.get_json(&key), None);

    assert!(draft.set("after".into()));
    assert_eq!(draft.get(), "after");
    assert_eq!(scope.get_json(&key), None);
}
