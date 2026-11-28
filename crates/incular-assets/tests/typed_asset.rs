//! Typed asset handles, async loading, clear states, font and shader
//! assets. Byte and string bundle APIs stay available.

use std::sync::{Arc, Mutex, PoisonError};

use incular_assets::{
    AssetBundle, AssetError, AssetFuture, AssetHandle, AssetId, AssetLoadState, FontAsset,
    FontBytes, FontId, FontLocation, MemoryAssetBundle, ShaderAsset, ShaderLanguage, TypedAsset,
};

fn bundle() -> MemoryAssetBundle {
    MemoryAssetBundle::new()
        .with("hello.txt", b"hello".to_vec())
        .with("bad.bin", vec![0xff, 0xfe])
        .with("fonts/Body.ttf", vec![0, 1, 2, 3, 4])
        .with("shaders/glow.wgsl", b"@fragment fn main() {}".to_vec())
        .with(
            "shaders/compute.glsl",
            b"#version 450\nvoid main() {}".to_vec(),
        )
}

#[test]
fn typed_handle_identity_and_key() {
    let first: AssetHandle<String> = AssetHandle::new("a/b.txt");
    let second: AssetHandle<String> = AssetHandle::new("a/b.txt");
    assert_ne!(first.id(), second.id(), "fresh handles get unique ids");
    assert_eq!(first.key(), "a/b.txt");

    let named = AssetHandle::<String>::with_id(AssetId(7), "x");
    assert_eq!(named.id(), AssetId(7));
    assert_eq!(named.clone(), named);
    assert!(format!("{named:?}").contains("AssetHandle"));

    // Different typed handles over the same key compare independently.
    let as_font: AssetHandle<FontAsset> = AssetHandle::with_id(AssetId(7), "x");
    assert_eq!(as_font.id(), named.id());
}

#[test]
fn typed_loading_keeps_byte_and_string_apis() {
    let bundle = bundle();
    let dyn_bundle: &dyn AssetBundle = &bundle;

    // Existing byte and string APIs are untouched.
    assert_eq!(&*dyn_bundle.load("hello.txt").unwrap(), b"hello");
    assert_eq!(
        &*dyn_bundle.load_string("hello.txt", false).unwrap(),
        "hello"
    );

    // Built-in typed assets.
    let text: String = dyn_bundle.load_typed("hello.txt").unwrap();
    assert_eq!(text, "hello");
    let bytes: Vec<u8> = dyn_bundle.load_typed("hello.txt").unwrap();
    assert_eq!(bytes, b"hello");
    let shared: Arc<[u8]> = dyn_bundle.load_typed("hello.txt").unwrap();
    assert_eq!(&*shared, b"hello");
    let shared_text: Arc<str> = dyn_bundle.load_typed("hello.txt").unwrap();
    assert_eq!(&*shared_text, "hello");

    // Errors keep their kind.
    assert_eq!(
        dyn_bundle.load_typed::<String>("missing").unwrap_err(),
        AssetError::NotFound("missing".into())
    );
    assert_eq!(
        dyn_bundle.load_typed::<String>("bad.bin").unwrap_err(),
        AssetError::InvalidUtf8("bad.bin".into())
    );
}

/// A custom typed asset.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Greeting(String);

impl TypedAsset for Greeting {
    fn decode_asset(key: &str, bytes: &[u8]) -> Result<Self, AssetError> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| AssetError::InvalidUtf8(key.to_owned()))?;
        Ok(Self(text.trim().to_owned()))
    }
}

#[test]
fn custom_typed_assets_and_handles() {
    let bundle = bundle();
    let dyn_bundle: &dyn AssetBundle = &bundle;
    let handle = AssetHandle::<Greeting>::new("hello.txt");

    assert_eq!(handle.load(dyn_bundle).unwrap(), Greeting("hello".into()));

    let async_result = Arc::new(Mutex::new(Vec::new()));
    let sink = async_result.clone();
    handle.load_async(
        dyn_bundle,
        Box::new(move |result| {
            sink.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(result)
        }),
    );
    let got = async_result
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(got, vec![Ok(Greeting("hello".into()))]);
}

/// Minimal executor for tests; no async runtime is required by the crate.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, Wake, Waker};

    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

