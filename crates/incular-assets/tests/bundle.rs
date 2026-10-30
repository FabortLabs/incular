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
        "Unable to load asset: \"missing\"."
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
