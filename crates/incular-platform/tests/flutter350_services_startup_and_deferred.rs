use std::{
    cell::{Cell, RefCell},
    future::{Future, poll_fn},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    AppLifecycleBinding, DeferredComponent, DeferredComponentError, FlutterPlatformMessenger,
    message_codecs::{
        JsonMethodCodec, MessageValue as V, MethodCall, MethodCodecError, PlatformException,
        StandardMethodCodec,
    },
};
use serde_json::Value;

type Calls = Rc<RefCell<Vec<(&'static str, Vec<u8>)>>>;

struct Host {
    calls: Calls,
    ready: Rc<Cell<bool>>,
    reply: Option<Vec<u8>>,
}

impl FlutterPlatformMessenger for Host {
    type Error = std::convert::Infallible;

    async fn send_platform_message(
        &mut self,
        channel: &'static str,
        message: Option<Vec<u8>>,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        self.calls
            .borrow_mut()
            .push((channel, message.expect("method payload")));
        poll_fn(|_| {
            if self.ready.get() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        Ok(self.reply.take())
    }
}

fn poll<F: Future>(future: std::pin::Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn completing_services_initialization_sends_one_optional_platform_method() {
    let calls: Calls = Rc::default();
    let ready = Rc::new(Cell::new(false));
    let binding = AppLifecycleBinding::default();
    let mut host = Host {
        calls: calls.clone(),
        ready: ready.clone(),
        reply: Some(JsonMethodCodec::encode_success_envelope(&Value::Null).unwrap()),
    };
    assert!(calls.borrow().is_empty());
    let mut complete = pin!(binding.initialization_complete(&mut host));
    assert!(poll(complete.as_mut()).is_pending());
    {
        let calls = calls.borrow();
        let [(channel, bytes)] = calls.as_slice() else {
            panic!("one initialization call")
        };
        assert_eq!(*channel, "flutter/platform");
        assert_eq!(
            JsonMethodCodec::decode_method_call(bytes).unwrap(),
            MethodCall {
                method: "System.initializationComplete".into(),
                arguments: Value::Null,
            }
        );
    }
    ready.set(true);
    assert!(matches!(poll(complete.as_mut()), Poll::Ready(Ok(()))));
    assert_eq!(calls.borrow().len(), 1);
}

#[test]
fn install_and_uninstall_wait_for_the_engine_and_send_asset_only_keys() {
    for method in ["installDeferredComponent", "uninstallDeferredComponent"] {
        let calls: Calls = Rc::default();
        let ready = Rc::new(Cell::new(false));
        let mut components = DeferredComponent::new(Host {
            calls: calls.clone(),
            ready: ready.clone(),
            reply: Some(StandardMethodCodec::encode_success_envelope(&V::Null).unwrap()),
        });
        let mut request = pin!(async {
            if method == "installDeferredComponent" {
                components.install("testComponentName").await
            } else {
                components.uninstall("testComponentName").await
            }
        });
        assert!(poll(request.as_mut()).is_pending());
        {
            let calls = calls.borrow();
            let [(channel, bytes)] = calls.as_slice() else {
                panic!("one deferred component call")
            };
            assert_eq!(*channel, "flutter/deferredcomponent");
            assert_eq!(
                StandardMethodCodec::decode_method_call(bytes).unwrap(),
                MethodCall {
                    method: method.into(),
                    arguments: V::Map(vec![
                        (V::String("loadingUnitId".into()), V::Int(-1)),
                        (
                            V::String("componentName".into()),
                            V::String("testComponentName".into())
                        ),
                    ]),
                }
            );
        }
        ready.set(true);
        assert!(matches!(poll(request.as_mut()), Poll::Ready(Ok(()))));
        assert_eq!(calls.borrow().len(), 1);
    }
}

#[test]
fn absent_optional_component_host_is_a_noop_but_real_platform_errors_propagate() {
    let error = PlatformException {
        code: "install-failed".into(),
        message: Some("asset split unavailable".into()),
        details: V::Null,
        stacktrace: None,
    };
    for reply in [
        None,
        Some(StandardMethodCodec::encode_error_envelope(&error).unwrap()),
    ] {
        let expected_error = reply.is_some();
        let mut components = DeferredComponent::new(Host {
            calls: Rc::default(),
            ready: Rc::new(Cell::new(true)),
            reply,
        });
        let mut install = pin!(components.install("testComponentName"));
        let Poll::Ready(result) = poll(install.as_mut()) else {
            panic!("host already ready")
        };
        if expected_error {
            assert!(
                matches!(result, Err(DeferredComponentError::Codec(MethodCodecError::Platform(actual))) if *actual == error)
            );
        } else {
            assert!(result.is_ok());
        }
    }
}
