use std::{
    cell::RefCell,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
};

use incular_image::{
    ImageChunkEvent, ImageChunkListener, ImageError, ImageErrorListener, ImageHandle,
    ImageListener, ImageStream, ImageStreamListener,
};

fn digest(listener: &ImageStreamListener) -> u64 {
    let mut hash = DefaultHasher::new();
    listener.hash(&mut hash);
    hash.finish()
}

#[test]
fn listener_equality_and_hash_track_all_three_shared_callbacks() {
    let image: ImageListener = Rc::new(|_, _| {});
    let other_image: ImageListener = Rc::new(|_, _| {});
    let chunk: ImageChunkListener = Rc::new(|_| {});
    let error: ImageErrorListener = Rc::new(|_| {});
    let basic = || ImageStreamListener::new(image.clone());
    let progress = || basic().on_chunk(chunk.clone());
    let all = || progress().on_error(error.clone());
    let errors_only = || basic().on_error(error.clone());
    for (a, b, equal) in [
        (basic(), basic(), true),
        (basic(), ImageStreamListener::new(other_image), false),
        (progress(), progress(), true),
        (all(), all(), true),
        (progress(), basic(), false),
        (all(), basic(), false),
        (all(), progress(), false),
        (all(), errors_only(), false),
    ] {
        assert_eq!(a == b, equal);
        assert_eq!(digest(&a) == digest(&b), equal);
    }
}

#[test]
fn a_compound_listener_observes_progress_then_its_error_and_cancels_both_on_drop() {
    let (stream, loader) = ImageStream::loading();
    let chunks = Rc::new(RefCell::new(Vec::new()));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let descriptor = ImageStreamListener::new(Rc::new(|_, _| panic!("not an image")))
        .on_chunk(Rc::new({
            let chunks = chunks.clone();
            move |event| chunks.borrow_mut().push(event)
        }))
        .on_error(Rc::new({
            let errors = errors.clone();
            move |error| errors.borrow_mut().push(error)
        }));
    let _listener = stream.listen(descriptor.clone());
    let packet = ImageChunkEvent::new(4, Some(9));
    assert!(loader.report_chunk(packet));
    assert!(loader.fail(ImageError::InvalidDimensions));
    assert_eq!(*chunks.borrow(), [packet]);
    assert_eq!(*errors.borrow(), [ImageError::InvalidDimensions]);

    let (later, loader) = ImageStream::loading();
    drop(later.listen(descriptor));
    assert!(loader.report_chunk(packet));
    assert!(loader.fail(ImageError::InvalidDimensions));
    assert_eq!(*chunks.borrow(), [packet]);
    assert_eq!(*errors.borrow(), [ImageError::InvalidDimensions]);
}

#[test]
fn ready_group_receives_image_info_with_the_same_pixels_synchronously() {
    let image = ImageHandle::from_rgba8(2, 1, vec![255; 8]).unwrap();
    let id = image.id();
    let stream = ImageStream::ready(image.clone());
    let observed = Rc::new(RefCell::new(None));
    let _listener = stream.listen(ImageStreamListener::new(Rc::new({
        let observed = observed.clone();
        move |info, synchronous| {
            observed.borrow_mut().replace((info, synchronous));
        }
    })));
    let (info, synchronous) = observed.take().unwrap();
    assert!(synchronous);
    assert_eq!(info.image().id(), id);
    assert_eq!(info.scale(), image.scale());
    assert_eq!(info.size_bytes(), 8);
}

#[test]
fn a_successful_frame_dispatches_the_snapshot_even_after_removal() {
    for handoff in [false, true] {
        for remove_before in [false, true] {
            let (stream, loader) = ImageStream::loading();
            let (source, source_loader) = ImageStream::loading();
            let events = Rc::new(RefCell::new(Vec::new()));
            let second = Rc::new(RefCell::new(None));
            let _first = stream.listen(ImageStreamListener::new(Rc::new({
                let events = events.clone();
                let second = second.clone();
                move |_, synchronous| {
                    events.borrow_mut().push(("A", synchronous));
                    let token = second.borrow_mut().take();
                    drop(token);
                }
            })));
            *second.borrow_mut() = Some(stream.listen(ImageStreamListener::new(Rc::new({
                let events = events.clone();
                move |_, synchronous| events.borrow_mut().push(("B", synchronous))
            }))));
            if handoff {
                assert!(loader.follow(&source));
            }
            if remove_before {
                let token = second.borrow_mut().take();
                drop(token);
            }
            let image = ImageHandle::from_rgba8(1, 1, vec![0x11, 0x22, 0x33, 0xff]).unwrap();
            assert!(if handoff {
                source_loader.complete(image)
            } else {
                loader.complete(image)
            });
            let want = if remove_before {
                vec![("A", false)]
            } else {
                vec![("A", false), ("B", false)]
            };
            assert_eq!(*events.borrow(), want, "handoff: {handoff}");
        }
    }
}
