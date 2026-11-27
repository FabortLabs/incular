//! Flutter's `Image(image: provider)` (`widgets/image.dart` `_ImageState`,
//! paint-012): the image follows its provider's stream through loading,
//! ready and failed states, with the frame, loading and error builders and
//! gapless playback.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_image::{ImageError, ImageHandle};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::{
    Image, ImageChunkEvent, ImageConfiguration, ImageProvider, ImageStream, ImageStreamCompleter,
    Widget,
};

/// A provider that loads synchronously through the default `resolve`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Immediate(ImageHandle);

impl ImageProvider for Immediate {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        Ok(self.0.clone())
    }
}

/// A provider whose stream the test completes, counting its resolutions.
#[derive(Clone, Debug)]
struct Deferred {
    id: u32,
    stream: ImageStream,
    resolutions: Rc<Cell<u32>>,
}

impl PartialEq for Deferred {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Deferred {}

impl ImageProvider for Deferred {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        Err(ImageError::Io("deferred providers resolve a stream".into()))
    }

    fn resolve(&self, _: &ImageConfiguration) -> ImageStream {
        self.resolutions.set(self.resolutions.get() + 1);
        self.stream.clone()
    }
}

fn deferred(id: u32) -> (Deferred, ImageStreamCompleter) {
    let (stream, completer) = ImageStream::loading();
    let provider = Deferred {
        id,
        stream,
        resolutions: Rc::default(),
    };
    (provider, completer)
}

/// An opaque `width` x `height` image.
fn pixels(width: u32, height: u32) -> ImageHandle {
    let len = usize::try_from(width * height * 4).expect("small image");
    ImageHandle::from_rgba8(width, height, vec![255; len]).expect("image")
}

/// The `(frame, was_synchronously_loaded)` pairs a frame builder saw.
type FrameLog = Rc<RefCell<Vec<(Option<u32>, bool)>>>;

struct Harness {
    tree: WidgetTree,
    root: ElementId,
}

impl Harness {
    fn new(widget: impl Into<Widget>) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget.into()).expect("mount");
        let mut harness = Self { tree, root };
        harness.layout();
        harness
    }

    fn layout(&mut self) {
        self.tree
            .layout(Constraints::loose(Size::new(200., 200.)))
            .expect("layout");
    }

    fn update(&mut self, widget: impl Into<Widget>) {
        self.tree.update(self.root, widget.into()).expect("update");
        self.layout();
    }

    fn size(&self) -> Size {
        self.tree.element_bounds(self.root).expect("bounds").size
    }
}

#[test]
fn a_loading_image_is_an_empty_box_until_its_stream_completes() {
    let (provider, completer) = deferred(1);
    let mut harness = Harness::new(Image::provider(provider).width(30.));
    // Flutter's `RawImage` with no image: the explicit width, the smallest
    // height.
    assert_eq!(harness.size(), Size::new(30., 0.));
    assert!(completer.complete(pixels(30, 12)));
    assert!(harness.tree.has_dirty_dependencies());
    harness.layout();
    assert_eq!(harness.size(), Size::new(30., 12.));
}

#[test]
fn the_loading_builder_sees_each_chunk_and_none_once_loaded() {
    let (provider, completer) = deferred(1);
    let seen: Rc<RefCell<Vec<Option<ImageChunkEvent>>>> = Rc::default();
    let image = Image::provider(provider).loading_builder({
        let seen = seen.clone();
        move |_, child, progress| {
            seen.borrow_mut().push(progress);
            child
        }
    });
    let mut harness = Harness::new(image);
    let chunk = ImageChunkEvent::new(5, Some(20));
    completer.report_chunk(chunk);
    harness.layout();
    completer.complete(pixels(4, 4));
    harness.layout();
    assert_eq!(*seen.borrow(), [None, Some(chunk), None]);
}

#[test]
fn the_frame_builder_reports_the_first_frame_and_synchronous_loads() {
    let frames: FrameLog = Rc::default();
    let record = |frames: &FrameLog| {
        let frames = frames.clone();
        move |_: &incular_widgets::BuildContext<'_>, child: Widget, frame, synchronous| {
            frames.borrow_mut().push((frame, synchronous));
            child
        }
    };

    let (provider, completer) = deferred(1);
    let mut harness = Harness::new(Image::provider(provider).frame_builder(record(&frames)));
    completer.complete(pixels(4, 4));
    harness.layout();
    assert_eq!(*frames.borrow(), [(None, false), (Some(0), false)]);

    // A provider that loads synchronously is ready on the first build.
    frames.borrow_mut().clear();
    let harness =
        Harness::new(Image::provider(Immediate(pixels(3, 2))).frame_builder(record(&frames)));
    assert_eq!(*frames.borrow(), [(Some(0), true)]);
    assert_eq!(harness.size(), Size::new(3., 2.));
}

