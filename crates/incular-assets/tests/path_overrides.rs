//! Logical-to-physical path overrides and the overriding bundle wrapper.

use std::sync::Arc;

use incular_assets::{
    AssetBundle, AssetError, CachingAssetBundle, DirectoryAssetBundle, FontAsset,
    MemoryAssetBundle, PathOverrideBundle, PathOverrides,
};

#[test]
fn overrides_map_logical_to_physical_once() {
    let mut overrides = PathOverrides::new();
    assert!(overrides.is_empty());
    assert_eq!(overrides.resolve("a").as_ref(), "a");

    assert_eq!(overrides.insert("a", "b"), None);
    assert_eq!(overrides.insert("a", "c").as_deref(), Some("b"));
    assert!(overrides.contains("a"));
    assert_eq!(overrides.get("a"), Some("c"));
    assert_eq!(overrides.len(), 1);

    // Overrides apply once; targets are not re-resolved (no chains/cycles).
    overrides.insert("c", "a");
    assert_eq!(overrides.resolve("a").as_ref(), "c");
    assert_eq!(overrides.resolve("c").as_ref(), "a");

    // Unknown keys borrow.
    assert!(matches!(
        overrides.resolve("unknown"),
        std::borrow::Cow::Borrowed("unknown")
    ));

    assert_eq!(overrides.remove("a").as_deref(), Some("c"));
    assert!(!overrides.contains("a"));
    assert_eq!(overrides.remove("a"), None);

    overrides.insert("x", "y");
    overrides.clear();
    assert!(overrides.is_empty());

    // Ordered iteration.
    let mut pairs = overrides.clone();
    pairs.insert("b", "2");
    pairs.insert("a", "1");
    let collected: Vec<_> = pairs.iter().collect();
    assert_eq!(collected, vec![("a", "1"), ("b", "2")]);

    // FromIterator.
    let from_pairs: PathOverrides = [("a".to_owned(), "b".to_owned())].into_iter().collect();
    assert_eq!(from_pairs.get("a"), Some("b"));
    let from_arcs: PathOverrides = [(Arc::<str>::from("a"), Arc::<str>::from("b"))]
        .into_iter()
        .collect();
    assert_eq!(from_arcs.get("a"), Some("b"));
}

#[test]
fn overriding_bundle_resolves_logical_keys_to_physical() {
    let inner = MemoryAssetBundle::new()
        .with("physical/data.txt", b"real".to_vec())
        .with("other.txt", b"other".to_vec());
    let bundle =
        PathOverrideBundle::new(inner).with_path_override("logical.txt", "physical/data.txt");
    let dyn_bundle: &dyn AssetBundle = &bundle;

    // Byte and string APIs see the physical key's content...
    assert_eq!(&*dyn_bundle.load("logical.txt").unwrap(), b"real");
    assert_eq!(
        &*dyn_bundle.load_string("logical.txt", false).unwrap(),
        "real"
    );
    // ...and unoverridden keys pass through.
    assert_eq!(&*dyn_bundle.load("other.txt").unwrap(), b"other");

    // Typed loading uses the logical key for identity, physical for reads.
    let text: String = dyn_bundle.load_typed("logical.txt").unwrap();
    assert_eq!(text, "real");
    let font_key_bundle =
        PathOverrideBundle::new(MemoryAssetBundle::new().with("fonts/Real.ttf", vec![1, 2, 3]))
            .with_path_override("fonts/Alias.ttf", "fonts/Real.ttf");
    let font: FontAsset = (&font_key_bundle as &dyn AssetBundle)
        .load_typed("fonts/Alias.ttf")
        .unwrap();
    assert_eq!(
        font.key(),
        "fonts/Alias.ttf",
        "identity keeps the logical key"
    );
    assert_eq!(font.bytes().as_ref(), &[1, 2, 3]);

    // Errors are re-keyed to the logical key.
    let missing = dyn_bundle.load("nope").unwrap_err();
    assert_eq!(missing, AssetError::NotFound("nope".into()));
    let invalid = PathOverrideBundle::new(MemoryAssetBundle::new().with("bin", vec![0xff]))
        .with_path_override("text", "bin");
    assert_eq!(
        (&invalid as &dyn AssetBundle)
            .load_string("text", false)
            .unwrap_err(),
        AssetError::InvalidUtf8("text".into())
    );

    // Async loading resolves and re-keys too.
    let (tx, rx) = std::sync::mpsc::channel();
    dyn_bundle.load_async("logical.txt", Box::new(move |r| tx.send(r).unwrap()));
    assert_eq!(&*rx.recv().unwrap().unwrap(), b"real");
    let (tx, rx) = std::sync::mpsc::channel();
    dyn_bundle.load_async("missing-logical", Box::new(move |r| tx.send(r).unwrap()));
    assert_eq!(
        rx.recv().unwrap().unwrap_err(),
        AssetError::NotFound("missing-logical".into())
    );

    // Typed async loading routes through the same resolution.
    let typed_bundle = PathOverrideBundle::new(MemoryAssetBundle::new().with("p", b"v".to_vec()))
        .with_path_override("l", "p");
    let (tx, rx) = std::sync::mpsc::channel();
    (&typed_bundle as &dyn AssetBundle)
        .load_typed_async::<String>("l", Box::new(move |r| tx.send(r).unwrap()));
    assert_eq!(rx.recv().unwrap(), Ok("v".into()));
}

