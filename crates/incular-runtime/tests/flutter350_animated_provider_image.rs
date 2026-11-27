use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use incular_config::{Constraints, RuntimeEnvironment};
use incular_core::{Size, time::Instant};
use incular_image::{AnimatedWebpDecoder, ImageDecodeCompletion, ImageFrameCodec, ImagePlayback};
use incular_rendering::PaintCommand;
use incular_runtime::Runtime;
use incular_widgets::{Image, MediaQuery, SizedBox, Stack, TickerMode, Widget};

const TWO_FRAMES: &[u8] = include_bytes!("../../incular-image/tests/fixtures/webp_two_cycles.webp");

struct Codec {
    decoder: RefCell<AnimatedWebpDecoder>,
    requests: Rc<Cell<usize>>,
}

impl ImageFrameCodec for Codec {
    fn frame_count(&self) -> NonZeroUsize {
        self.decoder.borrow().frame_count()
    }

    fn repetition_count(&self) -> Option<usize> {
        self.decoder.borrow().repetition_count()
    }

    fn decode_next(&self, callback: ImageDecodeCompletion) {
        self.requests.set(self.requests.get() + 1);
        let frame = self.decoder.borrow_mut().decode_next();
        callback(frame);
    }
}

type FrameLog = Rc<RefCell<Vec<(Option<u32>, bool)>>>;

fn build(source: &ImagePlayback, log: &FrameLog) -> Widget {
    let log = log.clone();
    Image::playback(source.clone())
        .frame_builder(move |_, widget, frame, synchronous| {
            log.borrow_mut().push((frame, synchronous));
            widget
        })
        .into()
}

fn playback() -> (ImagePlayback, Rc<Cell<usize>>) {
    let source = ImagePlayback::scheduled();
    let requests = Rc::new(Cell::new(0));
    assert!(
        source.attach(Rc::new(Codec {
            decoder: RefCell::new(
                AnimatedWebpDecoder::from_bytes(Arc::from(TWO_FRAMES))
                    .unwrap()
                    .unwrap(),
            ),
            requests: requests.clone(),
        }))
    );
    (source, requests)
}

