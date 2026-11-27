//! Buffers messages arriving from platform plugins until a handler exists.

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

/// Nullable binary platform payload. An empty byte buffer is distinct from null.
pub type FlutterBinaryPayload = Option<Vec<u8>>;
/// Reply to one inbound platform message. The responder cannot be called twice.
pub type FlutterBinaryReply = Box<dyn FnOnce(FlutterBinaryPayload)>;
/// A task scheduled after the current call stack has returned.
pub type FlutterChannelMicrotask = Box<dyn FnOnce()>;

pub(crate) fn encode_reply<T, E>(
    reply: FlutterBinaryReply,
    encode: impl FnOnce(T) -> Result<FlutterBinaryPayload, E>,
) -> impl FnOnce(T) -> Result<(), E> {
    move |message| match encode(message) {
        Ok(bytes) => {
            reply(bytes);
            Ok(())
        }
        Err(error) => {
            reply(None);
            Err(error)
        }
    }
}

type Handler = Rc<dyn Fn(FlutterBinaryPayload, FlutterBinaryReply)>;

struct Message {
    payload: FlutterBinaryPayload,
    reply: FlutterBinaryReply,
}

struct Channel {
    capacity: usize,
    pending: VecDeque<Message>,
    handler: Option<Handler>,
    draining: bool,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            capacity: FlutterInboundMessenger::DEFAULT_CAPACITY,
            pending: VecDeque::new(),
            handler: None,
            draining: false,
        }
    }
}

struct Inner {
    channels: RefCell<HashMap<Rc<str>, Channel>>,
    schedule: Box<dyn Fn(FlutterChannelMicrotask)>,
}

/// Local receiver for Flutter-compatible platform messages. Outbound calls
/// use the separate [crate::FlutterPlatformMessenger] transport.
///
/// A host supplies the local microtask scheduler. Before a handler is installed
/// a channel keeps its newest messages up to its capacity. Installing a handler
/// drains queued messages in FIFO order, one message per microtask. Direct
/// delivery resumes once the drain has finished.
#[derive(Clone)]
pub struct FlutterInboundMessenger(Rc<Inner>);

impl FlutterInboundMessenger {
    pub const DEFAULT_CAPACITY: usize = 1;

    /// `schedule` must enqueue, not run the task inline. A handler can reply
    /// later by moving its one-shot responder into its own local task.
    pub fn new(schedule: impl Fn(FlutterChannelMicrotask) + 'static) -> Self {
        Self(Rc::new(Inner {
            channels: RefCell::default(),
            schedule: Box::new(schedule),
        }))
    }

    /// Installs or replaces one channel's inbound handler.
    pub fn set_message_handler(
        &self,
        name: &str,
        handler: impl Fn(FlutterBinaryPayload, FlutterBinaryReply) + 'static,
    ) {
        let (next_drain, replaced) = {
            let mut channels = self.0.channels.borrow_mut();
            let key = channel_key(&channels, name);
            let channel = channels.entry(key.clone()).or_default();
            let replaced = channel.handler.replace(Rc::new(handler));
            let start_drain = replaced.is_none() && !channel.draining;
            if start_drain {
                channel.draining = true;
            }
            (start_drain.then_some(key), replaced)
        };
        drop(replaced);
        if let Some(key) = next_drain {
            self.schedule_drain(key);
        }
    }

    /// Removes the handler. Pending messages stay queued up to capacity.
    pub fn clear_message_handler(&self, name: &str) {
        let removed = self
            .0
            .channels
            .borrow_mut()
            .get_mut(name)
            .and_then(|channel| channel.handler.take());
        drop(removed);
    }

    /// Sets the number of messages to keep without a handler or during a
    /// drain. Messages removed from the front receive a null reply.
    pub fn resize(&self, name: &str, capacity: usize) {
        let dropped = {
            let mut channels = self.0.channels.borrow_mut();
            let key = channel_key(&channels, name);
            let channel = channels.entry(key).or_default();
            channel.capacity = capacity;
            let overflow = channel.pending.len().saturating_sub(capacity);
            channel.pending.drain(..overflow).collect::<Vec<_>>()
        };
        for message in dropped {
            (message.reply)(None);
        }
    }

    /// Receives a platform message. An already listening channel dispatches
    /// immediately unless older buffered messages are draining. At zero
    /// capacity, a message that cannot be delivered is discarded without a reply.
    pub fn push(
        &self,
        name: &str,
        payload: FlutterBinaryPayload,
        reply: impl FnOnce(FlutterBinaryPayload) + 'static,
    ) {
        let message = Message {
            payload,
            reply: Box::new(reply),
        };
        let dropped = {
            let mut channels = self.0.channels.borrow_mut();
            let key = channel_key(&channels, name);
            let channel = channels.entry(key).or_default();
            if !channel.draining
                && let Some(handler) = channel.handler.clone()
            {
                drop(channels);
                handler(message.payload, message.reply);
                return;
            }
            if channel.capacity == 0 {
                return;
            }
            let oldest = if channel.pending.len() == channel.capacity {
                channel.pending.pop_front()
            } else {
                None
            };
            channel.pending.push_back(message);
            oldest
        };
        if let Some(oldest) = dropped {
            (oldest.reply)(None);
        }
    }

    fn schedule_drain(&self, name: Rc<str>) {
        let weak = Rc::downgrade(&self.0);
        (self.0.schedule)(Box::new(move || {
            if let Some(inner) = weak.upgrade() {
                Self(inner).drain_one(name);
            }
        }));
    }

    fn drain_one(&self, name: Rc<str>) {
        let next = {
            // A draining channel remains in the registry, even if cleared.
            let mut channels = self.0.channels.borrow_mut();
            let channel = channels.get_mut(&*name).expect("registered drain channel");
            if let Some(handler) = channel.handler.clone()
                && let Some(message) = channel.pending.pop_front()
            {
                Some((handler, message))
            } else {
                channel.draining = false;
                None
            }
        };
        if let Some((handler, message)) = next {
            // Queue the next turn after handler-scheduled microtasks, even if
            // a host catches a handler panic outside this receiver.
            let _continue_drain = DrainContinuation {
                messenger: self,
                name,
            };
            handler(message.payload, message.reply);
        }
    }
}

struct DrainContinuation<'a> {
    messenger: &'a FlutterInboundMessenger,
    name: Rc<str>,
}

impl Drop for DrainContinuation<'_> {
    fn drop(&mut self) {
        self.messenger.schedule_drain(self.name.clone());
    }
}

fn channel_key(channels: &HashMap<Rc<str>, Channel>, name: &str) -> Rc<str> {
    channels
        .get_key_value(name)
        .map_or_else(|| Rc::from(name), |(key, _)| key.clone())
}
