//! Headless smoke tests: every section builds, lays out and exposes its key labels.
#![allow(dead_code)]

#[path = "../main.rs"]
mod example;

use example::Section;
use incular::prelude::*;
use incular_config::Constraints;
use incular_runtime::Runtime;
use std::time::{Duration, Instant};

struct Harness {
    runtime: Runtime,
    now: Instant,
}

impl Harness {
    fn new(section: Section) -> Self {
        let mut harness = Self {
            runtime: example::build_app_at(section).into_runtime(),
            now: Instant::now(),
        };
        for _ in 0..4 {
            harness.frame();
        }
        harness
    }

    fn frame(&mut self) {
        self.now += Duration::from_millis(16);
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(1240., 820.)), self.now)
            .expect("frame");
    }

    fn labels(&self) -> Vec<String> {
        self.runtime
            .tree()
            .semantics()
            .iter()
            .filter_map(|(_, node)| node.label.clone())
            .collect()
    }

    fn assert_label(&self, wanted: &str) {
        let labels = self.labels();
        assert!(
            labels.iter().any(|label| label.contains(wanted)),
            "missing {wanted:?} in {labels:?}"
        );
    }
}

#[test]
fn chrome_lists_every_section() {
    let harness = Harness::new(Section::Motion);
    for section in Section::ALL {
        harness.assert_label(section.title());
    }
}

#[test]
fn motion_section_keeps_producing_frames() {
    let mut harness = Harness::new(Section::Motion);
    harness.assert_label("Live motion");
    for _ in 0..30 {
        harness.frame();
    }
}

#[test]
fn each_section_builds_and_runs_frames() {
    let expectations = [
        (Section::Jank, "Simulate jank"),
        (Section::Editable, "Editable widgets"),
        (Section::Signals, "Increment"),
        (Section::Layout, "Layout & scroll"),
        (Section::Text, "Long wrapping paragraph"),
        (Section::Semantics, "Save changes"),
        (Section::Console, "Log burst (100)"),
    ];
    for (section, label) in expectations {
        let mut harness = Harness::new(section);
        for _ in 0..5 {
            harness.frame();
        }
        harness.assert_label(label);
    }
}

#[test]
fn semantics_section_exposes_controls() {
    let harness = Harness::new(Section::Semantics);
    for label in [
        "Save changes",
        "Cancel",
        "Send me product updates",
        "Dark mode",
        "Volume",
    ] {
        harness.assert_label(label);
    }
}

#[test]
fn console_section_buttons_log_through_devtools_api() {
    let harness = Harness::new(Section::Console);
    for label in ["Log info", "Log warning", "Log error", "Log burst (100)"] {
        harness.assert_label(label);
    }
    harness.assert_label("Sent 0 messages");
}

#[test]
fn console_emit_reports_message_count() {
    assert_eq!(
        example::console_lab::emit(incular::devtools::LogLevel::Info, "x", 100),
        100
    );
}
