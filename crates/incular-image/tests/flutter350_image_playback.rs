use incular_core::time::Instant;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    num::NonZeroUsize,
    rc::Rc,
    time::Duration,
};

use incular_image::{
    DecodedImageFrame, ImageDecodeCompletion, ImageError, ImageFrameCodec, ImageFrameDemand,
    ImageHandle, ImageInfo, ImagePlayback,
};

#[derive(Default)]
struct Driver {
    asked: Cell<usize>,
    disposed: Cell<bool>,
    pending: RefCell<VecDeque<ImageDecodeCompletion>>,
}

struct Codec {
    driver: Rc<Driver>,
    count: usize,
    repeats: Option<usize>,
}

impl ImageFrameCodec for Codec {
    fn frame_count(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.count).unwrap()
    }
    fn repetition_count(&self) -> Option<usize> {
        self.repeats
    }
    fn decode_next(&self, complete: ImageDecodeCompletion) {
        self.driver.asked.set(self.driver.asked.get() + 1);
        self.driver.pending.borrow_mut().push_back(complete);
    }
}

impl Drop for Codec {
    fn drop(&mut self) {
        self.driver.disposed.set(true);
    }
}

impl Driver {
    fn attach(self: &Rc<Self>, stream: &ImagePlayback, count: usize, repeats: Option<usize>) {
        assert!(stream.attach(Rc::new(Codec {
            driver: self.clone(),
            count,
            repeats,
        })));
    }

    fn complete(&self, frame: &ImageInfo, milliseconds: u64) {
        let completion = self
            .pending
            .borrow_mut()
            .pop_front()
            .expect("queued decode");
        completion(Ok(DecodedImageFrame {
            image: frame.clone(),
            duration: Duration::from_millis(milliseconds),
        }));
    }
}

fn image(width: u32, height: u32) -> ImageInfo {
    ImageInfo::new(
        ImageHandle::from_rgba8(width, height, vec![0_u8; (width * height * 4) as usize]).unwrap(),
    )
}

type Delivered = Rc<RefCell<Vec<(Result<ImageInfo, ImageError>, bool)>>>;

fn recorder(delivered: &Delivered) -> impl Fn(Result<ImageInfo, ImageError>, bool) + 'static {
    let delivered = delivered.clone();
    move |result, sync| delivered.borrow_mut().push((result, sync))
}

fn assert_frame(delivered: &Delivered, index: usize, expected: &ImageInfo, synchronous: bool) {
    let delivered = delivered.borrow();
    assert!(delivered[index].0.as_ref().unwrap().is_clone_of(expected));
    assert_eq!(delivered[index].1, synchronous);
}

#[test]
fn first_listener_starts_an_already_attached_codec() {
    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::new(|| {});
    codec.attach(&stream, 1, Some(0));
    assert_eq!(codec.asked.get(), 0);
    let _listener = stream.listen(|_, _| {}).unwrap();
    assert_eq!(codec.asked.get(), 1);
    assert!(!codec.disposed.get());
}

#[test]
fn codec_attachment_starts_already_listening_stream() {
    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::new(|| {});
    let _listener = stream.listen(|_, _| {}).unwrap();
    assert_eq!(codec.asked.get(), 0);
    codec.attach(&stream, 1, Some(0));
    assert_eq!(codec.asked.get(), 1);
    assert!(!codec.disposed.get());
}

#[test]
fn finite_playback_presented_at_app_frames_and_stops_after_one_cycle() {
    let codec = Rc::new(Driver::default());
    let delivered = Delivered::default();
    let wakes = Rc::new(Cell::new(0));
    let wake_count = wakes.clone();
    let stream = ImagePlayback::new(move || wake_count.set(wake_count.get() + 1));
    let _listener = stream.listen(recorder(&delivered)).unwrap();
    codec.attach(&stream, 2, Some(0));
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();
    codec.complete(&frame1, 200);
    assert!(delivered.borrow().is_empty());
    assert_eq!(stream.frame_demand(), Some(ImageFrameDemand::NextFrame));
    assert_eq!(wakes.get(), 1);
    assert!(stream.advance(t0));
    assert_frame(&delivered, 0, &frame1, false);
    codec.complete(&frame2, 400);
    let t1 = t0 + Duration::from_millis(200);
    assert_eq!(stream.frame_demand(), Some(ImageFrameDemand::At(t1)));
    assert!(!stream.advance(t1 - Duration::from_millis(1)));
    assert!(!codec.disposed.get());
    assert!(stream.advance(t1));
    assert_frame(&delivered, 1, &frame2, false);
    assert!(codec.disposed.get());
    assert_eq!(codec.asked.get(), 2);
    assert!(!stream.advance(t1 + Duration::from_millis(400)));
    assert_eq!(delivered.borrow().len(), 2);
    assert!(codec.pending.borrow().is_empty());
}

