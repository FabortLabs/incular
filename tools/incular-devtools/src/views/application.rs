//! Application: target identity, protocol and native windows.

use super::clipboard::Clipboard;
use super::icons::IconName;
use super::shared::{
    Variant, badge, button, column, divider, empty_state, gap, glyph, key_value, line_text,
    section_with, status_badge, surface_raised, wide_row,
};
use super::{primary, success, text_muted, text_primary};
use crate::{inspector::ConnectionState, transport::ClientBridge};
use incular::prelude::*;
use incular_devtools_protocol::{FrameRecordEvent, RequestMethod, TargetInfo, WindowSummary};

/// Plain-text description of the target for bug reports.
pub(crate) fn target_text(info: Option<&TargetInfo>, windows: &[WindowSummary]) -> String {
    let Some(info) = info else {
        return "No connected target".to_owned();
    };
    let mut lines = vec![
        format!("Executable: {}", info.executable),
        format!("PID: {}", info.pid),
        format!("Platform: {}", info.platform),
        format!("Framework: Incular {}", info.framework_version),
        format!("DevTools protocol: {}", info.protocol_version),
    ];
    for window in windows {
        lines.push(format!(
            "Window: {} · {:.0}×{:.0} @ {:.2}×",
            window.title, window.logical_size[0], window.logical_size[1], window.scale_factor
        ));
    }
    lines.join("\n")
}

pub(crate) fn build_application(
    target_info: Option<TargetInfo>,
    connection: ConnectionState,
    windows: Vec<WindowSummary>,
    frames: Vec<FrameRecordEvent>,
    bridge: ClientBridge,
    clipboard: Clipboard,
) -> Widget {
    let connected = connection == ConnectionState::Connected;
    let copy_text = target_text(target_info.as_ref(), &windows);
    let actions = wide_row([
        button(
            if connected {
                "Refresh target info"
            } else {
                "Retry connection"
            },
            Some(IconName::Refresh),
            Variant::Secondary,
            true,
            move || {
                if connected {
                    bridge.send_or_report(RequestMethod::GetTargetInfo);
                } else {
                    bridge.retry();
                }
            },
        ),
        gap(8., 1.),
        button(
            "Copy target info",
            Some(IconName::Copy),
            Variant::Secondary,
            target_info.is_some(),
            move || clipboard.copy(copy_text.clone(), "target information"),
        ),
    ]);
    let mut identity = Vec::new();
    if let Some(info) = target_info {
        identity.extend([
            key_value("Framework", format!("Incular {}", info.framework_version)),
            key_value("Process", format!("PID {}", info.pid)),
            key_value("Executable", info.executable),
            key_value("Platform", info.platform),
            key_value("Protocol", format!("v{}", info.protocol_version)),
            gap(1., 12.),
        ]);
    } else {
        identity.push(empty_state(
            IconName::Plug,
            "No target",
            "Start a DevTools-enabled application, then retry the connection.",
        ));
    }
    identity.push(actions);

    let mut window_cards = Vec::new();
    for window in windows {
        let observed = frames
            .iter()
            .filter(|frame| frame.window == window.id)
            .count();
        window_cards.push(
            Container::with_child(Padding::all(
                16.,
                column([
                    wide_row([
                        glyph(IconName::AppWindow, 16., primary()),
                        gap(10., 1.),
                        Expanded::new(line_text(window.title.clone(), 13.5, text_primary())).into(),
                        badge(format!("{observed} frames"), text_muted()),
                    ]),
                    gap(1., 10.),
                    divider(),
                    gap(1., 6.),
                    key_value(
                        "Logical size",
                        format!(
                            "{:.0} × {:.0}",
                            window.logical_size[0], window.logical_size[1]
                        ),
                    ),
                    key_value("Display scale", format!("{:.2}×", window.scale_factor)),
                    key_value(
                        "Physical size",
                        format!(
                            "{:.0} × {:.0}",
                            window.logical_size[0] * window.scale_factor as f32,
                            window.logical_size[1] * window.scale_factor as f32
                        ),
                    ),
                ]),
            ))
            .color(surface_raised())
            .radius(10.)
            .into(),
        );
        window_cards.push(gap(1., 10.));
    }
    if window_cards.is_empty() {
        window_cards.push(empty_state(
            IconName::AppWindow,
            "No live windows",
            "The target has not reported any native window yet.",
        ));
    }
    column([
        section_with(
            "Session identity",
            "Connected application and framework metadata",
            Some(status_badge(
                connection.label(),
                if connected {
                    success()
                } else {
                    super::shared::warning()
                },
            )),
            column(identity),
        ),
        gap(1., 14.),
        section_with(
            "Native windows",
            "Window metrics reported by the target",
            None,
            column(window_cards),
        ),
    ])
}
