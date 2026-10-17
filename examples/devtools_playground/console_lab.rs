//! Section 8: application log messages sent to the DevTools Console.
use super::theme::*;
use incular::controls_prelude::{Button, PrimaryButton};
use incular::devtools::{LogLevel, log};
use incular::prelude::*;

const TARGET: &str = "playground::console";

#[derive(Clone)]
pub struct ConsoleLab {
    pub sent: Signal<u32>,
}

impl ConsoleLab {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ConsoleLab {
    fn default() -> Self {
        Self {
            sent: Signal::new(0),
        }
    }
}

/// Logs `count` numbered messages of `level` and returns how many were sent.
pub fn emit(level: LogLevel, label: &str, count: u32) -> u32 {
    for index in 1..=count {
        log(level, TARGET, format!("{label} #{index}"));
    }
    count
}

pub fn view(lab: &ConsoleLab) -> Widget {
    let button = |label: &'static str, level: LogLevel, count: u32, primary: bool| {
        let sent = lab.sent.clone();
        let click = move || {
            let n = emit(level, label, count);
            sent.update(|total| *total += n);
        };
        if primary {
            Widget::from(PrimaryButton::new(label).on_click(click))
        } else {
            Widget::from(Button::new(label).on_click(click))
        }
    };
    let buttons = card(
        "Send log messages",
        "Each button calls incular::devtools::log; open the DevTools Console to see them.",
        520.,
        Column::new([
            Row::new([
                button("Log info", LogLevel::Info, 1, true),
                hgap(10.),
                button("Log warning", LogLevel::Warn, 1, false),
                hgap(10.),
                button("Log error", LogLevel::Error, 1, false),
                hgap(10.),
                button("Log burst (100)", LogLevel::Debug, 100, false),
            ])
            .into(),
            gap(12.),
            muted(format!("Sent {} messages", lab.sent.get())),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Start),
    );
    let about = card(
        "How it works",
        "Records go to a bounded queue (1024 entries, oldest dropped, 4 KiB per message), safe from any thread. The desktop host forwards them to DevTools; without the devtools feature they are discarded.",
        520.,
        body("Use the Console level filters to isolate errors, warnings or debug output."),
    );
    page(
        "Console",
        "Log application messages into the DevTools Console.",
        vec![buttons, about],
    )
}
