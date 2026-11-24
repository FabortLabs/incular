//! Public lifecycle and clipboard behavior against pinned Flutter 3.50.0.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use incular_animation::Ticker;
use incular_config::Constraints;
use incular_core::{Color, Size, time::Instant};
use incular_platform::{
    Clipboard, ClipboardCapabilities, ClipboardError, ClipboardWriteReport, DataTransfer,
    MemoryClipboard, PlatformEvent, PlatformLifecycle, TransferFormat, TransferReadRequest,
};
use incular_runtime::{ApplicationLifecycle, Runtime};
use incular_widgets::Widget;

const TEXT: &str = "Hello world";
const SIZE: Size = Size {
    width: 100.,
    height: 100.,
};

fn runtime() -> Runtime {
    Runtime::new(Widget::box_(SIZE, Color::WHITE)).unwrap()
}

// Flutter services/clipboard_test.dart:17. Seed the host clipboard rather
// than letting the convenience getter and setter hide a symmetric fault.
#[test]
fn clipboard_get_text_returns_host_plain_text() {
    let mut host = MemoryClipboard::default();
    host.write(DataTransfer::plain_text(TEXT)).unwrap();
    assert_eq!(host.get_text().as_deref(), Some(TEXT));
}

// Flutter services/clipboard_test.dart:26. The optional plain-text
// convenience, not the rich read API, corresponds to Flutter's nullable read.
#[test]
fn clipboard_get_text_returns_none_when_host_has_no_data() {
    let mut host = MemoryClipboard::default();
    assert_eq!(host.get_text(), None);
}

#[derive(Default)]
struct RecordingClipboard(Rc<RefCell<Vec<DataTransfer>>>);

impl Clipboard for RecordingClipboard {
    fn capabilities(&self) -> ClipboardCapabilities {
        MemoryClipboard::default().capabilities()
    }

    fn read(&mut self, _: TransferReadRequest) -> Result<DataTransfer, ClipboardError> {
        panic!("setting clipboard text must not read the host clipboard");
    }

    fn write(&mut self, data: DataTransfer) -> Result<ClipboardWriteReport, ClipboardError> {
        let formats = data.formats();
        self.0.borrow_mut().push(data);
        Ok(ClipboardWriteReport::all_written(formats))
    }
}

// Flutter services/clipboard_test.dart:46. Inspect exactly what reaches the
// injected platform boundary, not another call through the setter's wrapper.
#[test]
fn clipboard_set_text_sends_only_plain_text_to_host() {
    let mut runtime = runtime();
    let host = RecordingClipboard::default();
    let writes = host.0.clone();
    runtime.set_clipboard(Box::new(host));
    runtime.set_clipboard_text(TEXT);

    let writes = writes.borrow();
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].formats(), [TransferFormat::PlainText]);
    assert_eq!(writes[0].plain_text_value().unwrap().as_deref(), Some(TEXT));
}

fn set_lifecycle(runtime: &mut Runtime, state: PlatformLifecycle) {
    let _ = runtime.handle_platform_event(PlatformEvent::Lifecycle(state));
}

fn pump_if_requested(runtime: &mut Runtime, time: Instant) {
    if runtime.frame_requested() {
        runtime
            .run_frame_at(Constraints::tight(SIZE), time)
            .unwrap();
    }
}

// Flutter scheduler/ticker_test.dart:154.
#[test]
fn suspension_stops_ticking_without_deactivating_an_active_ticker() {
    let mut runtime = runtime();
    set_lifecycle(&mut runtime, PlatformLifecycle::Resumed);
    let ticks = Rc::new(Cell::new(0));
    let ticker = Ticker::new({
        let ticks = ticks.clone();
        move |_| ticks.set(ticks.get() + 1)
    });
    ticker.start();
    assert!(ticker.is_ticking());
    assert!(ticker.is_active());
    assert_eq!(ticks.get(), 0);

    set_lifecycle(&mut runtime, PlatformLifecycle::Suspended);
    assert_eq!(runtime.lifecycle(), ApplicationLifecycle::Suspended);
    assert!(
        ticker.is_active(),
        "suspension does not stop the ticker's run"
    );
    assert!(
        !ticker.is_ticking(),
        "suspension disables ticker frame delivery"
    );

    ticker.stop();
    set_lifecycle(&mut runtime, PlatformLifecycle::Resumed);
}

// Flutter scheduler/ticker_test.dart:178. The host delivers a frame only
// when the runtime requests one; this does not force-render while suspended.
#[test]
fn a_ticker_started_while_suspended_delivers_its_first_tick_after_resume() {
    let mut runtime = runtime();
    let base = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(SIZE), base)
        .unwrap();
    set_lifecycle(&mut runtime, PlatformLifecycle::Suspended);
    assert_eq!(runtime.lifecycle(), ApplicationLifecycle::Suspended);

    let ticks = Rc::new(Cell::new(0));
    let ticker = Ticker::new({
        let ticks = ticks.clone();
        move |_| ticks.set(ticks.get() + 1)
    });
    ticker.start();
    let before_frame = (ticker.is_ticking(), ticks.get());
    pump_if_requested(&mut runtime, base + Duration::from_millis(10));
    let during_suspend = (ticker.is_ticking(), ticks.get());
    set_lifecycle(&mut runtime, PlatformLifecycle::Resumed);
    assert_eq!(runtime.lifecycle(), ApplicationLifecycle::Active);
    pump_if_requested(&mut runtime, base + Duration::from_millis(20));
    let after_resume = (ticker.is_ticking(), ticks.get());
    ticker.stop();

    assert_eq!(
        [before_frame, during_suspend, after_resume],
        [(false, 0), (false, 0), (true, 1)]
    );
}
