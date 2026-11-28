use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::{Future, poll_fn},
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use incular_platform::{
    AppLifecycleState, FlutterPlatformMessenger, message_codecs::JsonMethodCodec,
};
use incular_widgets::{ClipboardStatus::*, ClipboardStatusNotifier, ClipboardStatusTask};

#[derive(Clone, Default)]
struct Host {
    replies: Rc<RefCell<VecDeque<Option<Vec<u8>>>>>,
    requests: Rc<Cell<usize>>,
    hold: Rc<Cell<bool>>,
}

impl Host {
    fn reply(&self, response: &[u8]) {
        self.replies.borrow_mut().push_back(Some(response.into()));
    }
}

impl FlutterPlatformMessenger for Host {
    type Error = &'static str;

    async fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        let call = JsonMethodCodec::decode_method_call(&payload.unwrap()).unwrap();
        assert_eq!(channel, "flutter/platform");
        assert_eq!(call.method, "Clipboard.hasStrings");
        assert_eq!(call.arguments, "text/plain");
        self.requests.set(self.requests.get() + 1);
        poll_fn(|_| {
            if self.hold.get() {
                Poll::Pending
            } else {
                Poll::Ready(Ok(self
                    .replies
                    .borrow_mut()
                    .pop_front()
                    .expect("scripted reply")))
            }
        })
        .await
    }
}

type Tasks = Rc<RefCell<VecDeque<ClipboardStatusTask>>>;
type Errors = Rc<RefCell<Vec<String>>>;

fn mount(host: &Host) -> (ClipboardStatusNotifier, Tasks, Errors) {
    let tasks: Tasks = Rc::default();
    let errors: Errors = Rc::default();
    let notifier = ClipboardStatusNotifier::new(
        Unknown,
        host.clone(),
        {
            let tasks = tasks.clone();
            move |task| tasks.borrow_mut().push_back(task)
        },
        {
            let errors = errors.clone();
            move |error| errors.borrow_mut().push(error.to_owned())
        },
    );
    (notifier, tasks, errors)
}

fn complete(future: impl Future<Output = ()>) {
    assert_eq!(
        pin!(future).poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(())
    );
}

#[test]
fn an_unsuccessful_clipboard_api_completes_and_leaves_status_unknown() {
    let host = Host::default();
    let (notifier, _, errors) = mount(&host);
    assert_eq!(notifier.value(), Unknown);
    host.reply(br#"["clipboard","permission denied",null]"#);
    complete(notifier.update());
    assert_eq!(notifier.value(), Unknown);
    assert_eq!(host.requests.get(), 1);
    assert_eq!(errors.borrow().len(), 1);
    assert!(errors.borrow()[0].contains("permission denied"));

    host.reply(br#"[{"value":true}]"#);
    complete(notifier.update());
    assert_eq!(notifier.value(), Pasteable);
    host.replies.borrow_mut().push_back(None);
    complete(notifier.update());
    assert_eq!(notifier.value(), Unknown);
    assert_eq!(errors.borrow().len(), 2);
}

#[test]
fn refresh_sets_status_from_actual_clipboard_presence_and_deduplicates_equal_replies() {
    let host = Host::default();
    let (notifier, tasks, errors) = mount(&host);
    assert_eq!(notifier.value(), Unknown);
    host.reply(br#"[{"value":false}]"#);
    complete(notifier.update());
    assert_eq!(notifier.value(), NotPasteable);

    let count = Rc::new(Cell::new(0));
    let _subscription = notifier.add_listener({
        let count = count.clone();
        move || count.set(count.get() + 1)
    });
    assert!(tasks.borrow().is_empty());
    host.reply(br#"[{"value":true}]"#);
    complete(notifier.update());
    assert_eq!(notifier.value(), Pasteable);
    assert_eq!(count.get(), 1);

    host.reply(br#"[{"value":true}]"#);
    complete(notifier.update());
    assert_eq!(count.get(), 1);
    assert!(errors.borrow().is_empty());
}

#[test]
fn resume_refreshes_for_active_listeners_and_disposal_ignores_an_in_flight_reply() {
    let host = Host::default();
    let (notifier, tasks, _) = mount(&host);
    notifier.did_change_app_lifecycle_state(AppLifecycleState::Resumed);
    assert!(tasks.borrow().is_empty());
    let count = Rc::new(Cell::new(0));
    let subscription = notifier.add_listener({
        let count = count.clone();
        move || count.set(count.get() + 1)
    });
    host.reply(br#"[null]"#);
    let initial = tasks.borrow_mut().pop_front().unwrap();
    complete(initial);
    assert_eq!(notifier.value(), NotPasteable);
    assert_eq!(count.get(), 1);

    notifier.did_change_app_lifecycle_state(AppLifecycleState::Inactive);
    assert!(tasks.borrow().is_empty());
    notifier.did_change_app_lifecycle_state(AppLifecycleState::Resumed);
    let mut scheduled = tasks.borrow_mut().pop_front().unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    host.hold.set(true);
    assert_eq!(scheduled.as_mut().poll(&mut cx), Poll::Pending);
    assert_eq!(host.requests.get(), 2);
    drop(subscription);
    notifier.did_change_app_lifecycle_state(AppLifecycleState::Resumed);
    assert!(tasks.borrow().is_empty());
    notifier.dispose();
    host.hold.set(false);
    host.reply(br#"[{"value":true}]"#);
    assert_eq!(scheduled.as_mut().poll(&mut cx), Poll::Ready(()));
    assert_eq!(notifier.value(), NotPasteable);
    assert_eq!(count.get(), 1);
    complete(notifier.update());
    assert_eq!(host.requests.get(), 2);
}
