//! Headless frame profiler for any DevTools-enabled target, including
//! DevTools itself (start it with `INCULAR_DEVTOOLS=1`).
//!
//! ```text
//! cargo run -p incular-devtools-ui --example devtools_profile_probe -- \
//!     --target-pid <pid> [--seconds 10] [--deep]
//! ```
//!
//! Prints per-phase CPU statistics over the frames the target presents, and
//! with `--deep` the widget types that spent the most build, layout and paint
//! time.
use futures_util::{SinkExt, StreamExt};
use incular_devtools_protocol::{
    DevWidgetId, DevtoolsProfilerMode, FrameTimingsWire, Hello, Message, PROTOCOL_VERSION,
    PeerKind, RequestMethod, ResponsePayload, TargetEvent, TracePhase, TreeDelta,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio_tungstenite::tungstenite::Message as Frame;

fn argument(name: &str) -> Option<String> {
    let mut arguments = std::env::args().skip_while(|value| value != name);
    arguments.next()?;
    arguments.next()
}

fn stats(label: &str, mut values: Vec<u32>) {
    if values.is_empty() {
        return;
    }
    values.sort_unstable();
    let mean = values.iter().map(|value| f64::from(*value)).sum::<f64>() / values.len() as f64;
    let p95 = values[((values.len() as f64 * 0.95).ceil() as usize).clamp(1, values.len()) - 1];
    let max = values[values.len() - 1];
    println!(
        "{label:<18} {:>9.2} {:>9.2} {:>9.2}",
        mean / 1000.,
        f64::from(p95) / 1000.,
        f64::from(max) / 1000.
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pid = argument("--target-pid")
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or("--target-pid <pid> is required")?;
    let seconds = argument("--seconds")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(10);
    let deep = std::env::args().any(|value| value == "--deep");
    let report = incular_devtools_ui::session::scan_sessions();
    let record = incular_devtools_ui::session::select_session(&report.sessions, Some(pid))
        .ok_or("no DevTools discovery record for that pid")?;
    let (socket, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}/", record.port)).await?;
    let (mut sink, mut source) = socket.split();
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        devtools_version: incular_devtools_protocol::DEVTOOLS_VERSION.into(),
        kind: PeerKind::Devtools,
        auth_token: record.auth_token.clone(),
        take_over: true,
    };
    sink.send(Frame::Text(serde_json::to_string(&hello)?))
        .await?;
    let mut next_id = 1_u64;
    let mut send = |body: RequestMethod| {
        let message = Message::Request {
            request_id: next_id,
            body,
        };
        next_id += 1;
        serde_json::to_string(&message)
    };
    let mode = if deep {
        DevtoolsProfilerMode::Deep
    } else {
        DevtoolsProfilerMode::Performance
    };
    sink.send(Frame::Text(send(RequestMethod::SetProfilerMode { mode })?))
        .await?;
    if deep {
        sink.send(Frame::Text(send(RequestMethod::StartRecording)?))
            .await?;
    }
    sink.send(Frame::Text(send(RequestMethod::GetTargetInfo)?))
        .await?;

    let mut timings: Vec<FrameTimingsWire> = Vec::new();
    let mut work: HashMap<(DevWidgetId, TracePhase), (u64, u64)> = HashMap::new();
    let mut window = None;
    let deadline = Instant::now() + Duration::from_secs(seconds);
    println!("profiling pid {pid} for {seconds}s ({mode:?})…");
    while Instant::now() < deadline {
        let Ok(Some(frame)) = tokio::time::timeout(Duration::from_millis(200), source.next()).await
        else {
            continue;
        };
        let Ok(Frame::Text(text)) = frame else {
            continue;
        };
        match serde_json::from_str::<Message>(&text) {
            Ok(Message::Event(TargetEvent::FrameRecord(frame))) => {
                window.get_or_insert(frame.window);
                timings.push(frame.timings);
            }
            Ok(Message::Event(TargetEvent::DeepTrace(trace))) => {
                for event in trace.events {
                    let entry = work.entry((event.node, event.phase)).or_default();
                    entry.0 += u64::from(event.duration_us);
                    entry.1 += 1;
                }
            }
            Ok(Message::Response {
                payload: Ok(ResponsePayload::TargetInfo(info)),
                ..
            }) => {
                window = info.windows.first().map(|window| window.id).or(window);
            }
            _ => {}
        }
    }
    println!("{} frames", timings.len());
    println!(
        "{:<18} {:>9} {:>9} {:>9}",
        "phase (ms)", "mean", "p95", "max"
    );
    for (label, pick) in [
        (
            "events",
            (|t: &FrameTimingsWire| t.event_processing) as fn(&FrameTimingsWire) -> u32,
        ),
        ("messages", |t| t.runtime_messages),
        ("build", |t| t.build),
        ("layout", |t| t.layout),
        ("paint", |t| t.paint),
        ("semantics", |t| t.semantics),
        ("composite", |t| t.composite),
        ("prepare", |t| t.prepare),
        ("encode", |t| t.encode),
        ("submit", |t| t.submit),
        ("cpu total", |t| t.cpu_total),
    ] {
        stats(label, timings.iter().map(pick).collect());
    }
    if !deep || work.is_empty() {
        return Ok(());
    }
    // Resolve widget type names from one tree snapshot.
    let mut names = HashMap::new();
    if let Some(window) = window {
        sink.send(Frame::Text(send(RequestMethod::StopRecording)?))
            .await?;
        sink.send(Frame::Text(send(RequestMethod::GetWidgetTree { window })?))
            .await?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && names.is_empty() {
            let Ok(Some(Ok(Frame::Text(text)))) =
                tokio::time::timeout(Duration::from_millis(500), source.next()).await
            else {
                continue;
            };
            if let Ok(Message::Response {
                payload: Ok(ResponsePayload::WidgetTree { deltas, .. }),
                ..
            }) = serde_json::from_str::<Message>(&text)
            {
                for delta in deltas {
                    if let TreeDelta::Snapshot { root, nodes, .. } = delta {
                        names.insert(root.id, root.type_name.clone());
                        for node in nodes {
                            names.insert(node.id, node.type_name);
                        }
                    }
                }
            }
        }
    }
    let mut by_type: HashMap<(String, TracePhase), (u64, u64)> = HashMap::new();
    for ((node, phase), (total, count)) in &work {
        let name = names
            .get(node)
            .cloned()
            .unwrap_or_else(|| "<unmounted>".into());
        let entry = by_type.entry((name, *phase)).or_default();
        entry.0 += total;
        entry.1 += count;
    }
    let mut ranked = by_type.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    println!(
        "\n{:<34} {:<10} {:>10} {:>9}",
        "widget type", "phase", "total ms", "count"
    );
    for ((name, phase), (total, count)) in ranked.into_iter().take(30) {
        println!(
            "{name:<34} {:<10} {:>10.2} {count:>9}",
            format!("{phase:?}"),
            total as f64 / 1000.
        );
    }
    Ok(())
}
