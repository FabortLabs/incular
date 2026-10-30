//! Flutter's `FutureBuilder` and `StreamBuilder` (`widgets/async.dart`):
//! the snapshot each build receives as the source completes or emits.

use incular_config::Constraints;
use incular_core::{Color, Size};
use incular_widgets::internal::{ElementId, WidgetTree};
use incular_widgets::{
    AsyncSnapshot, ConnectionState, FutureBuilder, StreamBuilder, UiFuture, UiStreamController,
    Widget,
};

/// Encodes a snapshot as a box: the width is the data, or 100 plus the
/// error, and the height the connection state.
fn snapshot_box(snapshot: &AsyncSnapshot<u8, u8>) -> Widget {
    let width = match (snapshot.data, snapshot.error) {
        (Some(data), None) => f32::from(data),
        (None, Some(error)) => 100. + f32::from(error),
        (None, None) => 0.,
        (Some(_), Some(_)) => panic!("data and error are exclusive"),
    };
    let height = match snapshot.connection_state {
        ConnectionState::None => 1.,
        ConnectionState::Waiting => 2.,
        ConnectionState::Active => 3.,
        ConnectionState::Done => 4.,
    };
    Widget::box_(Size::new(width, height), Color::WHITE)
}

fn future_builder(future: Option<UiFuture<u8, u8>>) -> FutureBuilder<u8, u8> {
    FutureBuilder::new(future, |_, snapshot| snapshot_box(snapshot))
}

struct Harness {
    tree: WidgetTree,
    root: ElementId,
}

impl Harness {
    fn new(widget: Widget) -> Self {
        let mut tree = WidgetTree::new();
        let root = tree.mount(widget).expect("mount");
        let mut harness = Self { tree, root };
        harness.layout();
        harness
    }

    fn layout(&mut self) {
        self.tree
            .layout(Constraints::loose(Size::new(500., 500.)))
            .expect("layout");
    }

    fn update(&mut self, widget: Widget) {
        self.tree.update(self.root, widget).expect("update");
        self.layout();
    }

    /// The encoded snapshot of the last build.
    fn shown(&self) -> Size {
        self.tree.element_bounds(self.root).expect("bounds").size
    }
}

#[test]
fn a_future_is_waiting_until_it_completes() {
    let (future, completer) = UiFuture::pending();
    let mut harness = Harness::new(future_builder(Some(future)).into());
    assert_eq!(harness.shown(), Size::new(0., 2.));
    assert!(completer.complete(7));
    assert!(harness.tree.has_dirty_dependencies());
    harness.layout();
    assert_eq!(harness.shown(), Size::new(7., 4.));
    // A future completes once.
    assert!(!completer.complete(8));
}

#[test]
fn initial_data_shows_while_waiting_and_an_error_replaces_it() {
    let (future, completer) = UiFuture::pending();
    let mut harness = Harness::new(future_builder(Some(future)).initial_data(5).into());
    assert_eq!(harness.shown(), Size::new(5., 2.));
    completer.complete_error(3);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(103., 4.));
}

#[test]
fn without_a_future_the_snapshot_is_not_connected() {
    let harness = Harness::new(future_builder(None).initial_data(5).into());
    assert_eq!(harness.shown(), Size::new(5., 1.));
}

#[test]
fn a_completed_future_shows_its_result_on_the_first_build() {
    let harness = Harness::new(future_builder(Some(UiFuture::value(9))).into());
    assert_eq!(harness.shown(), Size::new(9., 4.));
}

#[test]
fn a_new_future_reconnects_and_ignores_the_old_one() {
    let (first, first_completer) = UiFuture::pending();
    let mut harness = Harness::new(future_builder(Some(first.clone())).into());
    first_completer.complete(3);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(3., 4.));

    // `_unsubscribe` keeps the data in state none, then `_subscribe` waits.
    let (second, second_completer) = UiFuture::pending();
    harness.update(future_builder(Some(second.clone())).into());
    assert_eq!(harness.shown(), Size::new(3., 2.));

    // The same future again changes nothing.
    harness.update(future_builder(Some(second)).into());
    assert_eq!(harness.shown(), Size::new(3., 2.));

    let (third, third_completer) = UiFuture::pending();
    harness.update(future_builder(Some(third)).into());
    second_completer.complete(8);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(3., 2.));
    third_completer.complete(9);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(9., 4.));

    // Dropping the future disconnects and keeps the data.
    harness.update(future_builder(None).into());
    assert_eq!(harness.shown(), Size::new(9., 1.));
}

fn stream_builder(controller: Option<&UiStreamController<u8, u8>>) -> StreamBuilder<u8, u8> {
    StreamBuilder::new(controller.map(UiStreamController::stream), |_, snapshot| {
        snapshot_box(snapshot)
    })
}

#[test]
fn a_stream_moves_from_waiting_through_active_to_done() {
    let controller = UiStreamController::new();
    let mut harness = Harness::new(stream_builder(Some(&controller)).into());
    assert_eq!(harness.shown(), Size::new(0., 2.));
    controller.add(1);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(1., 3.));
    // Events between frames fold in order; the build sees the last.
    controller.add(2);
    controller.add_error(4);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(104., 3.));
    controller.close();
    harness.layout();
    assert_eq!(harness.shown(), Size::new(104., 4.));
}

#[test]
fn a_new_stream_disconnects_and_reconnects() {
    let first = UiStreamController::new();
    let mut harness = Harness::new(stream_builder(Some(&first)).initial_data(6).into());
    assert_eq!(harness.shown(), Size::new(6., 2.));
    first.add(1);
    harness.layout();

    let second = UiStreamController::new();
    harness.update(stream_builder(Some(&second)).into());
    // `afterDisconnected` then `afterConnected`: the data stays.
    assert_eq!(harness.shown(), Size::new(1., 2.));
    first.add(7);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(1., 2.));
    second.add(2);
    harness.layout();
    assert_eq!(harness.shown(), Size::new(2., 3.));

    harness.update(stream_builder(None).into());
    assert_eq!(harness.shown(), Size::new(2., 1.));
}
