use incular_devtools::session::generate_token;

#[test]
fn generated_tokens_are_hex_encoded_and_unique() {
    let first = generate_token().expect("the test platform provides OS entropy");
    let second = generate_token().expect("the test platform provides OS entropy");

    assert_eq!(first.len(), 32);
    assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_ne!(first, second);
}

mod concurrent_clients {
    use futures_util::{SinkExt, StreamExt};
    use incular_devtools::{
        AgentHandle, discovery::list_sessions, session::SessionConfig, session::generate_token,
        spawn,
    };
    use incular_devtools_protocol::{
        DEVTOOLS_VERSION, ErrorCode, Hello, Message, PROTOCOL_VERSION, PeerKind, ResponsePayload,
    };
    use std::{sync::Arc, time::Duration};
    use tokio::net::TcpStream;
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message as Ws};

    type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

    fn agent() -> (AgentHandle, String, String) {
        let name = format!("busy-{}", generate_token().unwrap());
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
        (
            handle,
            format!("ws://127.0.0.1:{}", record.port),
            record.auth_token,
        )
    }

    async fn connect(url: &str, token: &str, take_over: bool) -> Client {
        let (mut client, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        let hello = Hello {
            protocol_version: PROTOCOL_VERSION,
            devtools_version: DEVTOOLS_VERSION.into(),
            kind: PeerKind::Devtools,
            auth_token: token.into(),
            take_over,
        };
        client
            .send(Ws::Text(serde_json::to_string(&hello).unwrap()))
            .await
            .unwrap();
        client
    }

    async fn next_message(client: &mut Client) -> Option<Message> {
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(3), client.next())
                .await
                .expect("target answers promptly")?;
            match frame.ok()? {
                Ws::Text(text) => return Some(serde_json::from_str(&text).unwrap()),
                Ws::Close(_) => return None,
                _ => {}
            }
        }
    }

    fn is_target_info(message: &Option<Message>) -> bool {
        matches!(
            message,
            Some(Message::Response {
                payload: Ok(ResponsePayload::TargetInfo(_)),
                ..
            })
        )
    }

    #[tokio::test]
    async fn second_client_is_rejected_busy_without_disturbing_the_first() {
        let (handle, url, token) = agent();
        let mut first = connect(&url, &token, false).await;
        assert!(is_target_info(&next_message(&mut first).await));

        let mut second = connect(&url, &token, false).await;
        match next_message(&mut second).await {
            Some(Message::Rejection { code, message }) => {
                assert_eq!(code, ErrorCode::SessionBusy);
                assert!(message.contains("already being inspected"));
            }
            other => panic!("expected SessionBusy, received {other:?}"),
        }
        assert!(next_message(&mut second).await.is_none());

        // The first session is still serving requests.
        let request = Message::Request {
            request_id: 1,
            body: incular_devtools_protocol::RequestMethod::GetTargetInfo,
        };
        first
            .send(Ws::Text(serde_json::to_string(&request).unwrap()))
            .await
            .unwrap();
        handle.stop();
        tokio::time::timeout(Duration::from_secs(2), handle.stopped())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn unauthorized_client_is_not_told_the_session_is_busy() {
        let (handle, url, token) = agent();
        let mut first = connect(&url, &token, false).await;
        assert!(is_target_info(&next_message(&mut first).await));
        let mut intruder = connect(&url, "wrong-token", true).await;
        match next_message(&mut intruder).await {
            Some(Message::Rejection { code, .. }) => assert_eq!(code, ErrorCode::Unauthorized),
            other => panic!("expected Unauthorized, received {other:?}"),
        }
        handle.stop();
    }

    #[tokio::test]
    async fn take_over_replaces_the_active_session() {
        let (handle, url, token) = agent();
        let mut first = connect(&url, &token, false).await;
        assert!(is_target_info(&next_message(&mut first).await));

        let mut second = connect(&url, &token, true).await;
        match next_message(&mut first).await {
            Some(Message::Rejection { code, message }) => {
                assert_eq!(code, ErrorCode::SessionBusy);
                assert!(message.contains("took over"));
            }
            other => panic!("expected take-over rejection, received {other:?}"),
        }
        assert!(is_target_info(&next_message(&mut second).await));

        // A later plain client is now busy because of the new owner.
        let mut third = connect(&url, &token, false).await;
        assert!(matches!(
            next_message(&mut third).await,
            Some(Message::Rejection {
                code: ErrorCode::SessionBusy,
                ..
            })
        ));
        handle.stop();
        tokio::time::timeout(Duration::from_secs(2), handle.stopped())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_new_client_can_connect_after_the_active_one_leaves() {
        let (handle, url, token) = agent();
        let mut first = connect(&url, &token, false).await;
        assert!(is_target_info(&next_message(&mut first).await));
        first.close(None).await.unwrap();
        drop(first);
        let mut second = connect(&url, &token, false).await;
        // The first session may take a moment to notice the close.
        let mut message = next_message(&mut second).await;
        for _ in 0..20 {
            if is_target_info(&message) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            second = connect(&url, &token, false).await;
            message = next_message(&mut second).await;
        }
        assert!(is_target_info(&message));
        handle.stop();
    }
}
