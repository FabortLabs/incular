//! Pinned Flutter widgets/box_decoration_test.dart image loading and reparent.
use incular_config::Constraints;
use incular_core::{Offset, Rect, Size, Transform};
use incular_image::{ImageError, ImageHandle};
use incular_rendering::{DisplayList, PaintCommand};
use incular_runtime::Runtime;
use incular_widgets::{
    BoxDecoration, DecoratedBox, DecorationImage, GlobalKey, ImageChunkEvent, ImageConfiguration,
    ImageProvider, ImageStream, ImageStreamCompleter, Positioned, RepaintBoundary, SizedBox, Stack,
    Widget,
};
use std::{cell::RefCell, rc::Rc};

const BOX: Size = Size {
    width: 40.,
    height: 30.,
};

#[derive(Clone, Debug)]
struct Deferred {
    stream: ImageStream,
    resolutions: Rc<RefCell<Vec<ImageConfiguration>>>,
}

impl PartialEq for Deferred {
    fn eq(&self, other: &Self) -> bool {
        self.stream == other.stream
    }
}
impl Eq for Deferred {}

impl ImageProvider for Deferred {
    fn load(&self) -> Result<ImageHandle, ImageError> {
        panic!("decoration must observe the provider's stream, not block on load");
    }

    fn resolve(&self, config: &ImageConfiguration) -> ImageStream {
        self.resolutions.borrow_mut().push(config.clone());
        self.stream.clone()
    }
}

fn pending() -> (Deferred, ImageStreamCompleter) {
    let (stream, signal) = ImageStream::loading();
    (
        Deferred {
            stream,
            resolutions: Rc::default(),
        },
        signal,
    )
}

fn decoration(provider: Deferred) -> Widget {
    DecoratedBox::new(SizedBox::new().width(BOX.width).height(BOX.height))
        .decoration(BoxDecoration::new().image(DecorationImage::from_provider(provider)))
        .into()
}

fn frame(runtime: &mut Runtime) -> DisplayList {
    runtime
        .run_frame(Constraints::tight(Size::new(300., 200.)))
        .unwrap()
        .0
}

fn pix() -> ImageHandle {
    ImageHandle::from_rgba8(2, 2, vec![255; 16]).unwrap()
}

fn draws(list: &DisplayList) -> Vec<(u64, Rect)> {
    let mut transforms = vec![Transform::IDENTITY];
    let mut images = Vec::new();
    for command in list.commands() {
        let current = *transforms.last().unwrap();
        match command {
            PaintCommand::PushTransform { transform } => {
                transforms.push(current.then(*transform));
            }
            PaintCommand::PopTransform => {
                transforms.pop();
            }
            PaintCommand::Image {
                image, destination, ..
            } => images.push((image.id().0, current.transform_rect_bbox(*destination))),
            _ => {}
        }
    }
    images
}

#[test]
fn a_deferred_decoration_stays_idle_until_the_decoded_frame_arrives() {
    let (provider, completion) = pending();
    let resolutions = provider.resolutions.clone();
    let mut runtime = Runtime::new(Stack::new([decoration(provider)]).into()).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
    assert_eq!(resolutions.borrow().len(), 1);
    assert_eq!(resolutions.borrow()[0].size, Some(BOX));

    // A decoration supplies no loading progress UI; it should stay idle.
    assert!(completion.report_chunk(ImageChunkEvent::new(5, Some(16))));
    assert!(!runtime.frame_requested());

    let done = pix();
    assert!(completion.complete(done.clone()));
    assert!(runtime.frame_requested());
    let draw = draws(&frame(&mut runtime));
    assert_eq!(draw.len(), 1);
    assert_eq!(draw[0].0, done.id().0);
    assert!(!runtime.frame_requested());
}

