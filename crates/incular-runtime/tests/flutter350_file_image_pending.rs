//! The mounted FileImage path uses the host worker and the real image cache.
#![cfg(not(target_arch = "wasm32"))]

use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant, SystemTime},
};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_image::{FileImage, ImageError, ImageSource};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{ProviderImage, Widget};

const PNG: &[u8] = include_bytes!(
    "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let stem = format!(
            "incular-file-image-runtime-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        );
        let root = std::env::temp_dir().join(stem);
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn file(&self) -> PathBuf {
        self.0.join("image.png")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn frame(runtime: &mut Runtime) -> DisplayList {
    runtime
        .run_frame(Constraints::tight(Size::new(32., 32.)))
        .unwrap()
        .0
}

fn until(runtime: &mut Runtime, mut saw: impl FnMut(&DisplayList) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let list = frame(runtime);
        if saw(&list) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "mounted file image did not settle"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn a_mounted_file_image_reports_the_empty_error_and_a_later_mount_decodes_the_written_file() {
    let fixture = Fixture::new();
    let path = fixture.file();
    fs::write(&path, []).unwrap();
    let provider = FileImage::new(&path).scale(2.);
    let errors = Rc::new(RefCell::new(Vec::new()));
    let mut original = Runtime::new(
        ProviderImage::new(provider.clone())
            .error_builder({
                let errors = errors.clone();
                move |_, error| {
                    if errors.borrow().is_empty() {
                        errors.borrow_mut().push(error.clone());
                    }
                    Widget::box_(Size::new(32., 32.), Color::BLACK)
                }
            })
            .into(),
    )
    .unwrap();
    assert!(errors.borrow().is_empty());
    let initial = frame(&mut original);
    assert!(
        initial
            .commands()
            .iter()
            .all(|c| !matches!(c, PaintCommand::Image { .. }))
    );
    if errors.borrow().is_empty() {
        until(&mut original, |_| !errors.borrow().is_empty());
    }
    assert_eq!(
        errors.borrow().as_slice(),
        &[ImageError::EmptyFile { path: path.clone() }]
    );
    assert_eq!(provider.cache_status(), Default::default());
    drop(original);

    fs::write(&path, PNG).unwrap();
    let mut mounted = Runtime::new(ProviderImage::new(provider.clone()).into()).unwrap();
    until(&mut mounted, |list| {
        list.commands().iter().any(|command| match command {
            PaintCommand::Image { image, .. } => {
                assert_eq!(
                    (
                        image.decoded().width(),
                        image.decoded().height(),
                        image.scale()
                    ),
                    (8, 8, 2.)
                );
                assert_eq!(image.source(), &ImageSource::FileSystem(path.clone()));
                true
            }
            _ => false,
        })
    });
    // The actual hosted widget used the shared async cache, not the legacy
    // headless synchronous fallback.
    assert!(provider.cache_status().keep_alive);
    assert!(!provider.cache_status().pending);
    provider.evict();
}