#[test]
fn a_failure_shows_the_error_builder() {
    let (provider, completer) = deferred(1);
    let errors: Rc<RefCell<Vec<ImageError>>> = Rc::default();
    let image = Image::provider(provider).error_builder({
        let errors = errors.clone();
        move |_, error: &ImageError| {
            errors.borrow_mut().push(error.clone());
            Widget::box_(Size::new(9., 9.), Color::WHITE)
        }
    });
    let mut harness = Harness::new(image);
    completer.fail(ImageError::Decode("bad bytes".into()));
    harness.layout();
    assert_eq!(harness.size(), Size::new(9., 9.));
    assert_eq!(*errors.borrow(), [ImageError::Decode("bad bytes".into())]);
}

#[test]
fn a_failure_without_an_error_builder_is_the_empty_box() {
    let (provider, completer) = deferred(1);
    let mut harness = Harness::new(Image::provider(provider).width(10.).height(6.));
    completer.fail(ImageError::InvalidDimensions);
    harness.layout();
    assert_eq!(harness.size(), Size::new(10., 6.));
}

#[test]
fn gapless_playback_keeps_the_old_image_while_a_new_provider_loads() {
    let (first, first_done) = deferred(1);
    first_done.complete(pixels(20, 10));
    let mut harness = Harness::new(Image::provider(first.clone()).gapless_playback(true));
    assert_eq!(harness.size(), Size::new(20., 10.));

    let (second, second_done) = deferred(2);
    harness.update(Image::provider(second).gapless_playback(true));
    assert_eq!(harness.size(), Size::new(20., 10.), "the old image stays");
    second_done.complete(pixels(8, 8));
    harness.layout();
    assert_eq!(harness.size(), Size::new(8., 8.));

    // Without gapless playback a new provider empties the box at once.
    let (third, _third_done) = deferred(3);
    harness.update(Image::provider(third));
    assert_eq!(harness.size(), Size::ZERO);
}

#[test]
fn an_equal_provider_keeps_its_stream() {
    let (provider, completer) = deferred(1);
    let resolutions = provider.resolutions.clone();
    let mut harness = Harness::new(Image::provider(provider.clone()));
    harness.update(Image::provider(provider.clone()));
    harness.update(Image::provider(provider));
    assert_eq!(resolutions.get(), 1);
    completer.complete(pixels(2, 2));
    harness.layout();
    assert_eq!(harness.size(), Size::new(2., 2.));
}

#[test]
fn a_semantic_label_describes_the_image() {
    let (provider, completer) = deferred(1);
    completer.complete(pixels(4, 4));
    let mut harness = Harness::new(Image::provider(provider).semantic_label("A cat"));
    harness.tree.update_semantics();
    let node = harness
        .tree
        .semantics()
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.label.as_deref() == Some("A cat"))
        .expect("labelled image");
    assert_eq!(node.role, incular_semantics::SemanticRole::Image);
    harness.layout();
}

#[test]
fn updating_memory_image_density_reflows_without_replacing_the_source_pixels() {
    use incular_image::MemoryImage;
    use incular_rendering::PaintCommand;

    let encoded: &[u8] = include_bytes!(
        "../../../audits/flutter-3.50.0-parity/reference-probes/fixtures/decode_cache_8x8.png"
    );
    let provider = MemoryImage::new(encoded);
    let mut harness = Harness::new(Image::provider(provider.clone()));
    assert_eq!(harness.size(), Size::new(8., 8.));
    let painted = |tree: &mut WidgetTree| {
        tree.paint()
            .commands()
            .iter()
            .find_map(|cmd| match cmd {
                PaintCommand::Image {
                    destination, image, ..
                } => Some((destination.size, image.id())),
                _ => None,
            })
            .expect("provider paints a decoded image")
    };
    let original = painted(&mut harness.tree);
    assert_eq!(original.0, Size::new(8., 8.));

    harness.update(Image::provider(provider.scale(2.)));
    assert_eq!(harness.size(), Size::new(4., 4.));
    assert_eq!(painted(&mut harness.tree), (Size::new(4., 4.), original.1));
}