#[test]
fn a_moved_keyed_decoration_keeps_its_pending_resolution_and_repaints_at_the_new_origin() {
    let (provider, completion) = pending();
    let resolutions = provider.resolutions.clone();
    let key = GlobalKey::new();
    let subtree = Widget::from(RepaintBoundary::new(decoration(provider))).with_key(key);
    let make_scene = |moved| {
        let nested = if moved {
            Positioned::builder()
                .left(80.)
                .top(37.)
                .child(subtree.clone())
                .build()
                .into()
        } else {
            subtree.clone()
        };
        Widget::from(Stack::new([nested]))
    };
    let mut runtime = Runtime::new(make_scene(false)).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
    let identity = runtime.tree().element_with_key(&key.into()).unwrap();
    let root = runtime.tree().root().unwrap();

    runtime.schedule_update(root, make_scene(true)).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
    assert_eq!(runtime.tree().element_with_key(&key.into()), Some(identity));
    assert_eq!(
        resolutions.borrow().len(),
        1,
        "moving is not a source change"
    );

    let done = pix();
    assert!(completion.complete(done.clone()));
    assert!(runtime.frame_requested());
    let painted = draws(&frame(&mut runtime));
    assert_eq!(
        painted,
        [(
            done.id().0,
            Rect::from_origin_size(Offset::new(99., 51.), Size::new(2., 2.))
        )]
    );
    assert!(!runtime.frame_requested());
}

#[test]
fn an_obsolete_decorations_stream_does_not_repaint_the_new_source() {
    let (old, old_completion) = pending();
    let (new, new_completion) = pending();
    let scene = |provider| Widget::from(Stack::new([decoration(provider)]));
    let mut runtime = Runtime::new(scene(old)).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(new)).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());

    assert!(old_completion.complete(pix()));
    assert!(!runtime.frame_requested());
    let visible = pix();
    assert!(new_completion.complete(visible.clone()));
    assert!(runtime.frame_requested());
    assert_eq!(draws(&frame(&mut runtime))[0].0, visible.id().0);
}

#[test]
fn a_followed_decoration_stream_observes_its_new_sources_frame_but_not_progress() {
    let (provider, proxy) = pending();
    let mut runtime = Runtime::new(Stack::new([decoration(provider)]).into()).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    let (source, completion) = ImageStream::loading();

    assert!(proxy.follow(&source));
    assert!(runtime.frame_requested());
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
    assert!(completion.report_chunk(ImageChunkEvent::new(3, Some(8))));
    assert!(!runtime.frame_requested());

    let visible = pix();
    assert!(completion.complete(visible.clone()));
    assert!(runtime.frame_requested());
    assert_eq!(draws(&frame(&mut runtime))[0].0, visible.id().0);
}

#[test]
fn a_loaded_decoration_image_is_clipped_by_circles_and_rounded_boxes_only() {
    use incular_widgets::{BorderRadius, BoxShape};

    for (shape, radius, clips) in [
        (BoxShape::Circle, None, true),
        (BoxShape::Rectangle, Some(BorderRadius::circular(16.)), true),
        (BoxShape::Rectangle, None, false),
    ] {
        let (provider, completion) = pending();
        let mut decoration = BoxDecoration::new()
            .shape(shape)
            .image(DecorationImage::from_provider(provider));
        decoration.border_radius = radius;
        let child = DecoratedBox::new(SizedBox::shrink()).decoration(decoration);
        let mut runtime = Runtime::new(child.into()).unwrap();
        assert!(draws(&frame(&mut runtime)).is_empty());
        assert!(!runtime.frame_requested());
        assert!(completion.complete(pix()));
        assert!(runtime.frame_requested());
        let list = frame(&mut runtime);
        let mut open_clips = 0;
        let mut images = 0;
        for command in list.commands() {
            match command {
                PaintCommand::PushClip { .. }
                | PaintCommand::PushClipRRect { .. }
                | PaintCommand::PushClipOval { .. }
                | PaintCommand::PushClipPath { .. } => open_clips += 1,
                PaintCommand::PopClip => open_clips -= 1,
                PaintCommand::Image { .. } => {
                    images += 1;
                    assert_eq!(open_clips, usize::from(clips), "{shape:?} {radius:?}");
                }
                _ => {}
            }
        }
        assert_eq!((images, open_clips), (1, 0));
        assert!(!runtime.frame_requested());
    }
}

