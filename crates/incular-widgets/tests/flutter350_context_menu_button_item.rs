use std::{
    cell::Cell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
};

use incular_widgets::{
    ContextMenuButtonItem as Item, ContextMenuButtonItemUpdate as Update,
    ContextMenuButtonKind as Kind, ContextMenuButtonType as Type,
};

fn hash(item: &Item) -> u64 {
    let mut hash = DefaultHasher::new();
    item.hash(&mut hash);
    hash.finish()
}

#[test]
fn defaults_to_custom() {
    let item = Item::default();
    assert_eq!(item.kind(), Kind::Custom);
    assert_eq!(item.button_type(), Type::Custom);
    assert!(item.on_pressed.is_none());
}

#[test]
fn every_legacy_type_maps_to_a_kind_and_back() {
    for old in Type::VALUES {
        let item = Item::disabled(old);
        assert_eq!(item.button_type(), old);
        assert_eq!(item.kind().name(), old.name());
    }
}

#[test]
fn each_matching_kind_reports_its_legacy_type() {
    for kind in Kind::VALUES {
        let item = Item::disabled(kind);
        assert_eq!(item.kind(), kind);
        if kind != Kind::Translate {
            assert_eq!(item.button_type().name(), kind.name());
        }
    }
}

#[test]
fn translate_has_no_legacy_type_and_does_not_equal_custom() {
    let translate = Item::disabled(Kind::Translate);
    assert_eq!(translate.button_type(), Type::Custom);
    assert_ne!(translate, Item::disabled(Type::Custom));
    assert_eq!(translate.resolved_label(), "Translate");
    assert_eq!(
        translate.label("Custom label").resolved_label(),
        "Custom label"
    );
}

#[test]
fn equivalent_kind_and_legacy_values_compare_and_hash_identically() {
    let old = Item::disabled(Type::Copy);
    let kind = Item::disabled(Kind::Copy);
    assert_eq!(old, kind);
    assert_eq!(hash(&old), hash(&kind));

    let hits = Rc::new(Cell::new(0));
    let first = Item::new(Type::Copy, {
        let hits = hits.clone();
        move || hits.set(hits.get() + 1)
    });
    let second = first.clone();
    assert_eq!(first, second);
    assert_eq!(hash(&first), hash(&second));
    assert_ne!(first, Item::new(Kind::Copy, || {}));
    second.press();
    assert_eq!(hits.get(), 1);
}

#[test]
fn copy_with_accepts_new_kinds_or_legacy_types_without_losing_other_fields() {
    let original = Item::new(Kind::Copy, || {});
    for key in [Kind::Paste, Type::Cut.into()] {
        let updated = original.copy_with(Update {
            kind: Some(key),
            ..Update::default()
        });
        assert_eq!(updated.kind(), key);
        assert!(Rc::ptr_eq(
            updated.on_pressed.as_ref().unwrap(),
            original.on_pressed.as_ref().unwrap()
        ));
    }
    let renamed = original.copy_with(Update {
        label: Some("hi".into()),
        ..Update::default()
    });
    assert_eq!(renamed.kind(), Kind::Copy);
    assert_eq!(renamed.resolved_label(), "hi");
}

#[test]
fn kinds_support_constant_patterns_in_consumers() {
    let describe = |kind| match kind {
        Kind::Copy => "copy",
        Kind::Translate => "translate",
        _ => "other",
    };
    assert_eq!(describe(Kind::Copy), "copy");
    assert_eq!(describe(Kind::Translate), "translate");
    assert_eq!(describe(Kind::Paste), "other");
}