#[test]
fn async_typed_loading_reports_loading_ready_failed_states() {
    let bundle = bundle();
    let dyn_bundle: &dyn AssetBundle = &bundle;

    // A synchronously completing bundle yields an already-ready future.
    let future: AssetFuture<String> = dyn_bundle.load_typed_future("hello.txt");
    assert!(future.is_ready(), "sync bundles complete immediately");
    assert_eq!(future.state(), AssetLoadState::Ready("hello".into()));
    assert_eq!(block_on(future), Ok("hello".into()));

    // Missing assets resolve to a `Failed` state, not a panic.
    let failed: AssetFuture<String> = dyn_bundle.load_typed_future("missing");
    assert_eq!(
        failed.state(),
        AssetLoadState::Failed(AssetError::NotFound("missing".into()))
    );
    assert_eq!(
        block_on(failed).unwrap_err(),
        AssetError::NotFound("missing".into())
    );

    // `AssetLoadState` shape.
    let loading: AssetLoadState<String> = AssetLoadState::Loading;
    assert!(loading.is_loading() && !loading.is_ready() && !loading.is_failed());
    assert_eq!(loading.ready(), None);
    let ready = AssetLoadState::Ready(3u32);
    assert!(ready.is_ready());
    assert_eq!(ready.ready(), Some(&3));
    assert_eq!(ready.clone().into_ready(), Some(3));
    assert_eq!(ready.map(|n| n + 1), AssetLoadState::Ready(4));
    let failed: AssetLoadState<u32> = AssetLoadState::Failed(AssetError::NotFound("k".into()));
    assert_eq!(failed.error(), Some(&AssetError::NotFound("k".into())));
    assert_eq!(
        AssetLoadState::<u32>::Loading.map(|n| n + 1),
        AssetLoadState::Loading
    );
    assert_eq!(
        AssetLoadState::<u32>::from_result(Err(AssetError::NotFound("k".into()))),
        AssetLoadState::Failed(AssetError::NotFound("k".into()))
    );
}

#[test]
fn async_typed_loading_is_non_blocking_on_slow_bundles() {
    // A bundle that completes on another thread later.
    struct Slow;
    impl AssetBundle for Slow {
        fn load(&self, key: &str) -> Result<Arc<[u8]>, AssetError> {
            Ok(Arc::from(key.as_bytes()))
        }
        fn load_async(&self, key: &str, done: incular_assets::AssetLoadCallback) {
            let bytes: Arc<[u8]> = Arc::from(key.as_bytes());
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(10));
                done(Ok(bytes));
            });
        }
    }

    let slow = Slow;
    let future: AssetFuture<String> = (&slow as &dyn AssetBundle).load_typed_future("later");
    // Initially `Loading`; no blocking on construction.
    assert!(future.is_loading());
    assert_eq!(future.state(), AssetLoadState::Loading);
    assert_eq!(block_on(future), Ok("later".into()));

    // Callback form is likewise non-blocking.
    let (tx, rx) = std::sync::mpsc::channel();
    (&slow as &dyn AssetBundle)
        .load_typed_async::<String>("soon", Box::new(move |result| tx.send(result).unwrap()));
    assert_eq!(rx.recv().unwrap(), Ok("soon".into()));
}

#[test]
fn font_asset_typed_loading_for_incular_text() {
    let bundle = bundle();
    let dyn_bundle: &dyn AssetBundle = &bundle;

    let font: FontAsset = dyn_bundle.load_typed("fonts/Body.ttf").unwrap();
    assert_eq!(font.key(), "fonts/Body.ttf");
    assert_eq!(font.bytes().as_ref(), &[0, 1, 2, 3, 4]);
    assert_eq!(font.face_index(), 0);
    assert!(font.location().is_default());
    assert_eq!(font.family(), None);

    // Handle-based async font loading.
    let handle = AssetHandle::<FontAsset>::new("fonts/Body.ttf");
    let loaded = block_on(handle.load_future(dyn_bundle)).unwrap();
    assert_eq!(loaded, font);

    // Converts to an `incular-text` `FontHandle` with caller-owned identity.
    let handle = loaded.clone().to_font_handle(FontId(42));
    assert_eq!(handle.id(), FontId(42));
    assert_eq!(handle.bytes().as_ref(), &[0, 1, 2, 3, 4]);
    assert_eq!(handle.face_index(), 0);

    // Face index / variation location / family are configurable and retained.
    let varied = loaded
        .with_face_index(2)
        .with_location(FontLocation::from_normalized_coordinates(&[0, -8192]))
        .with_family("Body");
    assert_eq!(varied.face_index(), 2);
    assert_eq!(varied.location().coordinates(), &[0, -8192]);
    assert_eq!(varied.family(), Some("Body"));
    let varied_handle = varied.to_font_handle(FontId(9));
    assert_eq!(varied_handle.face_index(), 2);
    assert_eq!(varied_handle.location().coordinates(), &[0, -8192]);

    // `incular-text`'s `FontLoader::add_font` accepts `B: Into<Vec<u8>>`.
    let bytes: Vec<u8> = varied.clone().into();
    assert_eq!(bytes, vec![0, 1, 2, 3, 4]);
    assert_eq!(varied.into_bytes(), vec![0, 1, 2, 3, 4]);

    // FontBytes sharing stays cheap.
    let shared = FontBytes::from(vec![9, 9]);
    let font = FontAsset::new("k", shared);
    assert_eq!(font.bytes().as_ref(), &[9, 9]);
}