fn pixels(runtime: &mut Runtime, at: Instant) -> Vec<[u8; 4]> {
    let (list, _) = runtime
        .run_frame_at(Constraints::loose(Size::new(100.0, 100.0)), at)
        .unwrap();
    list.commands()
        .iter()
        .filter_map(|command| match command {
            PaintCommand::Image { image, .. } => {
                Some(image.decoded().pixels()[0..4].try_into().unwrap())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn resolved_widget_numbers_frames_and_paints_the_real_decoded_images() {
    let (source, requests) = playback();
    let log = FrameLog::default();
    let mut runtime = Runtime::new(build(&source, &log)).unwrap();
    let t0 = Instant::now();
    assert!(pixels(&mut runtime, t0).is_empty());
    assert_eq!(*log.borrow(), [(None, false)]);
    assert_eq!(requests.get(), 1);

    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    assert_eq!(*log.borrow(), [(None, false), (Some(0), false)]);
    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    assert_eq!(log.borrow().len(), 2);
    assert_eq!(
        pixels(&mut runtime, t0 + Duration::from_millis(200)),
        [[0, 0, 255, 255]]
    );
    assert_eq!(
        *log.borrow(),
        [(None, false), (Some(0), false), (Some(1), false)]
    );
}

fn paused_widget(wrap: impl FnOnce(Widget) -> Widget) {
    let (source, requests) = playback();
    let log = FrameLog::default();
    let mut runtime = Runtime::new(wrap(build(&source, &log))).unwrap();
    let t0 = Instant::now();
    assert!(pixels(&mut runtime, t0).is_empty());
    assert_eq!(*log.borrow(), [(None, false)]);
    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    assert_eq!(*log.borrow(), [(None, false), (Some(0), false)]);
    for millis in [100, 200, 600, 800] {
        assert_eq!(
            pixels(&mut runtime, t0 + Duration::from_millis(millis)),
            [[255, 0, 0, 255]]
        );
        assert_eq!(*log.borrow(), [(None, false), (Some(0), false)]);
    }
    assert_eq!(requests.get(), 1);
    assert!(!runtime.frame_requested());
}

#[test]
fn reduced_motion_shows_the_first_frame_but_suppresses_later_frames() {
    paused_widget(|child| {
        MediaQuery::new(
            RuntimeEnvironment {
                reduced_motion: true,
                ..RuntimeEnvironment::default()
            },
            child,
        )
        .into()
    });
}

#[test]
fn disabled_ticker_mode_shows_the_first_frame_but_suppresses_later_frames() {
    paused_widget(|child| TickerMode::new(false, child).into());
}

#[test]
fn ticker_mode_pauses_the_widget_while_another_listener_keeps_the_stream_live() {
    let (source, _) = playback();
    let log = FrameLog::default();
    let scene = |enabled| TickerMode::new(enabled, build(&source, &log)).into();
    let mut runtime = Runtime::new(scene(true)).unwrap();
    let t0 = Instant::now();
    assert!(pixels(&mut runtime, t0).is_empty());
    let externally_seen = Rc::new(Cell::new(0usize));
    let seen = externally_seen.clone();
    let _other = source
        .listen(move |result, _| {
            assert!(result.is_ok());
            seen.set(seen.get() + 1);
        })
        .unwrap();
    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(false)).unwrap();
    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    assert_eq!(
        *log.borrow(),
        [(None, false), (Some(0), false), (Some(0), false)]
    );

    let t1 = t0 + Duration::from_millis(200);
    assert_eq!(pixels(&mut runtime, t1), [[255, 0, 0, 255]]);
    assert_eq!(log.borrow().len(), 3);

    let t2 = t0 + Duration::from_millis(600);
    assert_eq!(pixels(&mut runtime, t2), [[255, 0, 0, 255]]);
    assert_eq!(log.borrow().len(), 3);
    assert_eq!(externally_seen.get(), 3);

    runtime.schedule_update(root, scene(true)).unwrap();
    assert_eq!(pixels(&mut runtime, t2), [[255, 0, 0, 255]]);
    assert_eq!(log.borrow().len(), 4);
    assert_eq!(log.borrow()[3], (Some(1), true));
}

#[test]
fn reduced_motion_can_pause_and_resume_an_already_mounted_image() {
    let (source, _) = playback();
    let log = FrameLog::default();
    let scene = |paused| {
        MediaQuery::new(
            RuntimeEnvironment {
                reduced_motion: paused,
                ..RuntimeEnvironment::default()
            },
            build(&source, &log),
        )
        .into()
    };
    let mut runtime = Runtime::new(scene(false)).unwrap();
    let t0 = Instant::now();
    assert!(pixels(&mut runtime, t0).is_empty());
    let _other = source.listen(|_, _| {}).unwrap();
    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    assert_eq!(log.borrow().len(), 2);

    let at = |ms| t0 + Duration::from_millis(ms);
    assert_eq!(pixels(&mut runtime, at(200)), [[0, 0, 255, 255]]);
    assert_eq!(log.borrow()[2], (Some(1), false));

    let root = runtime.tree().root().unwrap();
    runtime.schedule_update(root, scene(true)).unwrap();
    assert_eq!(pixels(&mut runtime, at(200)), [[0, 0, 255, 255]]);
    assert_eq!(log.borrow()[3], (Some(1), false));
    assert_eq!(pixels(&mut runtime, at(600)), [[0, 0, 255, 255]]);
    assert_eq!(log.borrow().len(), 4);

    runtime.schedule_update(root, scene(false)).unwrap();
    assert_eq!(pixels(&mut runtime, at(600)), [[255, 0, 0, 255]]);
    assert_eq!(log.borrow()[4], (Some(2), true));
    assert_eq!(pixels(&mut runtime, at(800)), [[0, 0, 255, 255]]);
    assert_eq!(log.borrow()[5], (Some(3), true));
    assert_eq!(log.borrow().len(), 6);
}

fn drain_image_workers(runtime: &mut Runtime) {
    let timeout = Instant::now() + Duration::from_secs(2);
    loop {
        runtime.process_runtime_work();
        if runtime.runtime_diagnostics().active_tracked_tasks == 0 {
            return;
        }
        assert!(Instant::now() < timeout, "image worker did not complete");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn real_worker_codec_feeds_native_images_on_app_frames() {
    let log = FrameLog::default();
    let mut runtime = Runtime::new(Stack::new([] as [Widget; 0]).into()).unwrap();
    let source = runtime.spawner().load_animated_webp(Arc::from(TWO_FRAMES));
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, Stack::new([build(&source, &log)]).into())
        .unwrap();
    let t0 = Instant::now();
    assert!(pixels(&mut runtime, t0).is_empty());
    drain_image_workers(&mut runtime);
    assert_eq!(pixels(&mut runtime, t0), [[255, 0, 0, 255]]);
    drain_image_workers(&mut runtime);
    assert_eq!(
        pixels(&mut runtime, t0 + Duration::from_millis(199)),
        [[255, 0, 0, 255]]
    );
    assert_eq!(
        pixels(&mut runtime, t0 + Duration::from_millis(200)),
        [[0, 0, 255, 255]]
    );
}

#[test]
fn dropped_worker_source_is_not_kept_alive_by_codec_initialization() {
    let mut runtime = Runtime::new(Stack::new([] as [Widget; 0]).into()).unwrap();
    let source = runtime.spawner().load_animated_webp(Arc::from(TWO_FRAMES));
    let weak = source.downgrade();
    drop(source);
    assert!(weak.upgrade().is_none());
    drain_image_workers(&mut runtime);
    assert!(weak.upgrade().is_none());
}

#[test]
fn bad_encoded_source_notifies_the_mounted_error_builder() {
    let mut runtime = Runtime::new(Stack::new([] as [Widget; 0]).into()).unwrap();
    let source = runtime
        .spawner()
        .load_animated_webp(Arc::from(&b"bad input"[..]));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let captured = errors.clone();
    let widget = Image::playback(source).error_builder(move |_, error| {
        captured.borrow_mut().push(error.clone());
        Widget::from(SizedBox::new())
    });
    runtime
        .schedule_update(
            runtime.tree().root().unwrap(),
            Stack::new([Widget::from(widget)]).into(),
        )
        .unwrap();
    let now = Instant::now();
    assert!(pixels(&mut runtime, now).is_empty());
    drain_image_workers(&mut runtime);
    assert!(pixels(&mut runtime, now).is_empty());
    assert_eq!(errors.borrow().len(), 1);
}
