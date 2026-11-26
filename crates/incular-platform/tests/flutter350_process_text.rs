use incular_platform::{
    DefaultProcessTextService, FlutterPlatformMessenger, ProcessTextAction, ProcessTextError,
    ProcessTextService,
    message_codecs::{MessageValue as V, MethodCall, StandardMethodCodec as Codec},
};
use std::{
    cell::RefCell,
    future::{Future, ready},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type Calls = Rc<RefCell<Vec<(&'static str, MethodCall)>>>;
struct Host {
    calls: Calls,
    reply: fn(&MethodCall) -> Option<V>,
}

impl FlutterPlatformMessenger for Host {
    type Error = std::convert::Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        let call = Codec::decode_method_call(&payload).unwrap();
        let result =
            (self.reply)(&call).map(|value| Codec::encode_success_envelope(&value).unwrap());
        self.calls.borrow_mut().push((channel, call));
        ready(Ok(result))
    }
}

fn wait<T>(future: impl Future<Output = T>) -> T {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test host returns immediately"),
    }
}

fn service(reply: fn(&MethodCall) -> Option<V>) -> (DefaultProcessTextService<Host>, Calls) {
    let calls = Calls::default();
    (
        DefaultProcessTextService::new(Host {
            calls: calls.clone(),
            reply,
        }),
        calls,
    )
}

fn string(value: &str) -> V {
    V::String(value.to_owned())
}

#[test]
fn the_action_query_sends_one_standard_method_with_null_arguments() {
    let (mut service, calls) = service(|_| Some(V::Null));
    assert!(wait(service.query_text_actions()).unwrap().is_empty());
    assert_eq!(
        calls.take(),
        [(
            "flutter/processtext",
            MethodCall {
                method: "ProcessText.queryTextActions".into(),
                arguments: V::Null,
            }
        )]
    );
}

#[test]
fn a_text_processing_request_preserves_action_text_and_read_only_arguments() {
    let (mut service, calls) = service(|_| Some(V::Null));
    assert_eq!(
        wait(service.process_text_action("fakeActivity.fakeAction", "Flutter", false)).unwrap(),
        None
    );
    assert_eq!(
        calls.take(),
        [(
            "flutter/processtext",
            MethodCall {
                method: "ProcessText.processTextAction".into(),
                arguments: V::List(vec![
                    string("fakeActivity.fakeAction"),
                    string("Flutter"),
                    V::Bool(false)
                ]),
            }
        )]
    );
}

#[test]
fn replies_yield_ordered_actions_transformed_text_or_no_transformation() {
    const ONE: &str = "fakeActivity.fakeAction1";
    const TWO: &str = "fakeActivity.fakeAction2";
    let (mut service, _) = service(|call| match call.method.as_str() {
        "ProcessText.queryTextActions" => Some(V::Map(vec![
            (string(ONE), string("Action1")),
            (string(TWO), string("Action2")),
        ])),
        "ProcessText.processTextAction" => match &call.arguments {
            V::List(args) if args[0] == string(ONE) => {
                let V::String(text) = &args[1] else {
                    panic!("text argument")
                };
                Some(string(&format!("{text}!!!")))
            }
            _ => Some(V::Null),
        },
        _ => None,
    });
    assert_eq!(
        wait(service.query_text_actions()).unwrap(),
        [
            ProcessTextAction {
                id: ONE.into(),
                label: "Action1".into()
            },
            ProcessTextAction {
                id: TWO.into(),
                label: "Action2".into()
            },
        ]
    );
    assert_eq!(
        wait(service.process_text_action(ONE, "Flutter", false))
            .unwrap()
            .as_deref(),
        Some("Flutter!!!")
    );
    assert_eq!(
        wait(service.process_text_action(TWO, "Flutter", false)).unwrap(),
        None
    );
}

#[test]
fn absent_and_malformed_replies_follow_the_separate_query_and_process_contracts() {
    let (mut unhandled, _) = service(|_| None);
    assert!(wait(unhandled.query_text_actions()).unwrap().is_empty());
    assert_eq!(
        wait(unhandled.process_text_action("none", "text", true)).unwrap(),
        None
    );

    let (mut wrong_response, _) = service(|_| Some(V::Bool(false)));
    assert!(
        wait(wrong_response.query_text_actions())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        wait(wrong_response.process_text_action("wrong", "text", true)),
        Err(ProcessTextError::InvalidProcessedText)
    ));
}
