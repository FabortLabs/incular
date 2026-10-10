use incular_devtools_protocol::*;

#[test]
fn unbounded_constraints_round_trip_and_decode_existing_target_json() {
    let constraints = DebugValue::Constraints {
        min_width: 0.,
        max_width: 176.,
        min_height: 0.,
        max_height: f32::INFINITY,
    };
    let encoded = serde_json::to_string(&constraints).unwrap();
    assert!(encoded.contains("\"max_height\":null"));
    assert_eq!(
        serde_json::from_str::<DebugValue>(&encoded).unwrap(),
        constraints
    );
    let legacy =
        r#"{"constraints":{"min_width":0.0,"max_width":null,"min_height":0.0,"max_height":null}}"#;
    assert_eq!(
        serde_json::from_str::<DebugValue>(legacy).unwrap(),
        DebugValue::Constraints {
            min_width: 0.,
            max_width: f32::INFINITY,
            min_height: 0.,
            max_height: f32::INFINITY,
        }
    );
    let finite = r#"{"constraints":{"min_width":0.0,"max_width":176.0,"min_height":0.0,"max_height":720.0}}"#;
    let parsed: DebugValue = serde_json::from_str(finite).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), finite);
}

#[test]
fn hello_round_trips_and_validates() {
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        devtools_version: DEVTOOLS_VERSION.to_owned(),
        kind: PeerKind::Devtools,
        auth_token: "tok".into(),
    };
    let json = serde_json::to_string(&hello).unwrap();
    let parsed: Hello = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, hello);
    assert_eq!(
        check_hello(&parsed, PeerKind::Devtools, Some("tok")),
        Ok(())
    );
}

#[test]
fn wrong_version_rejected() {
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION + 1,
        devtools_version: "x".into(),
        kind: PeerKind::Devtools,
        auth_token: "t".into(),
    };
    assert_eq!(
        check_hello(&hello, PeerKind::Devtools, None),
        Err(ErrorCode::ProtocolVersionMismatch)
    );
}

#[test]
fn previous_additive_protocol_version_remains_compatible() {
    let hello = Hello {
        protocol_version: MIN_SUPPORTED_PROTOCOL_VERSION,
        devtools_version: "older-ui".into(),
        kind: PeerKind::Devtools,
        auth_token: "t".into(),
    };
    assert_eq!(check_hello(&hello, PeerKind::Devtools, Some("t")), Ok(()));
}

#[test]
fn wrong_token_rejected() {
    let hello = Hello {
        protocol_version: PROTOCOL_VERSION,
        devtools_version: "x".into(),
        kind: PeerKind::Devtools,
        auth_token: "bad".into(),
    };
    assert_eq!(
        check_hello(&hello, PeerKind::Devtools, Some("good")),
        Err(ErrorCode::Unauthorized)
    );
}

#[test]
fn request_response_round_trip() {
    let message = Message::Request {
        request_id: 7,
        body: RequestMethod::HighlightNode {
            window: DevWindowId::new(1, 2),
            id: Some(DevWidgetId::new(3, 4)),
        },
    };
    let json = serde_json::to_string(&message).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, message);

    let response = Message::Response {
        request_id: 7,
        payload: Err(ErrorCode::StaleId),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("stale_id"));
    assert_eq!(serde_json::from_str::<Message>(&json).unwrap(), response);
}

#[test]
fn signal_responses_round_trip_with_empty_and_populated_lists() {
    let signal = SignalSummary {
        id: DevSignalId::new(7, 3),
        name: Some("counter".into()),
        type_name: "i64".into(),
        generation: 2,
        write_count: 4,
        subscriber_count: 1,
        last_write_summary: Some("1 → 2".into()),
        editable: true,
    };
    let subscriber = SignalSubscriber {
        signal: signal.id,
        element: DevWidgetId::new(11, 5),
        path: "Column > Text".into(),
    };
    let responses = [
        ResponsePayload::Signals {
            signals: Vec::new(),
        },
        ResponsePayload::Signals {
            signals: vec![signal.clone()],
        },
        ResponsePayload::SignalSubscribers {
            subscribers: Vec::new(),
        },
        ResponsePayload::SignalSubscribers {
            subscribers: vec![subscriber],
        },
    ];

    for (index, payload) in responses.into_iter().enumerate() {
        let message = Message::Response {
            request_id: index as u64 + 1,
            payload: Ok(payload),
        };
        let encoded = serde_json::to_string(&message)
            .expect("internally tagged signal responses must serialize");
        assert_eq!(
            serde_json::from_str::<Message>(&encoded).expect("signal response must deserialize"),
            message
        );
    }
}

#[test]
fn malformed_message_rejected() {
    let result = serde_json::from_str::<Message>(r#"{"type":"nonsense"}"#);
    assert!(result.is_err());
}

#[test]
fn deep_trace_round_trip_preserves_hierarchy_and_drop_state() {
    let message = Message::Event(TargetEvent::DeepTrace(DeepFrameTrace {
        window: DevWindowId::new(2, 1),
        frame: 9,
        events: vec![
            TraceEvent {
                node: DevWidgetId::new(1, 0),
                phase: TracePhase::Layout,
                parent: None,
                start_us: 4,
                duration_us: 20,
            },
            TraceEvent {
                node: DevWidgetId::new(2, 0),
                phase: TracePhase::Layout,
                parent: Some(0),
                start_us: 6,
                duration_us: 8,
            },
        ],
        truncated: true,
        dropped_events: 3,
    }));
    let encoded = serde_json::to_string(&message).unwrap();
    assert_eq!(serde_json::from_str::<Message>(&encoded).unwrap(), message);
}

#[test]
fn tree_delta_round_trip() {
    let delta = TreeDelta::Snapshot {
        window: DevWindowId::new(0, 1),
        root: Box::new(WidgetNode {
            id: DevWidgetId::new(0, 1),
            parent: None,
            type_name: "Column".into(),
            key: None,
            label: None,
            child_ids: vec![DevWidgetId::new(1, 1)],
            revision: 3,
        }),
        nodes: Vec::new(),
        truncated: false,
    };
    let json = serde_json::to_string(&delta).unwrap();
    assert_eq!(serde_json::from_str::<TreeDelta>(&json).unwrap(), delta);
}

#[test]
fn opaque_ids_hide_raw_values_and_detect_generations() {
    let a = DevWidgetId::new(5, 9);
    let b = DevWidgetId::new(5, 10);
    assert_ne!(a, b, "generation distinguishes reused slots");
    let text = a.to_string();
    assert!(text.contains("5") && text.contains("9"));
}
