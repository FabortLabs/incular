//! Offline workspace: one coherent connection status plus every live target.

use super::shared::{
    WARNING, badge, column, heading, mono, page_title, panel, refresh, text_style,
};
use super::{
    BORDER, CONTROL, DANGER, PRIMARY, TEXT_MUTED, TEXT_PRIMARY, compact_button, gap, section,
    ui_text,
};
use crate::{
    diagnostics::format_age,
    inspector::{ConnectionState, Shared},
    transport::ClientBridge,
};
use incular::{material::RawMaterialButton, prelude::*};
use incular_devtools_protocol::DiscoveryRecord;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn status_color(state: ConnectionState) -> Color {
    match state {
        ConnectionState::Connected => PRIMARY,
        ConnectionState::Busy | ConnectionState::TakenOver => WARNING,
        ConnectionState::Disconnected => DANGER,
        _ => TEXT_MUTED,
    }
}

fn explanation(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Discovering => {
            "DevTools looks for applications that were started with INCULAR_DEVTOOLS=1 or --devtools and connects to the newest one."
        }
        ConnectionState::Connecting | ConnectionState::Authenticating => {
            "Opening a local WebSocket and authenticating with the session token."
        }
        ConnectionState::Busy => {
            "A target accepts one DevTools window at a time. Taking over ends the other window's session; it will not reconnect on its own."
        }
        ConnectionState::TakenOver => {
            "Another DevTools window took this session over. This window stays disconnected until you take it back."
        }
        ConnectionState::Disconnected => {
            "The connection ended. DevTools reconnects automatically with a growing delay."
        }
        ConnectionState::Connected | ConnectionState::Stopping => "",
    }
}

fn target_row(
    record: &DiscoveryRecord,
    current: bool,
    now_ms: u64,
    bridge: ClientBridge,
) -> Widget {
    let pid = record.pid;
    let connect: Widget = RawMaterialButton::new(format!("Connect to {} ({pid})", record.app_name))
        .size(Size::new(92., 30.))
        .content(Align::new(
            Alignment::CENTER,
            Text::new(if current { "Reconnect" } else { "Connect" }).style(text_style(
                12.,
                if current { PRIMARY } else { TEXT_PRIMARY },
            )),
        ))
        .color(CONTROL)
        .hover_color(Color::rgba(45, 53, 65, 255))
        .focused_color(Color::rgba(48, 65, 69, 255))
        .on_press(move || bridge.connect(pid, false))
        .into();
    Container::with_child(Padding::new(
        EdgeInsets::symmetric(14., 10.),
        Row::new([
            Widget::box_(
                Size::new(3., 34.),
                if current { PRIMARY } else { Color::TRANSPARENT },
            ),
            gap(12., 1.),
            Expanded::new(column([
                heading(record.app_name.clone(), 13.),
                gap(1., 4.),
                mono(
                    format!(
                        "pid {pid} · started {} · port {}",
                        format_age(now_ms.saturating_sub(record.started_unix_ms)),
                        record.port
                    ),
                    11.,
                    TEXT_MUTED,
                ),
            ]))
            .into(),
            badge(format!("protocol v{}", record.protocol_version), TEXT_MUTED),
            gap(12., 1.),
            connect,
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center),
    ))
    .border(Border::new(1., BORDER))
    .radius(6.)
    .into()
}

pub(crate) fn build_connection(shared: Shared, bridge: ClientBridge, tick: Signal<u64>) -> Widget {
    let (state, headline, targets, connect_pid, error) = {
        let model = shared.lock().expect("inspector state");
        (
            model.connection,
            model.connection_headline(Instant::now()).1,
            model.targets.clone(),
            model.connect_pid,
            model.error.clone(),
        )
    };
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64);
    let mut status = vec![
        Row::new([
            badge(state.badge(), status_color(state)),
            gap(12., 1.),
            Expanded::new(
                Text::new(headline)
                    .style(text_style(15., TEXT_PRIMARY).font_weight(FontWeight::W600))
                    .max_lines(Some(1))
                    .overflow(TextOverflow::Ellipsis),
            )
            .into(),
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into(),
        gap(1., 10.),
        ui_text(explanation(state), 12., TEXT_MUTED),
    ];
    if let Some(error) = error.filter(|_| state != ConnectionState::Discovering) {
        status.extend([gap(1., 10.), ui_text(error, 12., status_color(state))]);
    }
    let mut actions: Vec<Widget> = Vec::new();
    if state.is_conflict()
        && let Some(pid) = connect_pid
    {
        let take_bridge = bridge.clone();
        actions.push(compact_button("Take over", true, move || {
            take_bridge.connect(pid, true)
        }));
    }
    if matches!(
        state,
        ConnectionState::Disconnected | ConnectionState::Busy | ConnectionState::TakenOver
    ) {
        let retry_bridge = bridge.clone();
        actions.push(compact_button(
            if state == ConnectionState::Disconnected {
                "Retry now"
            } else {
                "Reconnect"
            },
            false,
            move || retry_bridge.retry(),
        ));
    }
    if !actions.is_empty() {
        status.extend([
            gap(1., 14.),
            Wrap::new(actions).spacing(8.).run_spacing(8.).into(),
        ]);
    }

    let refresh_shared = shared.clone();
    let refresh_tick = tick.clone();
    let list_action = compact_button("Refresh list", false, move || {
        let report = crate::session::scan_sessions();
        if let Ok(mut model) = refresh_shared.lock() {
            model.targets = report.sessions;
        }
        refresh(&refresh_tick);
    });
    let mut list: Vec<Widget> = vec![
        Row::new([
            Expanded::new(ui_text(
                format!(
                    "{} running {}",
                    targets.len(),
                    if targets.len() == 1 {
                        "target"
                    } else {
                        "targets"
                    }
                ),
                12.,
                TEXT_MUTED,
            ))
            .into(),
            list_action,
        ])
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .into(),
        gap(1., 12.),
    ];
    if targets.is_empty() {
        list.push(panel(
            Padding::all(
                16.,
                column([
                    heading("No DevTools-enabled application found", 13.),
                    gap(1., 6.),
                    ui_text(
                        "Start one with INCULAR_DEVTOOLS=1 (agent only) or --devtools (also opens this window).",
                        12.,
                        TEXT_MUTED,
                    ),
                ]),
            )
            .into(),
        ));
    }
    for (index, record) in targets.iter().enumerate() {
        if index > 0 {
            list.push(gap(1., 8.));
        }
        let current = connect_pid == Some(record.pid)
            && matches!(
                state,
                ConnectionState::Connecting
                    | ConnectionState::Authenticating
                    | ConnectionState::Busy
            );
        list.push(target_row(record, current, now_ms, bridge.clone()));
    }

    column([
        page_title(
            "Connection",
            "Connect Incular DevTools to a running application.",
        ),
        panel(Padding::all(18., column(status)).into()),
        gap(1., 20.),
        section(
            "Running targets",
            "Applications that published a DevTools discovery record",
            column(list),
        ),
    ])
}
