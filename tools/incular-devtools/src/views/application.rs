use super::shared::{badge, column, key_value, page_title};
use super::{PRIMARY, TEXT_MUTED, compact_button, gap, section, ui_text};
use crate::{inspector::ConnectionState, transport::ClientBridge};
use incular::prelude::*;
use incular_devtools_protocol::{FrameRecordEvent, RequestMethod, TargetInfo, WindowSummary};

pub(crate) fn build_application(
    target_info: Option<TargetInfo>,
    connection: ConnectionState,
    windows: Vec<WindowSummary>,
    frames: Vec<FrameRecordEvent>,
    bridge: ClientBridge,
) -> Widget {
    let connected = connection == ConnectionState::Connected;
    let control = compact_button(
        if connected {
            "Refresh target info"
        } else {
            "Retry connection"
        },
        false,
        move || {
            if connected {
                bridge.send_or_report(RequestMethod::GetTargetInfo);
            } else {
                bridge.retry();
            }
        },
    );
    let mut identity = vec![
        Row::new([
            badge(
                connection.label().to_uppercase(),
                if connected {
                    PRIMARY
                } else {
                    super::shared::WARNING
                },
            ),
            gap(12., 1.),
            control,
        ])
        .into(),
        gap(1., 16.),
    ];
    if let Some(info) = target_info {
        identity.extend([
            key_value("Framework", format!("Incular {}", info.framework_version)),
            key_value("Process", format!("PID {}", info.pid)),
            key_value("Executable", info.executable),
            key_value("Platform", info.platform),
            key_value("Protocol", info.protocol_version.to_string()),
        ]);
    } else {
        identity.push(ui_text(
            "Start a DevTools-enabled application, then retry the connection.",
            13.,
            TEXT_MUTED,
        ));
    }
    let mut window_rows = Vec::new();
    for window in windows {
        let observed = frames
            .iter()
            .filter(|frame| frame.window == window.id)
            .count();
        window_rows.extend([
            super::shared::heading(window.title, 14.),
            gap(1., 8.),
            key_value(
                "Logical size",
                format!(
                    "{:.0} × {:.0}",
                    window.logical_size[0], window.logical_size[1]
                ),
            ),
            key_value("Display scale", format!("{:.2}×", window.scale_factor)),
            key_value("Observed frames", observed.to_string()),
            gap(1., 12.),
        ]);
    }
    if window_rows.is_empty() {
        window_rows.push(ui_text("No live windows reported.", 13., TEXT_MUTED));
    }
    column([
        page_title(
            "Application",
            "Session details, target identity, and native windows.",
        ),
        section(
            "Session identity",
            "Connected application and framework metadata",
            column(identity),
        ),
        gap(1., 20.),
        section(
            "Native windows",
            "Window metrics reported by the target",
            column(window_rows),
        ),
    ])
}
