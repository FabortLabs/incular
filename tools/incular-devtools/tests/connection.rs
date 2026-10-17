use incular_devtools_ui::inspector::{ConnectionState, InspectorModel};
use std::time::{Duration, Instant};

fn logged(model: &InspectorModel) -> usize {
    model.filtered_console().len()
}

#[test]
fn repeated_identical_failures_are_logged_once() {
    let mut model = InspectorModel::default();
    for _ in 0..5 {
        model.transition(
            ConnectionState::Disconnected,
            Some("connection refused".into()),
            Some(Duration::from_secs(2)),
        );
        model.transition(ConnectionState::Connecting, None, None);
    }
    assert_eq!(logged(&model), 1);
    model.transition(
        ConnectionState::Disconnected,
        Some("target disconnected".into()),
        None,
    );
    assert_eq!(logged(&model), 2);
}

#[test]
fn a_successful_connection_rearms_failure_logging() {
    let mut model = InspectorModel::default();
    model.transition(ConnectionState::Disconnected, Some("refused".into()), None);
    model.transition(ConnectionState::Connected, None, None);
    assert_eq!(model.connection_headline(Instant::now()).0, "CONNECTED");
    model.transition(ConnectionState::Disconnected, Some("refused".into()), None);
    assert_eq!(logged(&model), 2);
}

#[test]
fn headline_matches_every_state() {
    let now = Instant::now();
    let mut model = InspectorModel::default();
    let cases = [
        (
            ConnectionState::Discovering,
            "SEARCHING",
            "Searching for targets",
        ),
        (ConnectionState::Connecting, "CONNECTING", "Connecting"),
        (ConnectionState::Authenticating, "CONNECTING", "Connecting"),
        (ConnectionState::Busy, "BUSY", "Session busy"),
        (ConnectionState::TakenOver, "TAKEN OVER", "Taken over"),
    ];
    for (state, badge, text) in cases {
        model.transition(state, Some("detail".into()), None);
        let (actual_badge, actual_text) = model.connection_headline(now);
        assert_eq!(actual_badge, badge);
        assert!(actual_text.starts_with(text), "{actual_text}");
    }
}

#[test]
fn disconnected_headline_counts_down_and_conflicts_do_not_retry() {
    let mut model = InspectorModel::default();
    model.transition(
        ConnectionState::Disconnected,
        Some("gone".into()),
        Some(Duration::from_secs(4)),
    );
    let (badge, text) = model.connection_headline(Instant::now());
    assert_eq!(badge, "OFFLINE");
    assert!(text.contains("retrying in"), "{text}");
    model.transition(ConnectionState::Busy, Some("busy".into()), None);
    assert!(
        !model
            .connection_headline(Instant::now())
            .1
            .contains("retrying")
    );
    assert!(ConnectionState::Busy.is_conflict());
    assert!(ConnectionState::TakenOver.is_conflict());
    assert!(!ConnectionState::Disconnected.is_conflict());
}

#[test]
fn inspection_requires_a_v3_target() {
    let model = InspectorModel::default();
    assert!(!model.supports_inspection());
}
