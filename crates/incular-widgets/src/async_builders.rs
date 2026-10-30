//! Flutter's `FutureBuilder`, `StreamBuilder` and `StreamBuilderBase`
//! (`widgets/async.dart`).
//!
//! Widgets own no executor, so the asynchronous sources are UI-thread values
//! in the style of Dart's: a [`UiFuture`] completed through its
//! [`UiCompleter`], and a [`UiStream`] fed by its [`UiStreamController`].
//! Work running elsewhere completes them from a UI-thread callback, such as
//! the runtime's `spawn_into` completion. A builder that reads a source
//! tracks it, so a completion or event rebuilds it on the next frame.

use std::{
    cell::{Cell, RefCell},
    fmt,
    rc::{Rc, Weak},
};

use incular_core::reactivity::DependencySource;

use crate::tree::RetainedState;
use crate::{BuildContext, Widget};

/// Flutter's `ConnectionState`: the state of a connection to an
/// asynchronous computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConnectionState {
    /// Not connected, such as a builder with no future or stream.
    None,
    /// Connected and waiting for the first result.
    Waiting,
    /// A stream that has delivered at least one event and is still open.
    Active,
    /// The computation finished.
    Done,
}

/// Flutter's `AsyncSnapshot`: the latest interaction with a future or
/// stream. Data and error are exclusive except across a reconnection, where
/// [`AsyncSnapshot::in_state`] keeps the previous one.
#[derive(Clone, Debug, PartialEq)]
pub struct AsyncSnapshot<T, E> {
    pub connection_state: ConnectionState,
    pub data: Option<T>,
    pub error: Option<E>,
}

impl<T, E> AsyncSnapshot<T, E> {
    /// `AsyncSnapshot.nothing()`: no connection, data or error.
    #[must_use]
    pub const fn nothing() -> Self {
        Self {
            connection_state: ConnectionState::None,
            data: None,
            error: None,
        }
    }

    /// `AsyncSnapshot.waiting()`.
    #[must_use]
    pub const fn waiting() -> Self {
        Self {
            connection_state: ConnectionState::Waiting,
            data: None,
            error: None,
        }
    }

    #[must_use]
    pub const fn with_data(connection_state: ConnectionState, data: T) -> Self {
        Self {
            connection_state,
            data: Some(data),
            error: None,
        }
    }

    #[must_use]
    pub const fn with_error(connection_state: ConnectionState, error: E) -> Self {
        Self {
            connection_state,
            data: None,
            error: Some(error),
        }
    }

    /// The same data or error in another connection state.
    #[must_use]
    pub fn in_state(self, connection_state: ConnectionState) -> Self {
        Self {
            connection_state,
            ..self
        }
    }

    #[must_use]
    pub const fn has_data(&self) -> bool {
        self.data.is_some()
    }

    #[must_use]
    pub const fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

struct FutureCell<T, E> {
    outcome: RefCell<Option<Result<T, E>>>,
    changes: DependencySource,
}

/// A value that becomes available later on the UI thread, like Dart's
/// `Future`. Clones share one completion; two futures are the same future
/// only when one was cloned from the other.
pub struct UiFuture<T, E> {
    cell: Rc<FutureCell<T, E>>,
}

impl<T, E> Clone for UiFuture<T, E> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T, E> PartialEq for UiFuture<T, E> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cell, &other.cell)
    }
}

impl<T, E> fmt::Debug for UiFuture<T, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UiFuture")
            .field("completed", &self.cell.outcome.borrow().is_some())
            .finish()
    }
}

impl<T: Clone, E: Clone> UiFuture<T, E> {
    /// A pending future and the completer that finishes it.
    #[must_use]
    pub fn pending() -> (Self, UiCompleter<T, E>) {
        let future = Self {
            cell: Rc::new(FutureCell {
                outcome: RefCell::new(None),
                changes: DependencySource::default(),
            }),
        };
        let completer = UiCompleter {
            future: future.clone(),
        };
        (future, completer)
    }

