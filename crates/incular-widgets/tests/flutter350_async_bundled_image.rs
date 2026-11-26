use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use incular_animation::scheduler::{run_one_scheduled_task, set_scheduled_task_wake};
use incular_assets::{
    AssetFetchCallback, AssetFetcher, AssetResponse, CachingAssetBundle, NetworkAssetBundle,
};
use incular_config::Constraints;
use incular_core::Size;
use incular_image::{ExactAssetImage, ImageLoadState, ImageProvider};
use incular_widgets::{DefaultAssetBundle, Image, internal::WidgetTree};

const RED_PNG: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/hashed_asset_1x.png"
);
const RED_KEY: &str = "assets/foo.9f64a747.png";
const LOGICAL: &str = "assets/foo.png";

#[derive(Clone, Default)]
struct ControlledFetcher(Arc<Mutex<VecDeque<(String, AssetFetchCallback)>>>);

impl AssetFetcher for ControlledFetcher {
    fn fetch(&self, url: &str, done: AssetFetchCallback) {
        self.0.lock().unwrap().push_back((url.into(), done));
    }
}

impl ControlledFetcher {
    fn take(&self, path: &str) -> AssetFetchCallback {
        let (url, done) = self
            .0
            .lock()
            .unwrap()
            .pop_front()
            .expect("fetch was not started");
        assert_eq!(url, format!("https://assets.invalid/fixture/{path}"));
        done
    }
}

fn hashed_manifest() -> Vec<u8> {
    use incular_core::standard_message_codec::{
        MessageValue::{List, Map, String as Text},
        StandardMessageCodec,
    };
    StandardMessageCodec::encode_message(&Map(vec![(
        Text(LOGICAL.into()),
        List(vec![Map(vec![(
            Text("asset".into()),
            Text(RED_KEY.into()),
        )])]),
    )]))
    .unwrap()
    .unwrap()
}

#[test]
fn a_callback_only_bundle_can_complete_a_retained_image_on_its_ui_thread() {
    let fetcher = ControlledFetcher::default();
    let bundle = Arc::new(CachingAssetBundle::new(NetworkAssetBundle::new(
        "https://assets.invalid/fixture/",
        fetcher.clone(),
    )));
    let wakes = Arc::new(AtomicUsize::new(0));
    set_scheduled_task_wake({
        let wakes = wakes.clone();
        move || {
            wakes.fetch_add(1, Ordering::Relaxed);
        }
    });

    let frames = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let image = Image::asset(LOGICAL).frame_builder({
        let frames = frames.clone();
        move |_, child, frame, synchronous| {
            frames.borrow_mut().push((frame, synchronous));
            child
        }
    });
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(DefaultAssetBundle::new(bundle, image).into())
        .unwrap();
    let constraints = Constraints::loose(Size::new(100.0, 100.0));
    tree.layout(constraints).unwrap();
    assert_eq!(tree.element_bounds(root).unwrap().size, Size::ZERO);
    let _ = run_one_scheduled_task(); // The pending UI task subscribes to its result.
    let wakes_before_bytes = wakes.load(Ordering::Relaxed);
    let manifest = fetcher.take("AssetManifest.bin");
    manifest(Ok(AssetResponse::new(200, hashed_manifest())));
    let pixels = fetcher.take(RED_KEY);
    std::thread::spawn(move || pixels(Ok(AssetResponse::new(200, RED_PNG))))
        .join()
        .unwrap();
    assert!(wakes.load(Ordering::Relaxed) > wakes_before_bytes);
    assert_eq!(*frames.borrow(), [(None, false)]);
    assert!(
        !tree.has_dirty_dependencies(),
        "the fetch thread cannot touch retained UI"
    );
    let _ = run_one_scheduled_task();
    assert!(tree.has_dirty_dependencies());
    tree.layout(constraints).unwrap();
    assert_eq!(*frames.borrow(), [(None, false), (Some(0), false)]);
    assert_eq!(tree.element_bounds(root).unwrap().size, Size::new(3.0, 2.0));
}

#[test]
fn exact_variant_failure_completes_without_fetching_a_manifest() {
    let fetcher = ControlledFetcher::default();
    let bundle = Arc::new(NetworkAssetBundle::new(
        "https://assets.invalid/fixture/",
        fetcher.clone(),
    ));
    let exact = ExactAssetImage::new("icon.png").bundle(bundle).scale(2.0);
    let stream = exact.resolve(&incular_image::ImageConfiguration::new());
    assert_eq!(stream.state(), ImageLoadState::Loading(None));
    let _ = run_one_scheduled_task();
    fetcher.take("icon.png")(Ok(AssetResponse::new(404, [])));
    let _ = run_one_scheduled_task();
    let ImageLoadState::Failed(error) = stream.state() else {
        panic!("the missing physical asset must complete the stream with an error");
    };
    assert!(error.to_string().contains("icon.png"));
    assert!(
        fetcher.0.lock().unwrap().is_empty(),
        "do not fall back to any variant"
    );
}
