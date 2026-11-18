//! Element teardown only disposes the effects that still belong to that owner.

use std::{cell::Cell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_runtime::{Effect, Memo, Runtime, Signal};
use incular_widgets::internal::{ElementId, Key};
use incular_widgets::{Column, Widget};

fn leaf(key: u64) -> Widget {
    Widget::box_(Size::new(10., 10.), Color::WHITE).with_key(Key::Value(key))
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(100., 100.)))
        .expect("frame");
}

fn slots(count: u64) -> (Runtime, ElementId, Vec<ElementId>) {
    let runtime = Runtime::new(Column::new((0..count).map(leaf)).into()).expect("runtime");
    let root = runtime.tree().root().expect("root");
    let ids = runtime.tree().children(root).expect("slots").to_vec();
    (runtime, root, ids)
}

fn mount_effect(runtime: &mut Runtime, owner: ElementId, key: u64, effect: Effect) {
    runtime
        .register_builder(owner, move || {
            assert!(effect.mount());
            leaf(key)
        })
        .expect("effect builder");
}

fn retain(runtime: &mut Runtime, root: ElementId, keys: impl IntoIterator<Item = u64>) {
    runtime
        .schedule_update(root, Column::new(keys.into_iter().map(leaf)).into())
        .expect("replace children");
    frame(runtime);
}

#[test]
fn removing_many_owners_disposes_their_effects_and_preserves_the_survivor() {
    let source = Signal::new(0_u32);
    let (mut runtime, root, owners) = slots(64);
    let mut effects = Vec::new();
    let mut runs = Vec::new();
    for (key, owner) in owners.into_iter().enumerate() {
        let count = Rc::new(Cell::new(0));
        let effect = Effect::new({
            let source = source.clone();
            let count = count.clone();
            move || {
                let _ = source.get();
                count.set(count.get() + 1);
            }
        });
        mount_effect(&mut runtime, owner, key as u64, effect.clone());
        effects.push(effect);
        runs.push(count);
    }
    frame(&mut runtime);
    assert_eq!(source.dependent_count(), 64);

    retain(&mut runtime, root, [63]);
    assert!(effects[..63].iter().all(|effect| !effect.is_mounted()));
    assert!(effects[63].is_mounted());
    assert_eq!(source.dependent_count(), 1);
    source.set(1);
    frame(&mut runtime);
    assert!(runs[..63].iter().all(|runs| runs.get() == 1));
    assert_eq!(runs[63].get(), 2);
}

#[test]
fn an_effect_without_dependencies_is_still_disposed_with_its_owner() {
    let (mut runtime, root, owners) = slots(1);
    let effect = Effect::new(|| {});
    mount_effect(&mut runtime, owners[0], 0, effect.clone());
    frame(&mut runtime);
    assert!(effect.is_mounted());
    retain(&mut runtime, root, []);
    assert!(!effect.is_mounted());
}

#[test]
fn explicitly_disposed_effect_can_move_to_a_different_owner_in_the_same_runtime() {
    let source = Signal::new(0_u32);
    let seen = Rc::new(Cell::new(0));
    let effect = Effect::new({
        let source = source.clone();
        let seen = seen.clone();
        move || seen.set(source.get())
    });
    let (mut runtime, root, owners) = slots(2);
    mount_effect(&mut runtime, owners[0], 0, effect.clone());
    frame(&mut runtime);
    effect.dispose();
    assert_eq!(source.dependent_count(), 0);

    mount_effect(&mut runtime, owners[1], 1, effect.clone());
    frame(&mut runtime);
    retain(&mut runtime, root, [1]);
    assert!(
        effect.is_mounted(),
        "removing the old owner must not dispose the new binding"
    );
    source.set(7);
    frame(&mut runtime);
    assert_eq!(seen.get(), 7);
    assert_eq!(source.dependent_count(), 1);
    retain(&mut runtime, root, []);
    assert!(!effect.is_mounted());
    assert_eq!(source.dependent_count(), 0);
}

