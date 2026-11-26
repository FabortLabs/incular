use std::{
    cell::RefCell,
    io::Write,
    pin::Pin,
    rc::Rc,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

use flate2::{Compression, write::GzEncoder};
use futures_core::Stream;
use incular_assets::{
    AssetBundle, AssetError, LicenseNoticeFormat, MemoryAssetBundle, load_license_notices,
    register_license_notices,
};
use incular_core::{LicenseEntry, LicenseRegistry};

fn notices_document() -> String {
    [
        "L1Package1\nL1Package2\nL1Package3\n\nL1Paragraph1\n\nL1Paragraph2\n\nL1Paragraph3",
        "L2Package1\nL2Package2\nL2Package3\n\nL2Paragraph1\n\nL2Paragraph2\n\nL2Paragraph3",
    ]
    .join(&format!("\n{}\n", "-".repeat(80)))
}

fn gzip(text: &[u8]) -> Vec<u8> {
    let mut writer = GzEncoder::new(Vec::new(), Compression::fast());
    writer.write_all(text).unwrap();
    writer.finish().unwrap()
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Read {
    Bytes(String, thread::ThreadId),
    Text(String, bool),
}

#[derive(Default)]
struct TestBundle {
    storage: MemoryAssetBundle,
    reads: Mutex<Vec<Read>>,
}

impl AssetBundle for TestBundle {
    fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
        self.reads
            .lock()
            .unwrap()
            .push(Read::Bytes(key.into(), thread::current().id()));
        self.storage.load(key)
    }

    fn load_string(&self, key: &str, cache: bool) -> Result<Arc<str>, AssetError> {
        self.reads
            .lock()
            .unwrap()
            .push(Read::Text(key.into(), cache));
        self.storage.load_string(key, cache)
    }
}

fn bundled() -> TestBundle {
    let mut bundle = TestBundle::default();
    let text = notices_document();
    bundle.storage.insert("NOTICES.Z", gzip(text.as_bytes()));
    bundle.storage.insert("NOTICES", text.into_bytes());
    bundle
}

fn assert_original_entries(licenses: &[LicenseEntry]) {
    assert_eq!(licenses.len(), 2);
    for (index, entry) in licenses.iter().enumerate() {
        let n = index + 1;
        assert_eq!(
            entry.packages.as_ref(),
            [1, 2, 3].map(|suffix| format!("L{n}Package{suffix}")),
        );
        let paragraphs = entry.paragraphs().map(|p| p.text).collect::<Vec<_>>();
        assert_eq!(
            paragraphs,
            [1, 2, 3].map(|suffix| format!("L{n}Paragraph{suffix}"))
        );
    }
}

#[test]
fn the_web_path_requests_utf8_without_populating_the_bundle_cache() {
    let bundle = bundled();
    assert_original_entries(&load_license_notices(&bundle, LicenseNoticeFormat::WebText).unwrap());
    assert_eq!(
        *bundle.reads.lock().unwrap(),
        [Read::Text("NOTICES".into(), false)]
    );
}

struct SignalThread(thread::Thread);

impl Wake for SignalThread {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn collect(mut stream: impl Stream<Item = LicenseEntry> + Unpin) -> Vec<LicenseEntry> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let waker = Waker::from(Arc::new(SignalThread(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut out = Vec::new();
    loop {
        match Pin::new(&mut stream).poll_next(&mut context) {
            Poll::Ready(Some(license)) => out.push(license),
            Poll::Ready(None) => return out,
            Poll::Pending => {
                let now = Instant::now();
                assert!(now < deadline, "asset worker did not wake the collector");
                thread::park_timeout(deadline - now);
            }
        }
    }
}

#[test]
fn native_registry_lazily_loads_actual_gzip_and_repeats_the_original_order() {
    LicenseRegistry::reset();
    let bundle = Arc::new(bundled());
    register_license_notices(bundle.clone(), LicenseNoticeFormat::NativeGzip, |error| {
        panic!("unexpected notice error: {error}")
    });

    let first = LicenseRegistry::licenses();
    assert!(bundle.reads.lock().unwrap().is_empty());
    assert_original_entries(&collect(first));
    assert_original_entries(&collect(LicenseRegistry::licenses()));
    let reads = bundle.reads.lock().unwrap();
    let host = thread::current().id();
    assert_eq!(reads.len(), 2);
    assert!(
        reads
            .iter()
            .all(|read| matches!(read, Read::Bytes(key, on) if key == "NOTICES.Z" && *on != host))
    );
    LicenseRegistry::reset();
}

#[test]
fn a_broken_notice_reports_an_error_without_stopping_other_collectors() {
    LicenseRegistry::reset();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let mut bundle = MemoryAssetBundle::new();
    bundle.insert("NOTICES.Z", b"not a gzip".to_vec());
    register_license_notices(Arc::new(bundle), LicenseNoticeFormat::NativeGzip, {
        let errors = errors.clone();
        move |error| errors.borrow_mut().push(error)
    });
    LicenseRegistry::add_sync(|| [LicenseEntry::with_line_breaks(["built-in"], "fallback")]);
    let out = collect(LicenseRegistry::licenses());
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].packages.as_ref(), ["built-in"]);
    let errors = errors.borrow();
    assert!(matches!(errors.as_slice(), [AssetError::Parse { key, .. }] if key == "NOTICES.Z"));
    LicenseRegistry::reset();
}

#[test]
fn package_less_notices_and_invalid_utf8_use_their_real_asset_boundary() {
    let mut bundle = MemoryAssetBundle::new();
    let text = format!("First\nSecond\n{}\nlabel\n\n本文", "-".repeat(80));
    bundle.insert("NOTICES.Z", gzip(text.as_bytes()));
    let licenses = load_license_notices(&bundle, LicenseNoticeFormat::NativeGzip).unwrap();
    assert!(licenses[0].packages.is_empty());
    assert_eq!(
        licenses[0].paragraphs().next().unwrap().text,
        "First Second"
    );
    assert_eq!(licenses[1].packages.as_ref(), ["label"]);
    assert_eq!(licenses[1].paragraphs().next().unwrap().text, "本文");

    bundle.insert("NOTICES.Z", gzip(&[0xff]));
    assert_eq!(
        load_license_notices(&bundle, LicenseNoticeFormat::NativeGzip),
        Err(AssetError::InvalidUtf8("NOTICES.Z".into()))
    );
    assert_eq!(
        load_license_notices(&bundle, LicenseNoticeFormat::WebText),
        Err(AssetError::NotFound("NOTICES".into()))
    );
}

#[test]
fn the_web_collector_does_not_use_or_evict_preexisting_cached_text() {
    use incular_assets::CachingAssetBundle;

    struct ReplaceableText(Arc<Mutex<Arc<[u8]>>>);
    impl AssetBundle for ReplaceableText {
        fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
            if key == "NOTICES" {
                Ok(self.0.lock().unwrap().clone())
            } else {
                Err(AssetError::NotFound(key.into()))
            }
        }
    }
    LicenseRegistry::reset();
    let contents: Arc<Mutex<Arc<[u8]>>> = Arc::new(Mutex::new(Arc::from(&b"Old notice"[..])));
    let bundle = Arc::new(CachingAssetBundle::new(ReplaceableText(contents.clone())));
    assert_eq!(
        bundle.load_string("NOTICES", true).unwrap().as_ref(),
        "Old notice"
    );
    *contents.lock().unwrap() = Arc::from(&b"demo\n\nNew notice"[..]);

    register_license_notices(bundle.clone(), LicenseNoticeFormat::WebText, |error| {
        panic!("{error}")
    });
    let licenses = collect(LicenseRegistry::licenses());
    assert_eq!(licenses.len(), 1);
    assert_eq!(licenses[0].packages.as_ref(), ["demo"]);
    assert_eq!(licenses[0].paragraphs().next().unwrap().text, "New notice");
    assert_eq!(
        bundle.load_string("NOTICES", true).unwrap().as_ref(),
        "Old notice"
    );
    LicenseRegistry::reset();
}
