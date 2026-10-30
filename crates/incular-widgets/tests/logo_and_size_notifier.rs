//! Flutter's `FlutterLogo` in its three styles, `SizeChangedLayoutNotifier` and
//! notification bubbling through `NotificationListener`.

use std::{cell::RefCell, rc::Rc};

use incular_config::Constraints;
use incular_core::{Color, Offset, Rect, Size, Transform as CoreTransform};
use incular_rendering::{Brush, PaintCommand};
use incular_widgets::internal::WidgetTree;
use incular_widgets::{
    FlutterLogo, FlutterLogoStyle, IconTheme, NotificationDispatcher, NotificationListener,
    SizeChangedLayoutNotification, SizeChangedLayoutNotifier, Widget,
};

fn laid_out_size(widget: Widget) -> Size {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget).expect("mount");
    tree.layout(Constraints::loose(Size::new(500., 500.)))
        .expect("layout");
    tree.element_bounds(root).expect("bounds").size
}

#[test]
fn the_logo_takes_its_size_or_the_icon_theme_size() {
    assert_eq!(
        laid_out_size(FlutterLogo::new().into()),
        Size::new(24., 24.)
    );
    assert_eq!(
        laid_out_size(FlutterLogo::new().size(100.).into()),
        Size::new(100., 100.)
    );
    assert_eq!(
        laid_out_size(IconTheme::new(FlutterLogo::new()).size(40.).into()),
        Size::new(40., 40.)
    );
}

#[test]
fn the_mark_is_centered_in_the_largest_square() {
    let list = FlutterLogo::paint(Size::new(300., 202.));
    let commands = list.commands();
    // A 202 square at x = (300 - 202) / 2 = 49, at scale 1, with the 166
    // wide mark centered in it: 49 + (202 - 166) / 2 = 67.
    assert_eq!(
        commands[0],
        PaintCommand::PushTransform {
            transform: CoreTransform::translation(incular_core::Offset::new(67., 0.)),
        }
    );
    let fills: Vec<Brush> = commands
        .iter()
        .filter_map(|command| match command {
            PaintCommand::FillPath { brush, .. } => Some(brush.clone()),
            _ => None,
        })
        .collect();
    let light = Brush::Solid(Color::rgba(0x54, 0xC5, 0xF8, 0xFF));
    let dark = Brush::Solid(Color::rgba(0x01, 0x57, 0x9B, 0xFF));
    assert_eq!(fills[..3], [light.clone(), light, dark]);
    assert!(matches!(fills[3], Brush::LinearGradient(_)));
    assert!(commands.iter().any(|command| matches!(
        command,
        PaintCommand::Rect { color, .. } if *color == Color::rgba(0x29, 0xB6, 0xF6, 0xFF)
    )));
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "expected {expected}, got {actual}"
    );
}