#[test]
fn moving_one_of_several_effects_preserves_both_owner_groups() {
    for moved in 0..3 {
        let source = Signal::new(0_u32);
        let effects: Vec<_> = (0..3)
            .map(|_| {
                let source = source.clone();
                Effect::new(move || {
                    let _ = source.get();
                })
            })
            .collect();
        let (mut runtime, root, owners) = slots(2);
        runtime
            .register_builder(owners[0], {
                let effects = effects.clone();
                move || {
                    for effect in &effects {
                        assert!(effect.mount());
                    }
                    leaf(0)
                }
            })
            .expect("shared effect owner");
        frame(&mut runtime);
        assert_eq!(source.dependent_count(), 3);

        effects[moved].dispose();
        mount_effect(&mut runtime, owners[1], 1, effects[moved].clone());
        frame(&mut runtime);
        assert_eq!(source.dependent_count(), 3);
        retain(&mut runtime, root, [1]);
        for (index, effect) in effects.iter().enumerate() {
            assert_eq!(effect.is_mounted(), index == moved);
        }
        assert_eq!(source.dependent_count(), 1);
        source.set(1);
        frame(&mut runtime);
        assert!(effects[moved].is_mounted());
        retain(&mut runtime, root, []);
        assert!(!effects[moved].is_mounted());
        assert_eq!(source.dependent_count(), 0);
    }
}

#[test]
fn transferred_effect_survives_old_owner_removal_and_old_runtime_drop() {
    let source = Signal::new(0_u32);
    let seen = Rc::new(Cell::new(0));
    let effect = Effect::new({
        let source = source.clone();
        let seen = seen.clone();
        move || seen.set(source.get())
    });
    let (mut old, old_root, old_owners) = slots(1);
    let (mut current, _, current_owners) = slots(1);
    mount_effect(&mut old, old_owners[0], 0, effect.clone());
    frame(&mut old);
    mount_effect(&mut current, current_owners[0], 0, effect.clone());
    frame(&mut current);

    retain(&mut old, old_root, []);
    drop(old);
    assert!(effect.is_mounted());
    source.set(9);
    frame(&mut current);
    assert_eq!(seen.get(), 9);
    assert_eq!(source.dependent_count(), 1);
    drop(current);
    assert!(!effect.is_mounted());
    assert_eq!(source.dependent_count(), 0);
}

#[test]
fn shared_memo_survives_removal_of_an_effect_and_one_memo_consumer() {
    let source = Signal::new(2_u32);
    let memo = Memo::new({
        let source = source.clone();
        move || source.get() * 2
    });
    let effect = Effect::new({
        let source = source.clone();
        move || {
            let _ = source.get();
        }
    });
    let seen = Rc::new(Cell::new(0));
    let (mut runtime, root, owners) = slots(3);
    mount_effect(&mut runtime, owners[0], 0, effect.clone());
    for key in [1_usize, 2] {
        runtime
            .register_builder(owners[key], {
                let memo = memo.clone();
                let seen = seen.clone();
                move || {
                    seen.set(memo.get());
                    leaf(key as u64)
                }
            })
            .expect("memo consumer");
    }
    frame(&mut runtime);
    assert_eq!(source.dependent_count(), 2);
    retain(&mut runtime, root, [2]);
    assert!(!effect.is_mounted());
    assert_eq!(source.dependent_count(), 1);
    source.set(6);
    frame(&mut runtime);
    assert_eq!(seen.get(), 12);
    assert_eq!(source.dependent_count(), 1);
}

#[test]
fn ownership_index_does_not_keep_dropped_effects_alive() {
    struct DropProbe(Rc<Cell<bool>>);
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    let source = Signal::new(0_u32);
    let dropped = Rc::new(Cell::new(false));
    let probe = DropProbe(dropped.clone());
    let effect = Effect::new({
        let source = source.clone();
        move || {
            let _ = &probe;
            let _ = source.get();
        }
    });
    let (mut runtime, root, owners) = slots(1);
    mount_effect(&mut runtime, owners[0], 0, effect);
    frame(&mut runtime);
    assert!(!dropped.get());
    retain(&mut runtime, root, []);
    assert!(dropped.get());
    assert_eq!(source.dependent_count(), 0);
}
