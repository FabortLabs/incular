//! Real retained custom paint lifecycle from the pinned DecoratedSliver tests.
use std::{cell::RefCell, rc::Rc, time::Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{Color, Offset, Rect, Size};
use incular_rendering::{Canvas, PaintCommand};
use incular_scroll::ScrollController;
use incular_widgets::{
    CustomScrollView, DecoratedSliver, DecorationPosition, Directionality, SizedBox, Sliver,
    SliverBoxPaintConfiguration, SliverBoxPainter, SliverBoxPainterFactory, SliverToBoxAdapter,
    Widget, internal::WidgetTree,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const PAINT: Color = Color::rgba(130, 47, 87, 255);

#[derive(Default, Debug, PartialEq)]
struct Record {
    paints: Vec<SliverBoxPaintConfiguration>,
    disposals: usize,
}

type Records = Rc<RefCell<Vec<Record>>>;

struct Painter {
    index: usize,
    records: Records,
}

impl SliverBoxPainter for Painter {
    fn paint(&mut self, canvas: &mut Canvas, configuration: SliverBoxPaintConfiguration) {
        let mut records = self.records.borrow_mut();
        let record = &mut records[self.index];
        assert_eq!(record.disposals, 0, "a disposed painter must not paint");
        record.paints.push(configuration);
        canvas.rect(
            Rect::from_origin_size(Offset::ZERO, configuration.size),
            PAINT,
        );
    }

    fn dispose(&mut self) {
        self.records.borrow_mut()[self.index].disposals += 1;
    }
}

fn recording_factory() -> (SliverBoxPainterFactory, Records) {
    let records = Records::default();
    let factory = Rc::new({
        let records = records.clone();
        move || {
            let mut state = records.borrow_mut();
            let index = state.len();
            state.push(Record::default());
            Box::new(Painter {
                index,
                records: records.clone(),
            }) as Box<dyn SliverBoxPainter>
        }
    });
    (factory, records)
}

fn scene(
    factory: &SliverBoxPainterFactory,
    controller: &ScrollController,
    position: DecorationPosition,
) -> Widget {
    let sliver = DecoratedSliver::new_with_painter(
        factory.clone(),
        SliverToBoxAdapter::new(SizedBox::new().width(100.).height(100.)),
    )
    .position(position);
    Directionality::new(
        TextDirection::Ltr,
        CustomScrollView::new(vec![Box::new(sliver) as Box<dyn Sliver>])
            .controller(controller.clone()),
    )
    .into()
}

fn paint_sliver(tree: &mut WidgetTree) {
    tree.layout(Constraints::tight(VIEW)).unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let commands = tree.paint();
    assert!(commands.commands().iter().any(|command| {
        matches!(command, PaintCommand::Rect { rect, color } if *color == PAINT
            && *rect == Rect::from_origin_size(Offset::ZERO, Size::new(800., 100.)))
    }));
}

#[test]
fn mounted_decorated_sliver_creates_paints_and_disposes_the_box_painter() {
    let (factory, records) = recording_factory();
    let controller = ScrollController::new();
    let scene = scene(&factory, &controller, DecorationPosition::Background);
    assert!(
        records.borrow().is_empty(),
        "discardable descriptor does not allocate"
    );
    let mut tree = WidgetTree::new();
    let frame = |child: Widget| {
        Widget::from(
            SizedBox::new()
                .width(VIEW.width)
                .height(VIEW.height)
                .child(child),
        )
    };
    let root = tree.mount(frame(scene)).unwrap();
    paint_sliver(&mut tree);
    assert_eq!(
        &*records.borrow(),
        &[Record {
            paints: vec![SliverBoxPaintConfiguration {
                size: Size::new(800., 100.),
                text_direction: Some(TextDirection::Ltr),
            }],
            disposals: 0,
        }]
    );

    tree.update(root, frame(SizedBox::shrink().into())).unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    let _ = tree.paint();
    assert_eq!(records.borrow().len(), 1);
    assert_eq!(records.borrow()[0].disposals, 1);
    drop(tree);
    assert_eq!(records.borrow()[0].disposals, 1);
}

#[test]
fn replacing_a_decoration_creates_a_different_painter_and_releases_the_old_one() {
    let (first, first_records) = recording_factory();
    let (second, second_records) = recording_factory();
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(&first, &controller, DecorationPosition::Background))
        .unwrap();
    paint_sliver(&mut tree);
    assert_eq!(first_records.borrow().len(), 1);
    assert_eq!(first_records.borrow()[0].paints.len(), 1);
    assert!(second_records.borrow().is_empty());

    tree.update(
        root,
        scene(&second, &controller, DecorationPosition::Background),
    )
    .unwrap();
    paint_sliver(&mut tree);
    assert_eq!(first_records.borrow().len(), 1);
    assert_eq!(first_records.borrow()[0].paints.len(), 1);
    assert_eq!(first_records.borrow()[0].disposals, 1);
    assert_eq!(second_records.borrow().len(), 1);
    assert_eq!(second_records.borrow()[0].paints.len(), 1);
    assert_eq!(second_records.borrow()[0].disposals, 0);
}

#[test]
fn moving_paint_from_foreground_to_background_keeps_the_same_painter() {
    let (factory, records) = recording_factory();
    let controller = ScrollController::new();
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(&factory, &controller, DecorationPosition::Foreground))
        .unwrap();
    paint_sliver(&mut tree);
    assert_eq!(records.borrow().len(), 1);
    assert_eq!(records.borrow()[0].paints.len(), 1);

    tree.update(
        root,
        scene(&factory, &controller, DecorationPosition::Background),
    )
    .unwrap();
    paint_sliver(&mut tree);
    assert_eq!(records.borrow().len(), 1);
    assert_eq!(records.borrow()[0].paints.len(), 2);
    assert_eq!(records.borrow()[0].disposals, 0);
}
