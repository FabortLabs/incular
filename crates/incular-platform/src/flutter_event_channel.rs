//! JSON platform event channels with Flutter's first/last-listener lifecycle.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    future::Future,
    pin::Pin,
    rc::Rc,
};

use serde_json::Value;

use crate::{
    FlutterInboundMessenger, FlutterJsonMethodChannel, FlutterPlatformChannelError,
    FlutterPlatformMessenger,
    message_codecs::{JsonMethodCodec, JsonMethodCodecError, PlatformException},
};

pub type FlutterEventTask = Pin<Box<dyn Future<Output = ()>>>;

/// One decoded platform event. A success envelope containing null is Data;
/// a null binary message is Done.
#[derive(Clone, Debug, PartialEq)]
pub enum FlutterJsonStreamEvent {
    Data(Value),
    Error(PlatformException<Value>),
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlutterEventOperation {
    Listen,
    Cancel,
    Decode,
}

impl FlutterEventOperation {
    fn method(self) -> &'static str {
        match self {
            Self::Listen => "listen",
            Self::Cancel => "cancel",
            Self::Decode => unreachable!("decoding is not an outbound request"),
        }
    }
}

/// JSON event channel for a cloneable local transport. The host schedules
/// tasks on the same FIFO event loop as the inbound messenger's microtasks.
pub struct FlutterJsonEventChannel<M: FlutterPlatformMessenger> {
    name: &'static str,
    inbound: FlutterInboundMessenger,
    transport: M,
    schedule: Rc<dyn Fn(FlutterEventTask)>,
    report: Rc<ErrorReporter<M::Error>>,
}

type ErrorReporter<E> = dyn Fn(FlutterEventOperation, FlutterPlatformChannelError<E>);
type Listener = Rc<dyn Fn(FlutterJsonStreamEvent)>;

impl<M> FlutterJsonEventChannel<M>
where
    M: FlutterPlatformMessenger + Clone + 'static,
{
    pub fn new(
        name: &'static str,
        inbound: FlutterInboundMessenger,
        transport: M,
        schedule: impl Fn(FlutterEventTask) + 'static,
        report: impl Fn(FlutterEventOperation, FlutterPlatformChannelError<M::Error>) + 'static,
    ) -> Self {
        Self {
            name,
            inbound,
            transport,
            schedule: Rc::new(schedule),
            report: Rc::new(report),
        }
    }

    pub fn receive_broadcast_stream(&self, arguments: Value) -> FlutterJsonBroadcast<M> {
        FlutterJsonBroadcast(Rc::new(Broadcast {
            name: self.name,
            inbound: self.inbound.clone(),
            transport: self.transport.clone(),
            arguments,
            schedule: self.schedule.clone(),
            report: self.report.clone(),
            listeners: RefCell::new(BTreeMap::new()),
            next_listener: Cell::new(0),
            active: Cell::new(false),
            closed: Cell::new(false),
            operations: RefCell::new(VecDeque::new()),
            running_operations: Cell::new(false),
        }))
    }
}

pub struct FlutterJsonBroadcast<M: FlutterPlatformMessenger>(Rc<Broadcast<M>>);

impl<M: FlutterPlatformMessenger> Clone for FlutterJsonBroadcast<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<M> FlutterJsonBroadcast<M>
where
    M: FlutterPlatformMessenger + Clone + 'static,
{
    pub fn subscribe(
        &self,
        listener: impl Fn(FlutterJsonStreamEvent) + 'static,
    ) -> FlutterEventSubscription {
        let state = &self.0;
        if state.closed.get() {
            let live = Rc::new(Cell::new(true));
            let pending = live.clone();
            (state.schedule)(Box::pin(async move {
                if pending.get() {
                    listener(FlutterJsonStreamEvent::Done);
                }
            }));
            return FlutterEventSubscription(Some(Box::new(move || live.set(false))));
        }

        let id = state.next_listener.get();
        state
            .next_listener
            .set(id.checked_add(1).expect("too many event listeners"));
        state.listeners.borrow_mut().insert(id, Rc::new(listener));
        state.activate();
        let state = state.clone();
        FlutterEventSubscription(Some(Box::new(move || {
            let removed = state.listeners.borrow_mut().remove(&id);
            drop(removed);
            if state.listeners.borrow().is_empty() {
                state.deactivate();
            }
        })))
    }
}

