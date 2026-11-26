//! Flutter's `AssetBundle`, `CachingAssetBundle` and a directory bundle.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use incular_assets::{
    AssetBundle, AssetError, CachingAssetBundle, DirectoryAssetBundle, MemoryAssetBundle,
};

#[test]
fn memory_bundle_loads_bytes_and_strings() {
    let bundle = MemoryAssetBundle::new()
        .with("data/hello.txt", b"hello".to_vec())
        .with("bad.bin", vec![0xff, 0xfe]);
    assert_eq!(&*bundle.load("data/hello.txt").unwrap(), b"hello");
    assert_eq!(
        &*bundle.load_string("data/hello.txt", true).unwrap(),
        "hello"
    );
    assert_eq!(
        bundle.load("missing").unwrap_err(),
        AssetError::NotFound("missing".into())
    );
    assert_eq!(
        bundle.load("missing").unwrap_err().to_string(),
        "Unable to load asset: \"missing\".\nThe asset does not exist or has empty data."
    );
    assert_eq!(
        bundle.load_string("bad.bin", false).unwrap_err(),
        AssetError::InvalidUtf8("bad.bin".into())
    );
    let bundle: &dyn AssetBundle = &bundle;
    assert_eq!(
        bundle.load_structured_data("data/hello.txt", |text| Ok::<_, String>(text.len())),
        Ok(5)
    );
    assert_eq!(
        bundle.load_structured_data("data/hello.txt", |_| Err::<(), _>("bad")),
        Err(AssetError::Parse {
            key: "data/hello.txt".into(),
            message: "bad".into()
        })
    );
}

/// Counts loads, so caching is observable.
struct Counting {
    loads: Arc<AtomicUsize>,
    inner: MemoryAssetBundle,
}

impl AssetBundle for Counting {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        self.inner.load(key)
    }
}

#[test]
fn caching_bundle_keeps_strings_and_parsed_data_until_evicted() {
    let loads = Arc::new(AtomicUsize::new(0));
    let bundle = CachingAssetBundle::new(Counting {
        loads: loads.clone(),
        inner: MemoryAssetBundle::new().with("a", b"12".to_vec()),
    });
    let count = || loads.load(Ordering::SeqCst);
    bundle.load_string("a", true).unwrap();
    bundle.load_string("a", true).unwrap();
    assert_eq!(count(), 1);
    // An uncached read always loads; bytes are never cached.
    bundle.load_string("a", false).unwrap();
    bundle.load("a").unwrap();
    assert_eq!(count(), 3);
    let parse = |text: &str| text.parse::<u32>();
    assert_eq!(bundle.load_structured_data("a", parse), Ok(12));
    assert_eq!(bundle.load_structured_data("a", parse), Ok(12));
    assert_eq!(count(), 4);
    bundle.evict("a");
    bundle.load_string("a", true).unwrap();
    assert_eq!(bundle.load_structured_data("a", parse), Ok(12));
    assert_eq!(count(), 6);
    bundle.clear();
    bundle.load_string("a", true).unwrap();
    assert_eq!(count(), 7);
}

#[test]
fn caching_bundle_reports_a_structured_type_mismatch() {
    let bundle = CachingAssetBundle::new(MemoryAssetBundle::new().with("a", b"12".to_vec()));
    assert_eq!(
        bundle.load_structured_data("a", |text| text.parse::<u32>()),
        Ok(12)
    );
    let error = bundle
        .load_structured_data("a", |text| Ok::<_, String>(text.to_owned()))
        .unwrap_err();
    assert_eq!(error, AssetError::TypeMismatch { key: "a".into() });
    assert_eq!(
        error.to_string(),
        "Asset \"a\" is cached as a different type."
    );
    // The original entry is untouched.
    assert_eq!(
        bundle.load_structured_data("a", |text| text.parse::<u32>()),
        Ok(12)
    );
}

#[test]
fn directory_bundle_reads_files_under_its_root_only() {
    let root = std::env::temp_dir().join(format!("incular-assets-{}", std::process::id()));
    std::fs::create_dir_all(root.join("images")).unwrap();
    std::fs::write(root.join("images/logo.txt"), "logo").unwrap();
    let bundle = DirectoryAssetBundle::new(&root);
    assert_eq!(
        &*bundle.load_string("images/logo.txt", false).unwrap(),
        "logo"
    );
    for key in [
        "images/missing",
        "../outside",
        "images//logo.txt",
        "./images/logo.txt",
    ] {
        assert_eq!(
            bundle.load(key).unwrap_err(),
            AssetError::NotFound(key.into()),
            "{key}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn root_asset_missing_error_keeps_the_key_and_explanation() {
    use incular_assets::{replace_root_asset_bundle, root_asset_bundle};
    let old = replace_root_asset_bundle(None);
    let error = root_asset_bundle().load("not-exists").unwrap_err();
    assert_eq!(error, AssetError::NotFound("not-exists".into()));
    assert_eq!(
        error.to_string(),
        "Unable to load asset: \"not-exists\".\nThe asset does not exist or has empty data."
    );
    // The diagnostic describes a missing platform response. An explicit
    // zero-length byte response is not the same thing as missing.
    let bundle = MemoryAssetBundle::new().with("empty", Arc::<[u8]>::default());
    replace_root_asset_bundle(Some(Arc::new(bundle)));
    assert!(root_asset_bundle().load("empty").unwrap().is_empty());
    replace_root_asset_bundle(old);
}