#[test]
fn a_synchronously_ready_decoration_does_not_request_another_frame() {
    let completed = pix();
    let provider = Deferred {
        stream: ImageStream::ready(completed.clone()),
        resolutions: Rc::default(),
    };
    let mut runtime = Runtime::new(Stack::new([decoration(provider)]).into()).unwrap();
    let list = frame(&mut runtime);
    assert_eq!(draws(&list).len(), 1);
    assert_eq!(draws(&list)[0].0, completed.id().0);
    assert!(!runtime.frame_requested());
}

#[test]
fn newly_loaded_decoration_images_clip_only_when_their_shape_requires_it() {
    use incular_widgets::{BorderRadius, BoxShape, Radius};
    for (shape, radius, expected) in [
        (BoxShape::Rectangle, None, vec!["image"]),
        (BoxShape::Circle, None, vec!["path", "image", "restore"]),
        (
            BoxShape::Rectangle,
            Some(BorderRadius::all(Radius::circular(16.))),
            vec!["path", "image", "restore"],
        ),
    ] {
        let (provider, completer) = pending();
        let mut decoration = BoxDecoration::new()
            .shape(shape)
            .image(DecorationImage::from_provider(provider));
        decoration.border_radius = radius;
        let widget =
            DecoratedBox::new(SizedBox::new().width(100.).height(100.)).decoration(decoration);
        let mut runtime = Runtime::new(Stack::new([widget]).into()).unwrap();
        assert!(draws(&frame(&mut runtime)).is_empty());
        assert!(!runtime.frame_requested());
        let completed = pix();
        assert!(completer.complete(completed.clone()));
        assert!(runtime.frame_requested());
        let list = frame(&mut runtime);
        let operations: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushClipRRect { .. } => Some("rounded"),
                PaintCommand::PushClipOval { .. } => Some("oval"),
                PaintCommand::PushClipPath { .. } => Some("path"),
                PaintCommand::PushClip { .. } => Some("rectangle"),
                PaintCommand::Image { image, .. } => {
                    assert_eq!(image.id(), completed.id());
                    Some("image")
                }
                PaintCommand::PopClip => Some("restore"),
                _ => None,
            })
            .collect();
        assert_eq!(operations, expected, "{shape:?} with {radius:?}");
    }
}

fn zero_size_decoration(provider: Deferred) -> Widget {
    DecoratedBox::new(SizedBox::new().width(0.).height(0.))
        .decoration(BoxDecoration::new().image(DecorationImage::from_provider(provider)))
        .into()
}

#[test]
fn a_ready_provider_does_not_notify_during_zero_size_decoration_paint() {
    let provider = Deferred {
        stream: ImageStream::ready(pix()),
        resolutions: Rc::default(),
    };
    let resolutions = provider.resolutions.clone();
    let mut runtime = Runtime::new(Stack::new([zero_size_decoration(provider)]).into()).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
    assert_eq!(resolutions.borrow().len(), 1);
    assert_eq!(resolutions.borrow()[0].size, Some(Size::ZERO));
}

#[test]
fn an_unfinished_zero_size_decoration_notifies_only_after_completion() {
    let (provider, completion) = pending();
    let resolutions = provider.resolutions.clone();
    let mut runtime = Runtime::new(Stack::new([zero_size_decoration(provider)]).into()).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
    assert_eq!(resolutions.borrow().len(), 1);
    assert_eq!(resolutions.borrow()[0].size, Some(Size::ZERO));
    assert!(completion.complete(pix()));
    assert!(runtime.frame_requested());
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
}

#[test]
fn repeating_a_clone_on_a_decoration_stream_does_not_repaint() {
    let (stream, publisher) = ImageStream::frame_sequence();
    let provider = Deferred {
        stream,
        resolutions: Rc::default(),
    };
    let mut runtime = Runtime::new(Stack::new([zero_size_decoration(provider)]).into()).unwrap();
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());

    let first = pix();
    assert!(publisher.publish(first.clone()));
    assert!(runtime.frame_requested());
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());

    assert!(!publisher.publish(first));
    assert!(!runtime.frame_requested());

    assert!(publisher.publish(pix()));
    assert!(runtime.frame_requested());
    assert!(draws(&frame(&mut runtime)).is_empty());
    assert!(!runtime.frame_requested());
}