#[test]
fn shader_asset_typed_loading() {
    let bundle = bundle();
    let dyn_bundle: &dyn AssetBundle = &bundle;

    let shader: ShaderAsset = dyn_bundle.load_typed("shaders/glow.wgsl").unwrap();
    assert_eq!(shader.key(), "shaders/glow.wgsl");
    assert_eq!(shader.language(), ShaderLanguage::Wgsl);
    assert_eq!(shader.source_text().unwrap(), "@fragment fn main() {}");
    assert_eq!(shader.source_bytes(), b"@fragment fn main() {}");

    let glsl: ShaderAsset = dyn_bundle.load_typed("shaders/compute.glsl").unwrap();
    assert_eq!(glsl.language(), ShaderLanguage::Glsl);

    // Handle-based async shader loading.
    let handle = AssetHandle::<ShaderAsset>::new("shaders/glow.wgsl");
    let shader = block_on(handle.load_future(dyn_bundle)).unwrap();
    assert_eq!(shader.language(), ShaderLanguage::Wgsl);

    // Language inference from extension; unknown defaults to WGSL.
    assert_eq!(ShaderLanguage::from_key("a/b.vert"), ShaderLanguage::Glsl);
    assert_eq!(ShaderLanguage::from_key("a.metal"), ShaderLanguage::Metal);
    assert_eq!(ShaderLanguage::from_key("a.hlsl"), ShaderLanguage::Hlsl);
    assert_eq!(ShaderLanguage::from_key("a.spv"), ShaderLanguage::SpirV);
    assert_eq!(ShaderLanguage::from_key("a.spirv"), ShaderLanguage::SpirV);
    assert_eq!(ShaderLanguage::from_key("a.unknown"), ShaderLanguage::Wgsl);
    assert_eq!(ShaderLanguage::from_key("a"), ShaderLanguage::Wgsl);
    assert!(ShaderLanguage::Wgsl.is_text() && !ShaderLanguage::SpirV.is_text());

    // Binary SPIR-V payloads pass through; text shaders must be UTF-8.
    let bin = MemoryAssetBundle::new().with("s.spv", vec![0x03, 0x02, 0x23, 0x07]);
    let spirv: ShaderAsset = (&bin as &dyn AssetBundle).load_typed("s.spv").unwrap();
    assert_eq!(spirv.language(), ShaderLanguage::SpirV);
    assert!(spirv.source_text().is_err());

    let invalid = MemoryAssetBundle::new().with("s.wgsl", vec![0xff, 0xfe]);
    assert_eq!(
        (&invalid as &dyn AssetBundle)
            .load_typed::<ShaderAsset>("s.wgsl")
            .unwrap_err(),
        AssetError::InvalidUtf8("s.wgsl".into())
    );

    // Explicit language override.
    let shader = ShaderAsset::new(
        "x",
        Arc::<[u8]>::from(b"src".as_slice()),
        ShaderLanguage::Wgsl,
    )
    .with_language(ShaderLanguage::Glsl);
    assert_eq!(shader.language(), ShaderLanguage::Glsl);
}

#[test]
fn async_callback_form_reports_errors_with_key() {
    let bundle = bundle();
    let result = Arc::new(Mutex::new(None));
    let sink = result.clone();
    (&bundle as &dyn AssetBundle).load_typed_async::<ShaderAsset>(
        "nope.wgsl",
        Box::new(move |outcome| {
            *sink.lock().unwrap_or_else(PoisonError::into_inner) = Some(outcome)
        }),
    );
    let outcome = result
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .unwrap();
    assert_eq!(
        outcome.unwrap_err(),
        AssetError::NotFound("nope.wgsl".into())
    );
}