    /// A future that already completed with `value`.
    #[must_use]
    pub fn value(value: T) -> Self {
        let (future, completer) = Self::pending();
        completer.complete(value);
        future
    }

    /// A future that already completed with `error`.
    #[must_use]
    pub fn error(error: E) -> Self {
        let (future, completer) = Self::pending();
        completer.complete_error(error);
        future
    }

    /// The outcome, if the future completed. Reading it inside a build
    /// makes that build depend on the completion.
    #[must_use]
    pub fn outcome(&self) -> Option<Result<T, E>> {
        self.cell.changes.track();
        self.cell.outcome.borrow().clone()
    }

    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.cell.outcome.borrow().is_some()
    }
}

/// Completes a [`UiFuture`], like Dart's `Completer`. A future completes
/// once; later completions are ignored.
pub struct UiCompleter<T, E> {
    future: UiFuture<T, E>,
}

impl<T: Clone, E: Clone> UiCompleter<T, E> {
    #[must_use]
    pub fn future(&self) -> UiFuture<T, E> {
        self.future.clone()
    }

    /// Returns false when the future had already completed.
    pub fn complete(&self, value: T) -> bool {
        self.finish(Ok(value))
    }

    /// Returns false when the future had already completed.
    pub fn complete_error(&self, error: E) -> bool {
        self.finish(Err(error))
    }

    fn finish(&self, outcome: Result<T, E>) -> bool {
        let cell = &self.future.cell;
        if cell.outcome.borrow().is_some() {
            return false;
        }
        *cell.outcome.borrow_mut() = Some(outcome);
        cell.changes.notify();
        true
    }
}

/// Flutter's `FutureBuilder`: builds from the latest snapshot of `future`.
///
/// As in Flutter, the snapshot is waiting until the future completes, then
/// done with its data or error. Replacing the future reconnects, keeping the
/// previous data or error in the waiting snapshot. A future that already
/// completed shows its result on the first build, as Flutter does for a
/// `SynchronousFuture`.
pub struct FutureBuilder<T, E> {
    future: Option<UiFuture<T, E>>,
    initial_data: Option<T>,
    builder: AsyncWidgetBuilder<T, E>,
}

/// Builds a widget from the latest [`AsyncSnapshot`].
pub type AsyncWidgetBuilder<T, E> =
    Rc<dyn for<'a> Fn(&BuildContext<'a>, &AsyncSnapshot<T, E>) -> Widget>;

impl<T: Clone + 'static, E: Clone + 'static> FutureBuilder<T, E> {
    #[must_use]
    pub fn new<W: Into<Widget>>(
        future: Option<UiFuture<T, E>>,
        builder: impl for<'a> Fn(&BuildContext<'a>, &AsyncSnapshot<T, E>) -> W + 'static,
    ) -> Self {
        Self {
            future,
            initial_data: None,
            builder: Rc::new(move |context, snapshot| builder(context, snapshot).into()),
        }
    }

    /// The data of the first snapshot, before the future completes.
    #[must_use]
    pub fn initial_data(mut self, data: T) -> Self {
        self.initial_data = Some(data);
        self
    }
}

struct FutureBuilderState<T, E> {
    /// The future the snapshot follows, Flutter's `_activeCallbackIdentity`.
    future: RefCell<Option<UiFuture<T, E>>>,
    snapshot: RefCell<Option<AsyncSnapshot<T, E>>>,
}