#[test]
fn overrides_are_live_and_composable() {
    let bundle = PathOverrideBundle::new(
        MemoryAssetBundle::new()
            .with("a", b"A".to_vec())
            .with("b", b"B".to_vec()),
    );
    let dyn_bundle: &dyn AssetBundle = &bundle;
    assert_eq!(
        dyn_bundle.load("logical").unwrap_err().to_string(),
        "Unable to load asset: \"logical\".\nThe asset does not exist or has empty data."
    );

    assert_eq!(bundle.add_path_override("logical", "a"), None);
    assert_eq!(&*dyn_bundle.load("logical").unwrap(), b"A");
    assert_eq!(bundle.resolve_key("logical"), "a");

    // Replace, then remove.
    assert_eq!(
        bundle.add_path_override("logical", "b").as_deref(),
        Some("a")
    );
    assert_eq!(&*dyn_bundle.load("logical").unwrap(), b"B");
    assert_eq!(bundle.remove_path_override("logical").as_deref(), Some("b"));
    assert!(dyn_bundle.load("logical").is_err());

    // Snapshot + bulk replace + clear.
    bundle.add_path_override("x", "a");
    assert_eq!(bundle.path_overrides().get("x"), Some("a"));
    let mut replacement = PathOverrides::new();
    replacement.insert("x", "b");
    bundle.set_path_overrides(replacement);
    assert_eq!(&*dyn_bundle.load("x").unwrap(), b"B");
    bundle.clear_path_overrides();
    assert!(bundle.path_overrides().is_empty());

    // `with_overrides` starts populated; inner accessors work.
    let inner = MemoryAssetBundle::new().with("a", b"A".to_vec());
    let mut start = PathOverrides::new();
    start.insert("l", "a");
    let mut bundle = PathOverrideBundle::with_overrides(inner, start);
    assert_eq!(bundle.inner().load("a").unwrap().as_ref(), b"A");
    let _ = bundle.inner_mut();
    assert_eq!(bundle.into_inner().load("a").unwrap().as_ref(), b"A");
}

#[test]
fn caching_inside_overrides_keys_by_physical_and_evicts_both() {
    let loads = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    struct Counting {
        loads: Arc<std::sync::atomic::AtomicUsize>,
        inner: MemoryAssetBundle,
    }
    impl AssetBundle for Counting {
        fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
            self.loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.inner.load(key)
        }
    }

    let bundle = PathOverrideBundle::new(CachingAssetBundle::new(Counting {
        loads: loads.clone(),
        inner: MemoryAssetBundle::new().with("physical", b"v".to_vec()),
    }))
    .with_path_override("logical", "physical");
    let dyn_bundle: &dyn AssetBundle = &bundle;

    dyn_bundle.load_string("logical", true).unwrap();
    dyn_bundle.load_string("logical", true).unwrap();
    assert_eq!(
        loads.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "cached under physical key"
    );

    // Evicting the logical key invalidates the physical cache entry.
    dyn_bundle.evict("logical");
    dyn_bundle.load_string("logical", true).unwrap();
    assert_eq!(loads.load(std::sync::atomic::Ordering::SeqCst), 2);

    dyn_bundle.clear();
    dyn_bundle.load_string("logical", true).unwrap();
    assert_eq!(loads.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
fn directory_bundle_override_maps_to_another_key_under_root() {
    let root =
        std::env::temp_dir().join(format!("incular-assets-overrides-{}", std::process::id()));
    std::fs::create_dir_all(root.join("alt")).unwrap();
    std::fs::write(root.join("main.txt"), "main").unwrap();
    std::fs::write(root.join("alt").join("other.txt"), "other").unwrap();

    let bundle = PathOverrideBundle::new(DirectoryAssetBundle::new(&root))
        .with_path_override("main.txt", "alt/other.txt");
    assert_eq!(
        &*(&bundle as &dyn AssetBundle)
            .load_string("main.txt", false)
            .unwrap(),
        "other"
    );
    // Overrides cannot escape the root any more than plain keys can.
    let escaping = PathOverrideBundle::new(DirectoryAssetBundle::new(&root))
        .with_path_override("x", "../outside");
    assert!((&escaping as &dyn AssetBundle).load("x").is_err());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn manifest_keys_resolve_through_overrides() {
    // Minimal empty binary manifest: standard-message-codec empty map
    // (`MAP` tag 13, length 0).
    let inner = MemoryAssetBundle::new().with("custom/manifest", vec![0x0D, 0x00]);
    let bundle =
        PathOverrideBundle::new(inner).with_path_override("AssetManifest.bin", "custom/manifest");
    let manifest = (&bundle as &dyn AssetBundle)
        .read_asset_manifest(incular_assets::AssetManifestFormat::Binary)
        .unwrap();
    assert!(manifest.list_assets().is_empty());
}
