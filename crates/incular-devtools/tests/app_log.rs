//! An application log record becomes a `TargetEvent::Log` on the wire.

use futures_util::{SinkExt, StreamExt};
use incular_devtools::{
    discovery::list_sessions, session::SessionConfig, session::generate_token, spawn,
};
use incular_devtools_protocol::{
    DEVTOOLS_VERSION, Hello, Message, PROTOCOL_VERSION, PeerKind, TargetEvent,
};
use incular_runtime::{LogLevel, log};
use std::{sync::Arc, time::Duration};
use tokio_tungstenite::tungstenite::Message as Ws;

#[tokio::test]
async fn logged_message_arrives_as_log_event() {
    let name = format!("applog-{}", generate_token().unwrap());
    let handle = spawn(
        SessionConfig {
            app_name: name.clone(),
        },
        tokio::runtime::Handle::current(),
        Arc::new(|| {}),
    )
    .unwrap();
    let (record, _) = list_sessions()
        .into_iter()
        .find(|(record, _)| record.app_name == name)
        .unwrap();
    let (mut client, _) =
        tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{}", record.port))
            .await
            .unwrap();
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        devtools_version: DEVTOOLS_VERSION.into(),
        kind: PeerKind::Devtools,
        auth_token: record.auth_token,
        take_over: false,
    };
    client
        .send(Ws::Text(serde_json::to_string(&hello).unwrap()))
        .await
        .unwrap();

    // Telemetry is session-scoped: wait for the handshake ack before logging.
    let ack = tokio::time::timeout(Duration::from_secs(3), client.next())
        .await
        .expect("handshake ack");
    assert!(ack.is_some());
    let _ = incular_runtime::drain_logs(usize::MAX);
    log(LogLevel::Warn, "tests::applog", "hello from the app");
    assert_eq!(handle.telemetry.forward_app_logs(16), 1);

    let found = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(Ok(Ws::Text(text))) = client.next().await {
            if let Ok(Message::Event(TargetEvent::Log {
                level,
                target,
                message,
            })) = serde_json::from_str(&text)
            {
                return (level, target, message);
            }
        }
        panic!("connection closed before the log event");
    })
    .await
    .expect("log event delivered");
    assert_eq!(
        found,
        (
            "warn".into(),
            "tests::applog".into(),
            "hello from the app".into()
        )
    );
    handle.stop();
}