impl<T: 'static, E: 'static> RetainedState for FutureBuilderState<T, E> {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl<T: Clone + 'static, E: Clone + 'static> FutureBuilderState<T, E> {
    fn snapshot(&self, config: &FutureBuilder<T, E>) -> AsyncSnapshot<T, E> {
        let mut snapshot = self.snapshot.borrow_mut();
        let current = snapshot.take().unwrap_or_else(|| {
            // `initState`.
            config
                .initial_data
                .clone()
                .map_or_else(AsyncSnapshot::nothing, |data| {
                    AsyncSnapshot::with_data(ConnectionState::None, data)
                })
        });
        let mut followed = self.future.borrow_mut();
        let mut current = if *followed == config.future {
            current
        } else {
            // `didUpdateWidget` with a new future: `_unsubscribe` and
            // `_subscribe`.
            let current = if followed.is_some() {
                current.in_state(ConnectionState::None)
            } else {
                current
            };
            followed.clone_from(&config.future);
            match &config.future {
                Some(_) if current.connection_state != ConnectionState::Done => {
                    current.in_state(ConnectionState::Waiting)
                }
                _ => current,
            }
        };
        if let Some(future) = &config.future
            && current.connection_state == ConnectionState::Waiting
        {
            match future.outcome() {
                Some(Ok(data)) => current = AsyncSnapshot::with_data(ConnectionState::Done, data),
                Some(Err(error)) => {
                    current = AsyncSnapshot::with_error(ConnectionState::Done, error);
                }
                None => {}
            }
        }
        *snapshot = Some(current.clone());
        current
    }
}

impl<T: Clone + 'static, E: Clone + 'static> From<FutureBuilder<T, E>> for Widget {
    fn from(value: FutureBuilder<T, E>) -> Self {
        let config = Rc::new(value);
        Widget::retained_builder(
            |_| FutureBuilderState::<T, E> {
                future: RefCell::new(None),
                snapshot: RefCell::new(None),
            },
            move |state, context, _| {
                let snapshot = state.snapshot(&config);
                (config.builder)(context, &snapshot)
            },
            false,
        )
    }
}

/// One stream event, as Dart's `StreamSubscription` callbacks see it.
#[derive(Clone, Debug, PartialEq)]
pub enum StreamEvent<T, E> {
    Data(T),
    Error(E),
    Done,
}

type StreamListener<T, E> = dyn Fn(&StreamEvent<T, E>);

struct StreamCell<T, E> {
    listeners: RefCell<Vec<Weak<StreamListener<T, E>>>>,
    closed: Cell<bool>,
}

/// A sequence of UI-thread events, like a Dart broadcast `Stream`: events go
/// to the listeners present when they are added and are not buffered.
pub struct UiStream<T, E> {
    cell: Rc<StreamCell<T, E>>,
}

impl<T, E> Clone for UiStream<T, E> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T, E> PartialEq for UiStream<T, E> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cell, &other.cell)
    }
}

impl<T, E> fmt::Debug for UiStream<T, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UiStream")
            .field("closed", &self.cell.closed.get())
            .finish()
    }
}

/// Keeps a [`UiStream`] listener registered until dropped, Dart's
/// `StreamSubscription.cancel` on drop.
#[must_use = "dropping the subscription stops listening"]
pub struct UiStreamSubscription<T, E> {
    _listener: Rc<StreamListener<T, E>>,
}

impl<T, E> UiStream<T, E> {
    /// Listens until the subscription is dropped. A closed stream delivers
    /// [`StreamEvent::Done`] at once.
    pub fn listen(
        &self,
        listener: impl Fn(&StreamEvent<T, E>) + 'static,
    ) -> UiStreamSubscription<T, E> {
        let listener: Rc<StreamListener<T, E>> = Rc::new(listener);
        if self.cell.closed.get() {
            listener(&StreamEvent::Done);
        } else {
            self.cell
                .listeners
                .borrow_mut()
                .push(Rc::downgrade(&listener));
        }
        UiStreamSubscription {
            _listener: listener,
        }
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.cell.closed.get()
    }
}

/// Adds events to a [`UiStream`], like Dart's `StreamController.broadcast`.
pub struct UiStreamController<T, E> {
    stream: UiStream<T, E>,
}