#[test]
fn kept_alive_playback_stops_without_listeners_and_restarts_on_listen() {
    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::new(|| {});
    let delivered = Delivered::default();
    let subscription = stream.listen(recorder(&delivered)).unwrap();
    let keep_alive = stream.keep_alive().unwrap();
    codec.attach(&stream, 2, None);
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();
    codec.complete(&frame1, 200);
    assert!(stream.advance(t0));
    drop(subscription);
    codec.complete(&frame2, 400);
    assert!(!stream.advance(t0 + Duration::from_millis(400)));
    assert_eq!(codec.asked.get(), 2);
    let subscription = stream.listen(recorder(&delivered)).unwrap();
    assert_eq!(codec.asked.get(), 3);
    assert_eq!(delivered.borrow().len(), 2);
    assert_frame(&delivered, 1, &frame1, true);
    drop(keep_alive);
    assert!(!codec.disposed.get());
    drop(subscription);
    assert!(codec.disposed.get());
    // A completion can arrive even after the decoder and listener were retired.
    codec.complete(&frame2, 400);
    assert_eq!(stream.frame_demand(), None);
}

#[test]
fn readding_a_listener_does_not_present_the_discarded_scheduled_frame() {
    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::new(|| {});
    let delivered = Delivered::default();
    let subscription = stream.listen(recorder(&delivered)).unwrap();
    let handle = stream.keep_alive().unwrap();
    codec.attach(&stream, 2, None);
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();
    codec.complete(&frame1, 200);
    assert_eq!(codec.asked.get(), 1);
    drop(subscription);
    let subscription = stream.listen(recorder(&delivered)).unwrap();
    assert_eq!(codec.asked.get(), 2);
    assert!(!stream.advance(t0));
    assert!(delivered.borrow().is_empty());
    codec.complete(&frame2, 200);
    assert!(stream.advance(t0 + Duration::from_millis(16)));
    assert_eq!(delivered.borrow().len(), 1);
    assert_frame(&delivered, 0, &frame2, false);
    drop(subscription);
    drop(handle);
    assert!(codec.disposed.get());
}

#[test]
fn disposing_before_a_decode_completes_does_not_publish_a_frame() {
    let codec = Rc::new(Driver::default());
    let delivered = Delivered::default();
    let stream = ImagePlayback::new(|| panic!("a retired decode must not request a frame"));
    codec.attach(&stream, 1, Some(0));
    assert_eq!(codec.asked.get(), 0);
    let subscription = stream.listen(recorder(&delivered)).unwrap();
    assert_eq!(codec.asked.get(), 1);
    assert!(!codec.disposed.get());
    drop(subscription);
    assert!(codec.disposed.get());
    codec.complete(&image(20, 10), 200);
    assert!(delivered.borrow().is_empty());
    assert!(stream.listen(|_, _| unreachable!()).is_none());
}

#[test]
fn a_static_decode_delivers_once_without_waiting_for_an_app_frame() {
    let codec = Rc::new(Driver::default());
    let delivered = Delivered::default();
    let stream = ImagePlayback::new(|| panic!("a still image must not request an animation frame"));
    let observer = recorder(&delivered);
    let driver = codec.clone();
    let first = stream
        .listen(move |result, sync| {
            assert!(!driver.disposed.get());
            observer(result, sync);
        })
        .unwrap();
    codec.attach(&stream, 1, Some(0));
    assert!(!codec.disposed.get());
    let expected = image(20, 10);
    codec.complete(&expected, 200);
    assert!(codec.disposed.get());
    assert_eq!(codec.asked.get(), 1);
    assert_frame(&delivered, 0, &expected, false);
    assert_eq!(stream.frame_demand(), None);
    let _late_listener = stream.listen(recorder(&delivered)).unwrap();
    assert_frame(&delivered, 1, &expected, true);
    drop(first);
}

fn app_frame(at: Instant) {
    incular_animation::scheduler::begin_frame(at);
    incular_animation::scheduler::end_frame(at);
}

fn milliseconds(base: Instant, ms: u64) -> Instant {
    base + Duration::from_millis(ms)
}

