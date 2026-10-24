//! Flutter's caret blink: `_kCursorBlinkHalfPeriod` on/off, or the iOS key
//! frames (`_KeyFrame.iOSBlinkingCaretKeyFrames`) repeated each second with
//! `cursorOpacityAnimates`.

use std::time::{Duration, Instant};

use incular_text::TextEditingController;

#[test]
fn blink_opacity_follows_flutter() {
    let controller = TextEditingController::with_text("a");
    let start = Instant::now();
    assert_eq!(controller.caret_opacity(start, false), 0.0, "never reset");
    controller.reset_caret(start);
    let at = |millis: u64, animates: bool| {
        controller.caret_opacity(start + Duration::from_millis(millis), animates)
    };
    // On for 500 ms, off for 500 ms.
    for (millis, opacity) in [(0, 1.0), (499, 1.0), (500, 0.0), (999, 0.0), (1_000, 1.0)] {
        assert_eq!(at(millis, false), opacity, "{millis} ms");
    }
    // Discrete iOS key frames, held until the next one.
    for (millis, opacity) in [
        (0, 1.0),
        (520, 1.0),
        (540, 0.75),
        (600, 0.5),
        (620, 0.25),
        (700, 0.0),
        (860, 0.0),
        (890, 0.25),
        (930, 0.5),
        (990, 0.75),
        // The simulation restarts at one second.
        (1_000, 1.0),
        (1_540, 0.75),
    ] {
        assert_eq!(at(millis, true), opacity, "{millis} ms");
    }
}
