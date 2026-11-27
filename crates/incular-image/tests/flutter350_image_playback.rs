use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    num::NonZeroUsize,
    rc::Rc,
    time::{Duration, Instant},
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