fn assert_rect(actual: Rect, expected: Rect) {
    assert_close(actual.origin.x, expected.origin.x);
    assert_close(actual.origin.y, expected.origin.y);
    assert_close(actual.size.width, expected.size.width);
    assert_close(actual.size.height, expected.size.height);
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

/// The mark keeps 10.4 of its 202 units clear at each end, and the word
/// mark is laid out at 100 * 350 / 247 before scaling.
const INSET: f32 = 1. - 20.8 / 202.;
const BASE_FONT_SIZE: f32 = 100. * 350. / 247.;

#[test]
fn the_horizontal_logo_puts_the_word_mark_right_of_the_mark() {
    let layout = FlutterLogo::layout(Size::new(820., 232.), FlutterLogoStyle::Horizontal);
    assert_rect(layout.frame, rect(0., 0., 820., 232.));
    // The mark fills the frame height at the left.
    assert_rect(layout.mark, rect(0., 0., 232., 232.));
    // fontSize = 2/3 * 232 * (1 - 20.8/202) = 138.7406.
    let font_size = 2. / 3. * 232. * INSET;
    assert_close(font_size, 138.740_6);
    assert_close(layout.font_size, BASE_FONT_SIZE * font_size / 100.);
    // 256.4/820 * 820 - 32/350 * 138.7406 = 243.7151.
    assert_close(layout.text_anchor.x, 243.715_1);
    assert_close(layout.text_anchor.y, 0.);

    // In a square box the 820 x 232 logo is contained and centered:
    // scale 100/820, so 100 x 28.2927 at y = (100 - 28.2927) / 2.
    let square = FlutterLogo::layout(Size::new(100., 100.), FlutterLogoStyle::Horizontal);
    assert_rect(square.frame, rect(0., 35.853_66, 100., 28.292_68));
    assert_rect(square.mark, rect(0., 35.853_66, 28.292_68, 28.292_68));
}

#[test]
fn the_stacked_logo_centers_the_word_mark_below_the_mark() {
    let layout = FlutterLogo::layout(Size::new(252., 306.), FlutterLogoStyle::Stacked);
    // The mark is 306 * 191 / 306 = 191 square, centered: (252 - 191) / 2.
    assert_rect(layout.mark, rect(30.5, 0., 191., 191.));
    // fontSize = 0.35 * 191 * (1 - 20.8/202) = 59.9664.
    assert_close(
        layout.font_size,
        BASE_FONT_SIZE * 0.35 * 191. * INSET / 100.,
    );
    assert_close(layout.font_size, 84.972_68);
    // Centered on the mark, starting at its bottom.
    assert_close(layout.text_anchor.x, 126.);
    assert_close(layout.text_anchor.y, 191.);
}

#[test]
fn the_mark_only_logo_has_no_word_mark() {
    let layout = FlutterLogo::layout(Size::new(300., 202.), FlutterLogoStyle::MarkOnly);
    assert_rect(layout.mark, rect(49., 0., 202., 202.));
    assert_eq!(layout.font_size, 0.);
    // `paint` is the mark-only style: the same transform and commands.
    let plain = FlutterLogo::paint(Size::new(300., 202.));
    let styled = FlutterLogo::paint_mark(Size::new(300., 202.), FlutterLogoStyle::MarkOnly);
    assert_eq!(plain.commands().len(), styled.commands().len());
    assert_eq!(plain.commands()[0], styled.commands()[0]);
}

#[test]
fn a_styled_logo_keeps_its_square_box_and_draws_the_mark_in_its_square() {
    for style in [FlutterLogoStyle::Horizontal, FlutterLogoStyle::Stacked] {
        assert_eq!(
            laid_out_size(FlutterLogo::new().size(100.).style(style).into()),
            Size::new(100., 100.)
        );
    }
    // The mark transform starts at the mark square: the horizontal mark at
    // (0, 35.8537), scaled 28.2927 / 202.
    let list = FlutterLogo::paint_mark(Size::new(100., 100.), FlutterLogoStyle::Horizontal);
    let PaintCommand::PushTransform { transform } = list.commands()[0] else {
        panic!("the mark starts with its transform");
    };
    let origin = transform.transform_point(Offset::new(-18., 0.));
    assert_close(origin.x, 0.);
    assert_close(origin.y, 35.853_66);
}

#[test]
fn size_changes_are_reported_after_the_first_layout() {
    let reports = Rc::new(RefCell::new(Vec::new()));
    let notifier = |side: f32| -> Widget {
        let reports = reports.clone();
        SizeChangedLayoutNotifier::new(
            move |size| reports.borrow_mut().push(size),
            Widget::box_(Size::new(side, side), Color::WHITE),
        )
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(notifier(20.)).expect("mount");
    let layout = |tree: &mut WidgetTree| {
        tree.layout(Constraints::loose(Size::new(500., 500.)))
            .expect("layout");
    };
    layout(&mut tree);
    assert!(reports.borrow().is_empty());
    tree.update(root, notifier(20.)).expect("update");
    layout(&mut tree);
    assert!(reports.borrow().is_empty());
    tree.update(root, notifier(30.)).expect("update");
    layout(&mut tree);
    assert_eq!(*reports.borrow(), [Size::new(30., 30.)]);
}

/// Records which listeners saw a notification, innermost first.
type Log = Rc<RefCell<Vec<&'static str>>>;

fn notifying_tree(side: f32, log: &Log, inner_stops: bool) -> Widget {
    let (outer_log, inner_log, other_log) = (log.clone(), log.clone(), log.clone());
    NotificationListener::new(
        NotificationListener::new(
            NotificationListener::new(SizeChangedLayoutNotifier::notifying(Widget::box_(
                Size::new(side, side),
                Color::WHITE,
            )))
            .on(move |_: &u32| {
                other_log.borrow_mut().push("other type");
                true
            }),
        )
        .on(move |_: &SizeChangedLayoutNotification| {
            inner_log.borrow_mut().push("inner");
            inner_stops
        }),
    )
    .on(move |_: &SizeChangedLayoutNotification| {
        outer_log.borrow_mut().push("outer");
        false
    })
    .into()
}

fn relayout(tree: &mut WidgetTree) {
    tree.layout(Constraints::loose(Size::new(500., 500.)))
        .expect("layout");
}

#[test]
fn a_size_change_notification_bubbles_innermost_first() {
    let log: Log = Rc::default();
    let mut tree = WidgetTree::new();
    let root = tree.mount(notifying_tree(20., &log, false)).expect("mount");
    relayout(&mut tree);
    // The first layout is not a change.
    assert!(log.borrow().is_empty());
    tree.update(root, notifying_tree(30., &log, false))
        .expect("update");
    relayout(&mut tree);
    // A listener for another type is skipped; returning false lets the
    // notification continue outwards.
    assert_eq!(*log.borrow(), ["inner", "outer"]);
}

#[test]
fn a_listener_that_returns_true_stops_the_notification() {
    let log: Log = Rc::default();
    let mut tree = WidgetTree::new();
    let root = tree.mount(notifying_tree(20., &log, true)).expect("mount");
    relayout(&mut tree);
    tree.update(root, notifying_tree(30., &log, true))
        .expect("update");
    relayout(&mut tree);
    assert_eq!(*log.borrow(), ["inner"]);
}

#[test]
fn the_size_callback_runs_alongside_the_notification() {
    let log: Log = Rc::default();
    let notifier = |side: f32| -> Widget {
        let (callback_log, listener_log) = (log.clone(), log.clone());
        NotificationListener::new(SizeChangedLayoutNotifier::new(
            move |_| callback_log.borrow_mut().push("callback"),
            Widget::box_(Size::new(side, side), Color::WHITE),
        ))
        .on(move |_: &SizeChangedLayoutNotification| {
            listener_log.borrow_mut().push("listener");
            true
        })
        .into()
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(notifier(20.)).expect("mount");
    relayout(&mut tree);
    tree.update(root, notifier(25.)).expect("update");
    relayout(&mut tree);
    assert_eq!(*log.borrow(), ["callback", "listener"]);
}

#[test]
fn a_dispatcher_outside_every_listener_reaches_none() {
    assert!(!NotificationDispatcher::default().dispatch(&SizeChangedLayoutNotification));
}