#[test]
fn scheduled_animation_publishes_only_after_the_presented_frames_duration() {
    use incular_animation::scheduler::{has_scheduled_frame, next_scheduled_frame_deadline};

    let codec = Rc::new(Driver::default());
    let delivered = Delivered::default();
    let stream = ImagePlayback::scheduled();
    let sub = stream.listen(recorder(&delivered)).unwrap();
    codec.attach(&stream, 2, None);
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();

    codec.complete(&frame1, 200);
    assert!(delivered.borrow().is_empty());
    assert!(has_scheduled_frame());
    app_frame(t0);
    assert_eq!(delivered.borrow().len(), 1);
    assert_frame(&delivered, 0, &frame1, false);
    codec.complete(&frame2, 400);
    app_frame(milliseconds(t0, 100));
    assert_eq!(delivered.borrow().len(), 1);
    assert!(!has_scheduled_frame());
    assert_eq!(next_scheduled_frame_deadline(), Some(milliseconds(t0, 200)));
    app_frame(milliseconds(t0, 199));
    assert_eq!(delivered.borrow().len(), 1);
    app_frame(milliseconds(t0, 200));
    assert_eq!(delivered.borrow().len(), 2);
    assert_frame(&delivered, 1, &frame2, false);
    app_frame(milliseconds(t0, 600));
    assert!(!codec.disposed.get());
    drop(sub);
    assert!(codec.disposed.get());
}

#[test]
fn scheduled_animation_wraps_back_and_keeps_order_with_a_slow_host() {
    let codec = Rc::new(Driver::default());
    let delivered = Delivered::default();
    let stream = ImagePlayback::scheduled();
    let sub = stream.listen(recorder(&delivered)).unwrap();
    codec.attach(&stream, 2, None);
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();

    codec.complete(&frame1, 200);
    app_frame(t0);
    codec.complete(&frame2, 400);
    app_frame(milliseconds(t0, 200));
    codec.complete(&frame1, 200);
    app_frame(milliseconds(t0, 600));
    assert_eq!(delivered.borrow().len(), 3);
    assert_frame(&delivered, 0, &frame1, false);
    assert_frame(&delivered, 1, &frame2, false);
    assert_frame(&delivered, 2, &frame1, false);
    app_frame(milliseconds(t0, 800));
    assert!(!codec.disposed.get());
    drop(sub);
    assert!(codec.disposed.get());
}

#[test]
fn scheduled_animation_removing_one_listener_does_not_cancel_the_other() {
    use incular_animation::scheduler::next_scheduled_frame_deadline;

    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::scheduled();
    let (a, b) = (Delivered::default(), Delivered::default());
    let sub_a = stream.listen(recorder(&a)).unwrap();
    let sub_b = stream.listen(recorder(&b)).unwrap();
    codec.attach(&stream, 2, None);
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();

    codec.complete(&frame1, 200);
    app_frame(t0);
    assert_frame(&a, 0, &frame1, false);
    assert_frame(&b, 0, &frame1, false);
    codec.complete(&frame2, 400);
    app_frame(t0);
    drop(sub_a);
    assert_eq!(next_scheduled_frame_deadline(), Some(milliseconds(t0, 200)));
    app_frame(milliseconds(t0, 400));
    assert_eq!(a.borrow().len(), 1);
    assert_eq!(b.borrow().len(), 2);
    assert_frame(&b, 1, &frame2, false);
    assert!(!codec.disposed.get());
    drop(sub_b);
    assert!(codec.disposed.get());
}

#[test]
fn scheduled_animation_drops_the_last_timer_with_the_last_listener() {
    use incular_animation::scheduler::{has_scheduled_frame, next_scheduled_frame_deadline};

    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::scheduled();
    let delivered = Delivered::default();
    let sub = stream.listen(recorder(&delivered)).unwrap();
    codec.attach(&stream, 2, None);
    let t0 = Instant::now();
    codec.complete(&image(20, 10), 200);
    app_frame(t0);
    codec.complete(&image(200, 100), 400);
    app_frame(t0);
    assert_eq!(next_scheduled_frame_deadline(), Some(milliseconds(t0, 200)));
    assert!(!codec.disposed.get());
    drop(sub);
    assert!(codec.disposed.get());
    assert_eq!(next_scheduled_frame_deadline(), None);
    assert!(!has_scheduled_frame());
    app_frame(milliseconds(t0, 400));
    assert_eq!(delivered.borrow().len(), 1);
}

