use std::{cell::RefCell, rc::Rc};

use incular_image::{
    ImageChunkEvent, ImageChunkSubscription, ImageHandle, ImageLoadState, ImageStream,
};

#[test]
fn live_chunk_listener_receives_both_original_progress_packets_in_order() {
    let (stream, let_load) = ImageStream::loading();
    let events = Rc::new(RefCell::new(Vec::new()));
    let _listener = stream.listen_chunks({
        let events = events.clone();
        move |event| events.borrow_mut().push(event)
    });
    let packets = [
        ImageChunkEvent::new(1, Some(3)),
        ImageChunkEvent::new(2, Some(3)),
    ];
    for packet in packets {
        assert!(let_load.report_chunk(packet));
    }
    assert_eq!(*events.borrow(), packets);
    assert_eq!(stream.state(), ImageLoadState::Loading(Some(packets[1])));
}

#[test]
fn registration_does_not_replay_the_existing_snapshot() {
    let (stream, loader) = ImageStream::loading();
    let first = ImageChunkEvent::new(1, Some(3));
    let second = ImageChunkEvent::new(2, Some(3));
    assert!(loader.report_chunk(first));

    let events = Rc::new(RefCell::new(Vec::new()));
    let listener = stream.listen_chunks({
        let events = events.clone();
        move |event| events.borrow_mut().push(event)
    });
    assert!(events.borrow().is_empty());
    assert_eq!(stream.state(), ImageLoadState::Loading(Some(first)));
    assert!(loader.report_chunk(second));
    assert_eq!(*events.borrow(), [second]);
    drop(listener);
    assert!(loader.report_chunk(ImageChunkEvent::new(3, Some(3))));
    assert_eq!(*events.borrow(), [second]);
}

#[test]
fn a_deferred_handoff_forwards_only_live_chunks_without_waking_the_frame_listener() {
    let (outer, handoff) = ImageStream::loading();
    let stream = outer.map_image(Ok);
    let (source, loader) = ImageStream::loading();
    let old = ImageChunkEvent::new(1, Some(9));
    let repeated = ImageChunkEvent::new(2, Some(9));
    assert!(loader.report_chunk(old));
    let events = Rc::new(RefCell::new(Vec::new()));
    let progress = stream.listen_chunks({
        let events = events.clone();
        move |event| events.borrow_mut().push(event)
    });
    let frames = Rc::new(RefCell::new(Vec::new()));
    let _frames = stream.listen_frame({
        let frames = frames.clone();
        move |result, synchronous| frames.borrow_mut().push((result.unwrap(), synchronous))
    });

    assert!(handoff.follow(&source));
    assert!(events.borrow().is_empty());
    assert_eq!(stream.state(), ImageLoadState::Loading(Some(old)));
    assert!(loader.report_chunk(repeated));
    assert!(loader.report_chunk(repeated));
    assert_eq!(*events.borrow(), [repeated, repeated]);
    assert!(frames.borrow().is_empty());
    drop(progress);
    assert!(loader.report_chunk(ImageChunkEvent::new(3, Some(9))));
    assert_eq!(*events.borrow(), [repeated, repeated]);

    let image = ImageHandle::from_rgba8(1, 1, vec![255; 4]).unwrap();
    assert!(loader.complete(image.clone()));
    assert_eq!(*frames.borrow(), [(image, false)]);
    assert!(!loader.report_chunk(ImageChunkEvent::new(4, Some(9))));
}

#[test]
fn removal_during_a_callback_preserves_only_the_current_delivery_snapshot() {
    let (source, loader) = ImageStream::loading();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let removed_slot = Rc::new(RefCell::new(None::<ImageChunkSubscription>));
    let _first = source.listen_chunks({
        let trace = trace.clone();
        let removed_slot = removed_slot.clone();
        move |_| {
            trace.borrow_mut().push("first");
            drop(removed_slot.borrow_mut().take());
        }
    });
    let second = source.listen_chunks({
        let trace = trace.clone();
        move |_| trace.borrow_mut().push("second")
    });
    *removed_slot.borrow_mut() = Some(second);
    assert!(loader.report_chunk(ImageChunkEvent::new(1, Some(9))));
    assert!(loader.report_chunk(ImageChunkEvent::new(2, Some(9))));
    assert_eq!(*trace.borrow(), ["first", "second", "first"]);
}

#[test]
fn dropping_a_callback_may_drop_another_subscription_to_the_same_stream() {
    let (stream, loader) = ImageStream::loading();
    let nested =
        stream.listen_chunks(|_| panic!("nested listener should be removed with its owner"));
    let outer = stream.listen_chunks(move |_| {
        std::hint::black_box(&nested);
    });
    drop(outer);
    assert!(loader.report_chunk(ImageChunkEvent::new(1, None)));
}
