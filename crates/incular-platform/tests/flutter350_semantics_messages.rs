//! Flutter 3.50 accessibility announcement and event delivery.
use std::{
    cell::RefCell,
    future::{Future, ready},
    io,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_config::TextDirection;
use incular_platform::{
    AccessibilityMessenger, Assertiveness, SemanticsAnnouncement, SemanticsEvent,
    SemanticsMessageError, SemanticsService,
    message_codecs::{MessageValue as Msg, StandardMessageCodec},
};

type Calls = Rc<RefCell<Vec<(&'static str, Msg)>>>;

struct Capture(Calls);

impl AccessibilityMessenger for Capture {
    type Error = io::Error;
    fn send_accessibility_message(
        &mut self,
        channel: &'static str,
        bytes: Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        self.0.borrow_mut().push((
            channel,
            StandardMessageCodec::decode_message(Some(&bytes)).unwrap(),
        ));
        ready(Ok(()))
    }
}

fn keyed(entries: impl IntoIterator<Item = (&'static str, Msg)>) -> Msg {
    Msg::Map(
        entries
            .into_iter()
            .map(|(k, v)| (Msg::String(k.into()), v))
            .collect(),
    )
}

fn immediate<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("capture replies immediately"),
    }
}

#[test]
fn announcements_use_view_id_and_flutter_direction_and_priority_ordinals() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut service = SemanticsService::new(Capture(calls.clone()));
    immediate(service.send_announcement(SemanticsAnnouncement::new(
        0,
        "announcement 1",
        TextDirection::Ltr,
    )))
    .unwrap();
    immediate(
        service.send_announcement(
            SemanticsAnnouncement::new(0, "announcement 2", TextDirection::Rtl)
                .assertiveness(Assertiveness::Assertive),
        ),
    )
    .unwrap();

    let announce = |message: &str, direction, assertive| {
        let mut data = vec![
            ("viewId", Msg::Int(0)),
            ("message", Msg::String(message.into())),
            ("textDirection", Msg::Int(direction)),
        ];
        if assertive {
            data.push(("assertiveness", Msg::Int(1)));
        }
        (
            "flutter/accessibility",
            keyed([
                ("type", Msg::String("announce".into())),
                ("data", keyed(data)),
            ]),
        )
    };
    assert_eq!(
        *calls.borrow(),
        [
            announce("announcement 1", 1, false),
            announce("announcement 2", 0, true)
        ]
    );
}

#[test]
fn node_ids_are_optional_and_detached_nodes_do_not_send_events() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut service = SemanticsService::new(Capture(calls.clone()));
    assert!(!immediate(service.send_node_event(None, SemanticsEvent::Focus)).unwrap());
    assert!(calls.borrow().is_empty());
    for (event, name, fields) in [
        (
            SemanticsEvent::Tooltip("tip".into()),
            "tooltip",
            vec![("message", Msg::String("tip".into()))],
        ),
        (SemanticsEvent::Tap, "tap", vec![]),
        (SemanticsEvent::LongPress, "longPress", vec![]),
        (SemanticsEvent::Focus, "focus", vec![]),
    ] {
        assert!(immediate(service.send_node_event(Some(7), event)).unwrap());
        assert_eq!(
            calls.borrow_mut().pop().unwrap(),
            (
                "flutter/accessibility",
                keyed([
                    ("type", Msg::String(name.into())),
                    ("data", keyed(fields)),
                    ("nodeId", Msg::Int(7))
                ])
            )
        );
    }
}

struct Failing;

impl AccessibilityMessenger for Failing {
    type Error = io::Error;
    fn send_accessibility_message(
        &mut self,
        _channel: &'static str,
        _bytes: Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        let mut first = true;
        std::future::poll_fn(move |cx| {
            if std::mem::take(&mut first) {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(Err(io::Error::other("Test exception")))
            }
        })
    }
}

#[test]
fn an_attached_node_reports_its_async_transport_error_once_with_context() {
    let mut service = SemanticsService::new(Failing);
    let reports = Rc::new(RefCell::new(Vec::new()));
    let seen = reports.clone();
    service.set_node_error_reporter(move |report| {
        let SemanticsMessageError::Send(source) = report.error else {
            panic!("the transport failure must keep its original error");
        };
        seen.borrow_mut().push((
            report.node_id,
            report.library,
            report.context,
            source.to_string(),
        ));
    });
    assert!(!immediate(service.send_node_event(None, SemanticsEvent::Focus)).unwrap());
    assert!(reports.borrow().is_empty());
    let mut result = std::pin::pin!(service.send_node_event(
        Some(1),
        SemanticsEvent::Announce(SemanticsAnnouncement::new(0, "hello", TextDirection::Ltr)),
    ));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(result.as_mut().poll(&mut cx).is_pending());
    assert!(reports.borrow().is_empty());
    let Poll::Ready(Err(error)) = result.as_mut().poll(&mut cx) else {
        panic!("missing send error");
    };
    assert_eq!(
        error.to_string(),
        "while sending accessibility event: Test exception"
    );
    assert!(
        matches!(error,SemanticsMessageError::Send(source) if source.to_string()=="Test exception")
    );
    assert_eq!(
        *reports.borrow(),
        [(
            1,
            "semantics library",
            "while sending accessibility event",
            "Test exception".to_owned()
        )]
    );
}