#[test]
fn scheduled_animation_dilates_timer_delays() {
    use incular_animation::scheduler::{next_scheduled_frame_deadline, set_time_dilation};

    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            set_time_dilation(1.0);
        }
    }
    let _reset = Reset;
    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::scheduled();
    let delivered = Delivered::default();
    let sub = stream.listen(recorder(&delivered)).unwrap();
    codec.attach(&stream, 2, None);
    let t0 = Instant::now();
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    codec.complete(&frame1, 200);
    app_frame(t0);

    set_time_dilation(2.0);
    codec.complete(&frame2, 400);
    app_frame(t0);
    assert_eq!(next_scheduled_frame_deadline(), Some(milliseconds(t0, 400)));
    app_frame(milliseconds(t0, 200));
    assert_eq!(delivered.borrow().len(), 1);
    assert_eq!(codec.asked.get(), 2);
    app_frame(milliseconds(t0, 400));
    assert_eq!(codec.asked.get(), 3);
    assert_frame(&delivered, 1, &frame2, false);
    assert!(!codec.disposed.get());
    drop(sub);
    assert!(codec.disposed.get());
}

#[test]
fn scheduled_finite_animation_does_not_request_another_cycle() {
    use incular_animation::scheduler::{has_scheduled_frame, next_scheduled_frame_deadline};

    let codec = Rc::new(Driver::default());
    let delivered = Delivered::default();
    let stream = ImagePlayback::scheduled();
    let _sub = stream.listen(recorder(&delivered)).unwrap();
    codec.attach(&stream, 2, Some(0));
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();

    codec.complete(&frame1, 200);
    app_frame(t0);
    codec.complete(&frame2, 400);
    assert!(!codec.disposed.get());
    app_frame(milliseconds(t0, 200));
    assert!(codec.disposed.get());
    assert_eq!(codec.asked.get(), 2);
    assert!(!has_scheduled_frame());
    assert_eq!(next_scheduled_frame_deadline(), None);
    app_frame(milliseconds(t0, 600));
    assert_eq!(delivered.borrow().len(), 2);
    assert_frame(&delivered, 0, &frame1, false);
    assert_frame(&delivered, 1, &frame2, false);
}

#[test]
fn scheduled_idle_keepalive_does_not_request_work_until_a_listener_returns() {
    use incular_animation::scheduler::{has_scheduled_frame, next_scheduled_frame_deadline};

    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::scheduled();
    let delivered = Delivered::default();
    let listener = stream.listen(recorder(&delivered)).unwrap();
    let keep_alive = stream.keep_alive().unwrap();
    codec.attach(&stream, 2, None);
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();

    codec.complete(&frame1, 200);
    app_frame(t0);
    drop(listener);
    codec.complete(&frame2, 400);
    assert!(!has_scheduled_frame());
    assert_eq!(next_scheduled_frame_deadline(), None);
    app_frame(milliseconds(t0, 400));
    assert_eq!(codec.asked.get(), 2);
    let relisten = stream.listen(recorder(&delivered)).unwrap();
    assert_eq!(codec.asked.get(), 3);
    assert_frame(&delivered, 1, &frame1, true);
    drop(keep_alive);
    assert!(!codec.disposed.get());
    drop(relisten);
    assert!(codec.disposed.get());
}

#[test]
fn scheduled_pending_callback_never_presents_a_discarded_frame() {
    let codec = Rc::new(Driver::default());
    let stream = ImagePlayback::scheduled();
    let delivered = Delivered::default();
    let listener = stream.listen(recorder(&delivered)).unwrap();
    let keep_alive = stream.keep_alive().unwrap();
    codec.attach(&stream, 2, None);
    let frame1 = image(20, 10);
    let frame2 = image(200, 100);
    let t0 = Instant::now();

    codec.complete(&frame1, 200);
    assert_eq!(codec.asked.get(), 1);
    drop(listener);
    let relisten = stream.listen(recorder(&delivered)).unwrap();
    assert_eq!(codec.asked.get(), 2);
    app_frame(t0);
    assert!(delivered.borrow().is_empty());
    codec.complete(&frame2, 200);
    app_frame(milliseconds(t0, 16));
    assert_eq!(delivered.borrow().len(), 1);
    assert_frame(&delivered, 0, &frame2, false);
    drop(keep_alive);
    drop(relisten);
    assert!(codec.disposed.get());
}

#[test]
fn deferred_attachment_failure_reaches_active_and_late_observers() {
    let first = Delivered::default();
    let late = Delivered::default();
    let source = ImagePlayback::scheduled();
    let _first = source.listen(recorder(&first)).unwrap();
    let error = ImageError::Decode("bad codec header".into());
    assert!(source.attach_error(error.clone()));
    assert!(!source.attach_error(ImageError::Decode("unexpected second result".into())));
    assert_eq!(first.borrow().len(), 1);
    assert_error(&first, 0, &error, false);
    let _late = source.listen(recorder(&late)).unwrap();
    assert_eq!(late.borrow().len(), 1);
    assert_error(&late, 0, &error, true);
    assert_eq!(source.frame_demand(), None);
}

