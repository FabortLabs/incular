use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
};

use incular_core::{Color, ColorSwatch};

fn key_hash(value: &ColorSwatch<&str>) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn swatch<K: Copy + Eq + Hash, const N: usize>(
    primary: u32,
    shades: [(K, u32); N],
) -> ColorSwatch<K> {
    ColorSwatch::new(
        Color::from_argb32(primary),
        shades.map(|(key, color)| (key, Color::from_argb32(color))),
    )
}

#[test]
fn named_shades_preserve_lookup_order_and_order_independent_equality() {
    let entries = [
        ("2259 C", 0xff027223),
        ("2273 C", 0xff257226),
        ("2426 XGC", 0xff00932f),
        ("7732 XGC", 0xff007940),
    ];
    let first = swatch(0xff027223, entries);
    let second = swatch(0xff027223, entries);
    assert_eq!(first, second);
    assert_eq!(key_hash(&first), key_hash(&second));
    assert_eq!(first.get("2259 C"), Some(Color::from_argb32(0xff027223)));
    assert_eq!(first.get("unlisted"), None);
    assert_eq!(first.to_argb32(), 0xff027223);
    assert_eq!(
        first.keys().copied().collect::<Vec<_>>(),
        entries.map(|(key, _)| key)
    );

    let mut reversed = entries;
    reversed.reverse();
    let reordered = swatch(0xff027223, reversed);
    assert_eq!(first, reordered);
    assert_eq!(key_hash(&first), key_hash(&reordered));
    assert_eq!(
        reordered.keys().copied().collect::<Vec<_>>(),
        reversed.map(|(key, _)| key)
    );
}

#[test]
fn transparent_to_white_clamps_extremes_and_rounds_each_midpoint_byte() {
    let a = swatch(0x00000000, [(1, 0x00000000)]);
    let b = swatch(0xffffffff, [(1, 0xffffffff)]);
    for (t, packed) in [
        (0., 0),
        (0.5, 0x80808080),
        (1., 0xffffffff),
        (-0.1, 0),
        (1.1, 0xffffffff),
        (f64::NAN, 0xffffffff),
    ] {
        let mixed = ColorSwatch::lerp(Some(&a), Some(&b), t).unwrap();
        assert_eq!(mixed.to_argb32(), packed);
        assert_eq!(mixed.get(&1).unwrap().to_argb32(), packed);
        assert_eq!(mixed.keys().copied().collect::<Vec<_>>(), [1]);
    }
}

#[test]
fn identity_does_not_clone_and_two_absent_swatches_remain_absent() {
    assert!(ColorSwatch::<u8>::lerp(None, None, 0.).is_none());
    let a = swatch(0x00000000, [(1, 0x00000000)]);
    let same = ColorSwatch::lerp(Some(&a), Some(&a), 0.5).unwrap();
    let Cow::Borrowed(original) = same else {
        panic!("identical inputs should not allocate");
    };
    assert!(std::ptr::eq(original, &a));
}

#[test]
fn missing_shades_fade_without_affecting_the_key_order_or_rgb() {
    let a = swatch(
        0xff106030,
        [
            ("left-only", 0xff207040),
            ("shared", 0x00000000),
            ("another-left", 0xff408060),
        ],
    );
    let b = swatch(
        0xff509070,
        [("shared", 0xffffffff), ("right-only", 0xff60a080)],
    );
    let middle = ColorSwatch::lerp(Some(&a), Some(&b), 0.5).unwrap();
    assert_eq!(
        middle.keys().copied().collect::<Vec<_>>(),
        ["left-only", "shared", "another-left"]
    );
    assert_eq!(middle.get("right-only"), None);
    assert_eq!(middle.get("left-only").unwrap().to_argb32(), 0x80207040);
    assert_eq!(middle.get("shared").unwrap().to_argb32(), 0x80808080);
    assert_eq!(middle.to_argb32(), 0xff307850);

    let enter = ColorSwatch::lerp(None, Some(&b), 0.5).unwrap();
    assert_eq!(
        enter.keys().copied().collect::<Vec<_>>(),
        ["shared", "right-only"]
    );
    assert_eq!(enter.get("right-only").unwrap().to_argb32(), 0x8060a080);
    let exit = ColorSwatch::lerp(Some(&a), None, 0.5).unwrap();
    assert!(exit.keys().eq(a.keys()));
    assert_eq!(exit.to_argb32(), 0x80106030);

    let endpoint = ColorSwatch::lerp(Some(&a), Some(&b), 1.).unwrap();
    assert!(endpoint.keys().eq(a.keys()));
    assert_eq!(endpoint.get("left-only").unwrap().to_argb32(), 0x00207040);
}