impl<T, E> Default for UiStreamController<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E> UiStreamController<T, E> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            stream: UiStream {
                cell: Rc::new(StreamCell {
                    listeners: RefCell::new(Vec::new()),
                    closed: Cell::new(false),
                }),
            },
        }
    }

    #[must_use]
    pub fn stream(&self) -> UiStream<T, E> {
        self.stream.clone()
    }

    pub fn add(&self, data: T) {
        self.dispatch(&StreamEvent::Data(data));
    }

    pub fn add_error(&self, error: E) {
        self.dispatch(&StreamEvent::Error(error));
    }

    /// Delivers [`StreamEvent::Done`] and drops every listener. Later events
    /// are ignored.
    pub fn close(&self) {
        self.dispatch(&StreamEvent::Done);
        self.stream.cell.closed.set(true);
        self.stream.cell.listeners.borrow_mut().clear();
    }

    fn dispatch(&self, event: &StreamEvent<T, E>) {
        if self.stream.cell.closed.get() {
            return;
        }
        let listeners: Vec<_> = {
            let mut listeners = self.stream.cell.listeners.borrow_mut();
            listeners.retain(|listener| listener.strong_count() > 0);
            listeners.iter().filter_map(Weak::upgrade).collect()
        };
        for listener in listeners {
            listener(event);
        }
    }
}

/// Flutter's `StreamBuilderBase`: folds the interaction with a stream into
/// a summary and builds from it.
///
/// Turn an implementation into a widget with
/// [`StreamBuilderBase::into_widget`].
pub trait StreamBuilderBase: Sized + 'static {
    type Data: 'static;
    type Error: 'static;
    type Summary: Clone + 'static;

    /// The stream to follow. Replacing it with another reconnects.
    fn stream(&self) -> Option<UiStream<Self::Data, Self::Error>>;

    /// The summary before any interaction.
    fn initial(&self) -> Self::Summary;

    /// The stream was subscribed to.
    fn after_connected(&self, current: Self::Summary) -> Self::Summary {
        current
    }

    fn after_data(&self, current: Self::Summary, data: Self::Data) -> Self::Summary;

    fn after_error(&self, current: Self::Summary, _error: Self::Error) -> Self::Summary {
        current
    }

    /// The stream closed.
    fn after_done(&self, current: Self::Summary) -> Self::Summary {
        current
    }

    /// The builder stopped following a stream.
    fn after_disconnected(&self, current: Self::Summary) -> Self::Summary {
        current
    }

    fn build(&self, context: &BuildContext<'_>, summary: &Self::Summary) -> Widget;

    /// The widget that follows [`StreamBuilderBase::stream`].
    fn into_widget(self) -> Widget
    where
        Self::Data: Clone,
        Self::Error: Clone,
    {
        stream_builder_widget(self)
    }
}

struct StreamBuilderState<B: StreamBuilderBase> {
    config: RefCell<Option<Rc<B>>>,
    stream: RefCell<Option<UiStream<B::Data, B::Error>>>,
    subscription: RefCell<Option<UiStreamSubscription<B::Data, B::Error>>>,
    summary: RefCell<Option<B::Summary>>,
    changes: DependencySource,
}

impl<B: StreamBuilderBase> RetainedState for StreamBuilderState<B> {
    fn rebind_revision(&self, _revision: Rc<Cell<u64>>) {}
}

impl<B: StreamBuilderBase> StreamBuilderState<B>
where
    B::Data: Clone,
    B::Error: Clone,
{
    fn fold(&self, step: impl FnOnce(&B, B::Summary) -> B::Summary) {
        let config = self
            .config
            .borrow()
            .clone()
            .expect("the stream builder is configured before it listens");
        let mut summary = self.summary.borrow_mut();
        let current = summary.take().expect("the summary starts at `initial`");
        *summary = Some(step(&config, current));
        drop(summary);
        self.changes.notify();
    }

    /// `initState` and `didUpdateWidget`.
    fn update(self: &Rc<Self>, config: &Rc<B>) {
        *self.config.borrow_mut() = Some(config.clone());
        let first = self.summary.borrow().is_none();
        if first {
            *self.summary.borrow_mut() = Some(config.initial());
        }
        let stream = config.stream();
        if !first && *self.stream.borrow() == stream {
            return;
        }
        // `_unsubscribe`, then `_subscribe`.
        if self.subscription.borrow_mut().take().is_some() {
            self.fold(B::after_disconnected);
        }
        self.stream.borrow_mut().clone_from(&stream);
        if let Some(stream) = stream {
            self.fold(B::after_connected);
            let weak = Rc::downgrade(self);
            let subscription = stream.listen(move |event| {
                let Some(state) = weak.upgrade() else {
                    return;
                };
                match event {
                    StreamEvent::Data(data) => {
                        state.fold(|config, current| config.after_data(current, data.clone()));
                    }
                    StreamEvent::Error(error) => {
                        state.fold(|config, current| config.after_error(current, error.clone()));
                    }
                    StreamEvent::Done => state.fold(B::after_done),
                }
            });
            *self.subscription.borrow_mut() = Some(subscription);
        }
    }
}