#[test]
fn decode_error_retains_the_last_good_frame_and_does_not_retry_on_listen() {
    let first = Delivered::default();
    let late = Delivered::default();
    let source = ImagePlayback::scheduled();
    let codec = Rc::new(Driver::default());
    let listener = source.listen(recorder(&first)).unwrap();
    codec.attach(&source, 2, None);
    let expected = image(20, 10);
    codec.complete(&expected, 200);
    app_frame(Instant::now());
    let complete = codec.pending.borrow_mut().pop_front().unwrap();
    let error = ImageError::Decode("corrupt second frame".into());
    complete(Err(error.clone()));

    assert_frame(&first, 0, &expected, false);
    assert_error(&first, 1, &error, false);
    assert!(source.frame_demand().is_none());
    assert!(!codec.disposed.get());
    let late_listener = source.listen(recorder(&late)).unwrap();
    assert_frame(&late, 0, &expected, true);
    assert_error(&late, 1, &error, true);
    assert_eq!(codec.asked.get(), 2);
    drop(listener);
    drop(late_listener);
    assert!(codec.disposed.get());
}

#[test]
fn an_initialization_callback_can_hold_a_weak_playback_only() {
    let source = ImagePlayback::scheduled();
    let weak = source.downgrade();
    assert_eq!(weak.upgrade().as_ref(), Some(&source));
    drop(source);
    assert!(weak.upgrade().is_none());
}

fn assert_error(delivered: &Delivered, i: usize, expected: &ImageError, synchronous: bool) {
    let delivered = delivered.borrow();
    assert!(matches!(&delivered[i].0, Err(error) if error == expected));
    assert_eq!(delivered[i].1, synchronous);
}

#[test]
fn pending_animated_codec_delivers_both_original_progress_events() {
    use incular_animation::scheduler::has_scheduled_frame;
    use incular_image::{ImageChunkEvent, ImageStreamListener};

    let source = ImagePlayback::scheduled();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let chunks = seen.clone();
    let _listener = source
        .listen_group(
            ImageStreamListener::new(Rc::new(|_, _| panic!("codec is still pending")))
                .on_chunk(Rc::new(move |event| chunks.borrow_mut().push(event))),
        )
        .unwrap();
    let first = ImageChunkEvent::new(1, Some(3));
    let second = ImageChunkEvent::new(2, Some(3));
    assert!(source.report_chunk(first));
    assert!(source.report_chunk(second));
    assert_eq!(*seen.borrow(), [first, second]);
    assert!(!has_scheduled_frame());
}

#[test]
fn animated_download_chunks_are_not_replayed_for_a_new_listener() {
    use incular_image::{ImageChunkEvent, ImageStreamListener};

    let source = ImagePlayback::scheduled();
    let first = ImageChunkEvent::new(1, Some(3));
    let second = ImageChunkEvent::new(2, Some(3));
    assert!(source.report_chunk(first));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let chunks = seen.clone();
    let listener = source
        .listen_group(
            ImageStreamListener::new(Rc::new(|_, _| panic!("codec is still pending")))
                .on_chunk(Rc::new(move |event| chunks.borrow_mut().push(event))),
        )
        .unwrap();
    assert!(seen.borrow().is_empty());
    assert!(source.report_chunk(second));
    assert_eq!(*seen.borrow(), [second]);
    drop(listener);
    assert!(!source.report_chunk(first));
}

#[test]
fn a_chunk_transport_error_does_not_block_later_download_progress() {
    use incular_animation::scheduler::has_scheduled_frame;
    use incular_image::{ImageChunkEvent, ImageStreamListener};

    let source = ImagePlayback::scheduled();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let chunks = seen.clone();
    let errors = seen.clone();
    let _listener = source
        .listen_group(
            ImageStreamListener::new(Rc::new(|_, _| panic!("codec is still pending")))
                .on_chunk(Rc::new(move |event| chunks.borrow_mut().push(Ok(event))))
                .on_error(Rc::new(move |error| errors.borrow_mut().push(Err(error)))),
        )
        .unwrap();
    let error = ImageError::Io("download disconnected".into());
    let chunk = ImageChunkEvent::new(2, Some(3));
    assert!(source.report_transport_error(error.clone()));
    assert!(source.report_chunk(chunk));
    assert_eq!(*seen.borrow(), [Err(error), Ok(chunk)]);
    assert!(!has_scheduled_frame());
}
