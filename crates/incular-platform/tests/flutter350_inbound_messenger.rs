use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

use incular_platform::{FlutterChannelMicrotask, FlutterInboundMessenger};

#[derive(Clone, Default)]
struct Microtasks(Rc<RefCell<VecDeque<FlutterChannelMicrotask>>>);

impl Microtasks {
    fn schedule(&self, step: FlutterChannelMicrotask) {
        self.0.borrow_mut().push_back(step);
    }

    fn run_one(&self) -> bool {
        let next = self.0.borrow_mut().pop_front();
        if let Some(next) = next {
            next();
            true
        } else {
            false
        }
    }

    fn run_all(&self) {
        while self.run_one() {}
    }

    fn messenger(&self) -> FlutterInboundMessenger {
        let tasks = self.clone();
        FlutterInboundMessenger::new(move |work| tasks.schedule(work))
    }
}

#[test]
fn installing_a_handler_drains_the_buffer_and_replies() {
    let tasks = Microtasks::default();
    let messenger = tasks.messenger();
    let called = Rc::new(Cell::new(false));
    messenger.push("foo", Some(b"bar".to_vec()), {
        let called = called.clone();
        move |_| called.set(true)
    });

    let drained = Rc::new(Cell::new(false));
    messenger.set_message_handler("foo", {
        let drained = drained.clone();
        move |message, reply| {
            assert_eq!(message.as_deref(), Some(b"bar".as_slice()));
            drained.set(true);
            reply(None);
        }
    });
    assert!(!drained.get());
    assert!(!called.get());
    tasks.run_all();
    assert!(drained.get());
    assert!(called.get());
}

#[test]
fn default_binary_messenger_calls_the_callback_once() {
    let tasks = Microtasks::default();
    let messenger = tasks.messenger();
    let inbound = Rc::new(Cell::new(0));
    let outbound = Rc::new(Cell::new(0));
    let bar = b"bar".to_vec();
    messenger.push("foo", Some(bar.clone()), {
        let outbound = outbound.clone();
        move |message| {
            assert!(message.is_none());
            outbound.set(outbound.get() + 1);
        }
    });
    assert_eq!((inbound.get(), outbound.get()), (0, 0));

    messenger.set_message_handler("foo", {
        let (inbound, tasks) = (inbound.clone(), tasks.clone());
        move |message, reply| {
            assert_eq!(message.as_ref(), Some(&bar));
            inbound.set(inbound.get() + 1);
            tasks.schedule(Box::new(move || reply(None)));
        }
    });
    assert_eq!((inbound.get(), outbound.get()), (0, 0));
    assert!(tasks.run_one());
    assert_eq!((inbound.get(), outbound.get()), (1, 0));
    tasks.run_all();
    assert_eq!((inbound.get(), outbound.get()), (1, 1));
    tasks.run_all();
    assert_eq!((inbound.get(), outbound.get()), (1, 1));
}

#[test]
fn overflow_replacement_and_follow_up_turns_preserve_fifo_and_reply_order() {
    let tasks = Microtasks::default();
    let messenger = tasks.messenger();
    let log = Rc::new(RefCell::new(Vec::<(&str, u8)>::new()));
    let push = |channel, id| {
        let log = log.clone();
        messenger.push(channel, Some(vec![id]), move |response| {
            log.borrow_mut().push(match response {
                Some(bytes) => ("response", bytes[0]),
                None => ("dropped", id),
            });
        });
    };

    push("startup", 0);
    push("startup", 1);
    messenger.resize("startup", 3);
    push("startup", 2);
    assert_eq!(*log.borrow(), [("dropped", 0)]);

    let attach = |label| {
        let (log, tasks) = (log.clone(), tasks.clone());
        messenger.set_message_handler("startup", move |data, reply| {
            let data = data.unwrap();
            let id = data[0];
            log.borrow_mut().push((label, id));
            let log = log.clone();
            tasks.schedule(Box::new(move || {
                log.borrow_mut().push(("microtask", id));
                reply(Some(data));
            }));
        });
    };
    attach("old");
    push("startup", 3);
    assert_eq!(*log.borrow(), [("dropped", 0)]);
    assert!(tasks.run_one());
    assert_eq!(*log.borrow(), [("dropped", 0), ("old", 1)]);
    messenger.clear_message_handler("startup");
    assert!(tasks.run_one()); // The first handler's microtask may still reply.
    assert!(tasks.run_one()); // Draining pauses until the new handler exists.
    assert!(!tasks.run_one());
    attach("new");
    tasks.run_all();
    assert_eq!(
        *log.borrow(),
        [
            ("dropped", 0),
            ("old", 1),
            ("microtask", 1),
            ("response", 1),
            ("new", 2),
            ("microtask", 2),
            ("response", 2),
            ("new", 3),
            ("microtask", 3),
            ("response", 3),
        ]
    );
    push("startup", 4);
    assert_eq!(log.borrow().last(), Some(&("new", 4)));
    tasks.run_all();
    assert_eq!(log.borrow().last(), Some(&("response", 4)));
}

#[test]
fn shrinking_drops_oldest_reentrantly_and_preserves_empty_and_null_payloads() {
    let tasks = Microtasks::default();
    let messenger = tasks.messenger();
    messenger.resize("startup", 2);
    let resized = Rc::new(Cell::new(false));
    messenger.push("startup", Some(vec![42]), {
        let (messenger, resized) = (messenger.clone(), resized.clone());
        move |reply| {
            assert!(reply.is_none());
            resized.set(true);
            messenger.push("other", None, |_| {});
        }
    });
    let seen = Rc::new(RefCell::new(Vec::new()));
    messenger.push("startup", Some(vec![]), |_| {});
    messenger.resize("startup", 1);
    assert!(resized.get());
    for channel in ["startup", "other"] {
        messenger.set_message_handler(channel, {
            let seen = seen.clone();
            move |payload, reply| {
                seen.borrow_mut().push(payload);
                reply(None);
            }
        });
    }
    tasks.run_all();
    assert_eq!(*seen.borrow(), [Some(vec![]), None]);
}

