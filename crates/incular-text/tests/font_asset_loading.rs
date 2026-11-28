//! `incular-assets` typed font assets feed `FontLoader` without adapters.

use std::{convert::Infallible, future::ready};

use incular_assets::{AssetBundle, FontAsset, MemoryAssetBundle};
use incular_text::FontLoader;

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, Wake, Waker};

    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: std::sync::Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(std::sync::Arc::new(ThreadWaker(std::thread::current())));
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
fn typed_font_asset_queues_through_font_loader() {
    // Synthetic font bytes; registration is observed through a custom registrar.
    let bundle = MemoryAssetBundle::new().with("fonts/Body.ttf", vec![0x00, 0x01, 0x00, 0x00]);
    let dyn_bundle: &dyn AssetBundle = &bundle;

    let mut loader = FontLoader::new("Body");
    loader
        .add_font(dyn_bundle.load_typed_future::<FontAsset>("fonts/Body.ttf"))
        .unwrap();
    // Directly-built typed assets also work as futures.
    loader
        .add_font(ready(Ok::<_, Infallible>(FontAsset::new(
            "fonts/Other.ttf",
            vec![1, 2, 3, 4],
        ))))
        .unwrap();

    let mut registered: Vec<(String, Vec<u8>)> = Vec::new();
    block_on(loader.load_with(|family, bytes| {
        registered.push((family.to_string(), bytes));
        ready(Ok::<(), Infallible>(()))
    }))
    .unwrap();

    assert_eq!(
        registered,
        vec![
            ("Body".to_string(), vec![0x00, 0x01, 0x00, 0x00]),
            ("Body".to_string(), vec![1, 2, 3, 4]),
        ]
    );
}
