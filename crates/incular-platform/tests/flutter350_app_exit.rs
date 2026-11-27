use std::{
    cell::RefCell,
    future::Future,
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    AppExitResponse, AppExitType, FlutterAppExit, FlutterInboundMessenger, FlutterJsonMethodResult,
    FlutterPlatformMessenger,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use serde_json::{Value, json};

fn wait<T>(future: impl Future<Output = T>) -> T {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("the mock engine replies immediately"),
    }
}

#[derive(Default)]
struct Engine {
    sent: Vec<(&'static str, MethodCall<Value>)>,
    response: Option<Value>,
}

impl FlutterPlatformMessenger for Engine {
    type Error = std::convert::Infallible;

    async fn send_platform_message(
        &mut self,
        channel: &'static str,
        message: Option<Vec<u8>>,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        let call = JsonMethodCodec::decode_method_call(&message.unwrap()).unwrap();
        self.sent.push((channel, call));
        Ok(self
            .response
            .as_ref()
            .map(|value| JsonMethodCodec::encode_success_envelope(value).unwrap()))
    }
}

#[test]
fn required_exit_sends_exact_channel_method_type_and_default_code_and_returns_cancel() {
    let mut engine = Engine {
        response: Some(json!({"response": "cancel"})),
        ..Engine::default()
    };
    let response = wait(FlutterAppExit::exit_application(
        &mut engine,
        AppExitType::Required,
        None,
    ))
    .unwrap();
    assert_eq!(response, AppExitResponse::Cancel);
    assert_eq!(engine.sent.len(), 1);
    let (channel, call) = &engine.sent[0];
    assert_eq!(*channel, "flutter/platform");
    assert_eq!(call.method, "System.exitApplication");
    assert_eq!(call.arguments, json!({ "type": "required", "exitCode": 0 }));
}

#[test]
fn null_or_missing_response_cancels_but_unrecognized_object_defaults_to_exit() {
    for (result, response) in [
        (None, AppExitResponse::Cancel),
        (Some(Value::Null), AppExitResponse::Cancel),
        (Some(json!({"response":"exit"})), AppExitResponse::Exit),
        (Some(json!({"response":"unknown"})), AppExitResponse::Exit),
        (Some(json!({})), AppExitResponse::Exit),
    ] {
        let mut engine = Engine {
            response: result,
            ..Engine::default()
        };
        assert_eq!(
            wait(FlutterAppExit::exit_application(
                &mut engine,
                AppExitType::Cancelable,
                Some(-7),
            ))
            .unwrap(),
            response
        );
        assert_eq!(
            engine.sent[0].1.arguments,
            json!({ "type": "cancelable", "exitCode": -7 })
        );
    }
}

fn query_app(call: MethodCall<Value>, response: AppExitResponse) -> Option<Value> {
    let bus = FlutterInboundMessenger::new(|task| task());
    FlutterAppExit::CHANNEL.set_method_call_handler(&bus, move |call, reply| {
        let result = wait(FlutterAppExit::route_request(&call, || async { response }));
        reply(result.unwrap_or(FlutterJsonMethodResult::NotImplemented)).unwrap();
    });
    let bytes = JsonMethodCodec::encode_method_call(&call).unwrap();
    let payload = Rc::new(RefCell::new(None));
    let out = payload.clone();
    bus.push("flutter/platform", Some(bytes), move |response| {
        *out.borrow_mut() = Some(response);
    });
    payload
        .take()
        .unwrap()
        .map(|response| JsonMethodCodec::decode_envelope(&response).unwrap())
}

#[test]
fn default_inbound_application_exit_query_returns_exit_envelope() {
    let call = MethodCall {
        method: "System.requestAppExit".to_owned(),
        arguments: json!([{ "type": "cancelable" }]),
    };
    assert_eq!(
        query_app(call.clone(), AppExitResponse::default()),
        Some(json!({"response":"exit"})),
    );
    assert_eq!(
        query_app(call, AppExitResponse::Cancel),
        Some(json!({"response":"cancel"})),
    );
}

#[test]
fn exit_dispatch_leaves_unrelated_platform_methods_available_for_other_handlers() {
    let unrelated = MethodCall {
        method: "SystemChrome.systemUIChange".to_owned(),
        arguments: json!([true]),
    };
    assert!(
        wait(FlutterAppExit::route_request(&unrelated, || async {
            panic!("an unrelated platform method cannot ask the application to quit")
        }))
        .is_none()
    );
    assert_eq!(query_app(unrelated, AppExitResponse::default()), None);
}