fn stream_builder_widget<B: StreamBuilderBase>(widget: B) -> Widget
where
    B::Data: Clone,
    B::Error: Clone,
{
    let config = Rc::new(widget);
    Widget::retained_builder(
        |_| StreamBuilderState::<B> {
            config: RefCell::new(None),
            stream: RefCell::new(None),
            subscription: RefCell::new(None),
            summary: RefCell::new(None),
            changes: DependencySource::default(),
        },
        move |state, context, _| {
            state.update(&config);
            state.changes.track();
            let summary = state.summary.borrow().clone().expect("summary");
            config.build(context, &summary)
        },
        false,
    )
}

/// Flutter's `StreamBuilder`: builds from the latest [`AsyncSnapshot`] of
/// `stream`. The snapshot is waiting once connected, active with each data
/// or error event, and done when the stream closes.
pub struct StreamBuilder<T, E> {
    stream: Option<UiStream<T, E>>,
    initial_data: Option<T>,
    builder: AsyncWidgetBuilder<T, E>,
}

impl<T: Clone + 'static, E: Clone + 'static> StreamBuilder<T, E> {
    #[must_use]
    pub fn new<W: Into<Widget>>(
        stream: Option<UiStream<T, E>>,
        builder: impl for<'a> Fn(&BuildContext<'a>, &AsyncSnapshot<T, E>) -> W + 'static,
    ) -> Self {
        Self {
            stream,
            initial_data: None,
            builder: Rc::new(move |context, snapshot| builder(context, snapshot).into()),
        }
    }

    /// The data of the first snapshot, before any event.
    #[must_use]
    pub fn initial_data(mut self, data: T) -> Self {
        self.initial_data = Some(data);
        self
    }
}

impl<T: Clone + 'static, E: Clone + 'static> StreamBuilderBase for StreamBuilder<T, E> {
    type Data = T;
    type Error = E;
    type Summary = AsyncSnapshot<T, E>;

    fn stream(&self) -> Option<UiStream<T, E>> {
        self.stream.clone()
    }

    fn initial(&self) -> AsyncSnapshot<T, E> {
        self.initial_data
            .clone()
            .map_or_else(AsyncSnapshot::nothing, |data| {
                AsyncSnapshot::with_data(ConnectionState::None, data)
            })
    }

    fn after_connected(&self, current: AsyncSnapshot<T, E>) -> AsyncSnapshot<T, E> {
        current.in_state(ConnectionState::Waiting)
    }

    fn after_data(&self, _: AsyncSnapshot<T, E>, data: T) -> AsyncSnapshot<T, E> {
        AsyncSnapshot::with_data(ConnectionState::Active, data)
    }

    fn after_error(&self, _: AsyncSnapshot<T, E>, error: E) -> AsyncSnapshot<T, E> {
        AsyncSnapshot::with_error(ConnectionState::Active, error)
    }

    fn after_done(&self, current: AsyncSnapshot<T, E>) -> AsyncSnapshot<T, E> {
        current.in_state(ConnectionState::Done)
    }

    fn after_disconnected(&self, current: AsyncSnapshot<T, E>) -> AsyncSnapshot<T, E> {
        current.in_state(ConnectionState::None)
    }

    fn build(&self, context: &BuildContext<'_>, summary: &AsyncSnapshot<T, E>) -> Widget {
        (self.builder)(context, summary)
    }
}

impl<T: Clone + 'static, E: Clone + 'static> From<StreamBuilder<T, E>> for Widget {
    fn from(value: StreamBuilder<T, E>) -> Self {
        value.into_widget()
    }
}
