use std::{
    cell::RefCell,
    future::{Future, poll_fn},
    io,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    UndoDirection, UndoManager, UndoManagerClient, UndoManagerInvocationError,
    UndoManagerMessenger, UndoManagerSendError,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};

type Calls = Rc<RefCell<Vec<(&'static str, MethodCall<Value>)>>>;

struct Host {
    recorded: Calls,
    fail: bool,
}

impl UndoManagerMessenger for Host {
    type Error = io::Error;

    fn send_undo_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        self.recorded.borrow_mut().push((
            channel,
            JsonMethodCodec::decode_method_call(&payload).unwrap(),
        ));
        let fail = self.fail;
        let mut pending = true;
        poll_fn(move |cx| {
            if std::mem::take(&mut pending) {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else if fail {
                Poll::Ready(Err(io::Error::other("Channel failed")))
            } else {
                Poll::Ready(Ok(()))
            }
        })
    }
}

struct Client(Rc<RefCell<Vec<UndoDirection>>>);
impl UndoManagerClient for Client {
    fn undo(&mut self) {}
    fn redo(&mut self) {}
    fn can_undo(&self) -> bool {
        false
    }
    fn can_redo(&self) -> bool {
        false
    }
    fn handle_platform_undo(&mut self, direction: UndoDirection) {
        self.0.borrow_mut().push(direction);
    }
}

#[test]
fn inbound_json_undo_and_redo_are_delivered_to_the_current_client() {
    let mut service = UndoManager::new(Host {
        recorded: Calls::default(),
        fail: false,
    });
    let trace = Rc::new(RefCell::new(Vec::new()));
    service.set_client(Some(Box::new(Client(trace.clone()))));
    assert!(trace.borrow().is_empty());
    for (request, expected) in [
        (
            br#"{"args":["undo"],"method":"UndoManagerClient.handleUndo"}"#,
            UndoDirection::Undo,
        ),
        (
            br#"{"args":["redo"],"method":"UndoManagerClient.handleUndo"}"#,
            UndoDirection::Redo,
        ),
    ] {
        let reply = service.handle_platform_message(request).unwrap();
        assert_eq!(
            JsonMethodCodec::decode_envelope(&reply).unwrap(),
            Value::Null
        );
        assert_eq!(trace.borrow().last(), Some(&expected));
    }
    assert_eq!(*trace.borrow(), [UndoDirection::Undo, UndoDirection::Redo]);

    let invalid = br#"{"args":["invalid"],"method":"UndoManagerClient.handleUndo"}"#;
    assert!(matches!(
        service.handle_platform_message(invalid),
        Err(UndoManagerInvocationError::UnknownDirection(_))
    ));
    service.set_client(None);
    assert!(matches!(
        service.handle_platform_message(
            br#"{"method":"UndoManagerClient.handleUndo","args":["redo"]}"#
        ),
        Err(UndoManagerInvocationError::NoClient)
    ));
    assert_eq!(trace.borrow().len(), 2);
}

#[test]
fn an_outbound_state_failure_is_reported_once_after_the_channel_completes() {
    let recorded = Calls::default();
    let mut service = UndoManager::new(Host {
        recorded: recorded.clone(),
        fail: true,
    });
    let reports = Rc::new(RefCell::new(Vec::new()));
    let sink = reports.clone();
    service.set_error_reporter(move |report| {
        let UndoManagerSendError::Transport(source) = report.error else {
            panic!("wrong error kind");
        };
        sink.borrow_mut()
            .push((report.library, report.context, source.to_string()));
    });
    let mut sending = std::pin::pin!(service.set_undo_state(true, true));
    let mut context = Context::from_waker(Waker::noop());
    assert!(sending.as_mut().poll(&mut context).is_pending());
    assert!(reports.borrow().is_empty());
    assert_eq!(
        recorded.borrow().as_slice(),
        [(
            "flutter/undomanager",
            MethodCall {
                method: "UndoManager.setUndoState".into(),
                arguments: json!({"canUndo": true, "canRedo": true})
            },
        )]
    );
    let Poll::Ready(Err(UndoManagerSendError::Transport(error))) =
        sending.as_mut().poll(&mut context)
    else {
        panic!("the underlying error should remain available to Rust");
    };
    assert_eq!(error.to_string(), "Channel failed");
    assert_eq!(
        reports.borrow().as_slice(),
        [(
            "services library",
            "while sending the UndoManager.setUndoState event",
            "Channel failed".to_owned()
        )]
    );
}