fn control(method: &str, name: &str, second: &[u8], extra: bool) -> Vec<u8> {
    let mut bytes = vec![7, u8::try_from(method.len()).unwrap()];
    bytes.extend_from_slice(method.as_bytes());
    bytes.extend([12, if extra { 3 } else { 2 }, 7, u8::try_from(name.len()).unwrap()]);
    bytes.extend_from_slice(name.as_bytes());
    bytes.extend_from_slice(second);
    if extra {
        bytes.extend_from_slice(&[7, 6, b'f', b'u', b't', b'u', b'r', b'e']);
    }
    bytes
}

fn capacity(value: i32) -> Vec<u8> {
    [3].into_iter().chain(value.to_ne_bytes()).collect()
}

#[test]
fn channel_control_resize_applies_both_wire_formats_and_replies_once() {
    let tasks = Microtasks::default();
    let messenger = tasks.messenger();
    let control_replies = Rc::new(Cell::new(0));
    messenger.push(
        FlutterInboundMessenger::CONTROL_CHANNEL_NAME,
        Some(control("resize", "custom", &capacity(3), true)),
        {
            let control_replies = control_replies.clone();
            move |reply| {
                assert!(reply.is_none());
                control_replies.set(control_replies.get() + 1);
            }
        },
    );
    assert_eq!(control_replies.get(), 1);
    let dropped = Rc::new(RefCell::new(Vec::new()));
    for id in 1..=3 {
        let dropped = dropped.clone();
        messenger.push("custom", Some(vec![id]), move |reply| {
            if reply.is_none() {
                dropped.borrow_mut().push(id);
            }
        });
    }
    assert!(dropped.borrow().is_empty());
    messenger.handle_control_message(b"resize\rcustom\r1").unwrap();
    assert_eq!(*dropped.borrow(), [1, 2]);
    let received = Rc::new(RefCell::new(Vec::new()));
    messenger.set_message_handler("custom", {
        let received = received.clone();
        move |payload, reply| {
            received.borrow_mut().extend(payload.unwrap());
            reply(Some(vec![]));
        }
    });
    assert!(received.borrow().is_empty());
    tasks.run_all();
    assert_eq!(*received.borrow(), [3]);
    assert_eq!(*dropped.borrow(), [1, 2]);
}

#[cfg(debug_assertions)]
#[test]
fn channel_control_overflow_changes_only_the_warning_and_not_the_null_reply() {
    use incular_platform::FlutterChannelControlError;

    let tasks = Microtasks::default();
    let messenger = tasks.messenger();
    let warnings = Rc::new(RefCell::new(Vec::new()));
    let dropped = Rc::new(RefCell::new(Vec::new()));
    messenger.on_overflow_warning({
        let warnings = warnings.clone();
        move |channel, capacity| warnings.borrow_mut().push((channel.to_owned(), capacity))
    });
    messenger.handle_control_message(&control("overflow", "custom", &[1], false)).unwrap();
    let push = |id| {
        let dropped = dropped.clone();
        messenger.push("custom", None, move |reply| {
            assert!(reply.is_none());
            dropped.borrow_mut().push(id);
        });
    };
    push(1);
    push(2);
    assert!(warnings.borrow().is_empty());
    assert_eq!(*dropped.borrow(), [1]);
    messenger.handle_control_message(&control("overflow", "custom", &[2], false)).unwrap();
    push(3);
    assert_eq!(*warnings.borrow(), [("custom".to_owned(), 1)]);
    assert_eq!(*dropped.borrow(), [1, 2]);

    let good = control("resize", "custom", &capacity(5), false);
    for size in 0..good.len() {
        assert!(messenger.handle_control_message(&good[..size]).is_err());
    }
    assert_eq!(
        messenger.handle_control_message(&control("resize", "custom", &capacity(-1), false)),
        Err(FlutterChannelControlError::InvalidCapacity),
    );
    assert_eq!(
        messenger.handle_control_message(&control("resize", "bad\0name", &capacity(3), false)),
        Err(FlutterChannelControlError::InvalidChannel),
    );
    push(4);
    assert_eq!(warnings.borrow().len(), 2);
    assert_eq!(*dropped.borrow(), [1, 2, 3], "invalid commands mutate no capacity");
}

#[test]
fn control_reply_is_sent_even_if_a_dropped_message_callback_panics() {
    let tasks = Microtasks::default();
    let messenger = tasks.messenger();
    messenger.push("custom", Some(vec![1]), |reply| {
        assert!(reply.is_none());
        panic!("test callback panic");
    });
    let acknowledged = Rc::new(Cell::new(0));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let acknowledged = acknowledged.clone();
        messenger.push(
            FlutterInboundMessenger::CONTROL_CHANNEL_NAME,
            Some(control("resize", "custom", &capacity(0), false)),
            move |reply| {
                assert!(reply.is_none());
                acknowledged.set(acknowledged.get() + 1);
            },
        );
    }));
    assert!(result.is_err());
    assert_eq!(acknowledged.get(), 1);
    messenger.set_message_handler("custom", |_, _| panic!("the queue should be empty"));
    tasks.run_all();
}