/// Dropping a subscription removes its callback and cancels the platform
/// stream if this was the last subscriber.
#[must_use = "keep the subscription alive to receive platform events"]
pub struct FlutterEventSubscription(Option<Box<dyn FnOnce()>>);

impl Drop for FlutterEventSubscription {
    fn drop(&mut self) {
        if let Some(cancel) = self.0.take() {
            cancel();
        }
    }
}

struct Broadcast<M: FlutterPlatformMessenger> {
    name: &'static str,
    inbound: FlutterInboundMessenger,
    transport: M,
    arguments: Value,
    schedule: Rc<dyn Fn(FlutterEventTask)>,
    report: Rc<ErrorReporter<M::Error>>,
    listeners: RefCell<BTreeMap<u64, Listener>>,
    next_listener: Cell<u64>,
    active: Cell<bool>,
    closed: Cell<bool>,
    operations: RefCell<VecDeque<FlutterEventOperation>>,
    running_operations: Cell<bool>,
}

impl<M> Broadcast<M>
where
    M: FlutterPlatformMessenger + Clone + 'static,
{
    fn activate(self: &Rc<Self>) {
        if self.active.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        self.inbound
            .set_message_handler(self.name, move |payload, reply| {
                if let Some(state) = weak.upgrade() {
                    state.receive(payload.as_deref());
                }
                reply(None);
            });
        self.queue_control(FlutterEventOperation::Listen);
    }

    fn deactivate(self: &Rc<Self>) {
        if !self.active.replace(false) {
            return;
        }
        self.inbound.clear_message_handler(self.name);
        self.queue_control(FlutterEventOperation::Cancel);
    }

    fn receive(self: &Rc<Self>, payload: Option<&[u8]>) {
        if self.closed.get() || !self.active.get() {
            return;
        }
        let event = match payload.map(JsonMethodCodec::decode_envelope) {
            None => {
                self.closed.set(true);
                FlutterJsonStreamEvent::Done
            }
            Some(Ok(value)) => FlutterJsonStreamEvent::Data(value),
            Some(Err(JsonMethodCodecError::Platform(error))) => {
                FlutterJsonStreamEvent::Error(*error)
            }
            Some(Err(error)) => {
                (self.report)(FlutterEventOperation::Decode, error.into());
                return;
            }
        };
        let recipients: Vec<_> = self.listeners.borrow().keys().copied().collect();
        let state = self.clone();
        (self.schedule)(Box::pin(async move {
            for id in recipients {
                let listener = state.listeners.borrow().get(&id).cloned();
                if let Some(listener) = listener {
                    listener(event.clone());
                }
            }
            if event == FlutterJsonStreamEvent::Done {
                let removed = std::mem::take(&mut *state.listeners.borrow_mut());
                drop(removed);
                state.deactivate();
            }
        }));
    }

    fn queue_control(self: &Rc<Self>, operation: FlutterEventOperation) {
        self.operations.borrow_mut().push_back(operation);
        if self.running_operations.replace(true) {
            return;
        }
        let state = self.clone();
        (self.schedule)(Box::pin(async move {
            let mut transport = state.transport.clone();
            loop {
                let operation = state.operations.borrow_mut().pop_front();
                let Some(operation) = operation else { break };
                let channel = FlutterJsonMethodChannel::new(state.name);
                if let Err(error) = channel
                    .invoke(&mut transport, operation.method(), state.arguments.clone())
                    .await
                {
                    (state.report)(operation, error);
                }
            }
            state.running_operations.set(false);
        }));
    }
}
