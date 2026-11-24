//! Observable Flutter 3.50.0 SliverLayoutBuilder box adapter behavior.
use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    CustomScrollView, Directionality, GestureDetector, MediaQuery, MediaQueryData, Padding,
    ScrollController, SizedBox, Sliver, SliverLayoutBuilder, SliverToBoxAdapter, TextDirection,
    Widget, internal::Key,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
fn fixed(key: u64, h: f32) -> Widget {
    Widget::from(SizedBox::new().height(h)).with_key(Key::Value(key))
}
fn adapter(child: impl Into<Widget>) -> Box<dyn Sliver> {
    Box::new(SliverToBoxAdapter::new(child))
}
fn viewport(
    scroll: &ScrollController,
    children: impl IntoIterator<Item = Box<dyn Sliver>>,
    padded: bool,
) -> Widget {
    let view: Widget = CustomScrollView::new(children)
        .controller(scroll.clone())
        .into();
    let view = if padded {
        Padding::all(50., view).into()
    } else {
        view
    };
    Directionality::new(TextDirection::Ltr, view).into()
}
struct Host {
    runtime: Runtime,
    now: Instant,
}
impl Host {
    fn new(scene: Widget) -> Self {
        let mut host = Self {
            runtime: Runtime::new(scene).unwrap(),
            now: Instant::now(),
        };
        host.frame();
        host
    }
    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(VIEW), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }
    fn change(&mut self, scene: Widget) {
        let root = self.runtime.tree().root().unwrap();
        self.runtime.schedule_update(root, scene).unwrap();
        self.frame();
    }
    fn bounds(&self, which: u64) -> Rect {
        let tree = self.runtime.tree();
        tree.element_bounds(
            tree.element_with_key(&Key::Value(which))
                .expect("mounted child"),
        )
        .unwrap()
    }
    fn tap(&mut self, n: u64, position: Offset) {
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            self.runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: n,
                    device: 0,
                    kind: PointerDeviceKind::Touch,
                    buttons: if phase == PointerPhase::Down {
                        PRIMARY_POINTER_BUTTON
                    } else {
                        0
                    },
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    phase,
                    position,
                },
                self.now,
            );
            self.now += Duration::from_millis(4);
        }
        self.frame();
    }
}

#[test]
fn consecutive_builders_see_reduced_remaining_viewport_extent() {
    let capture = Rc::new(RefCell::new([None, None]));
    let slivers: Vec<Box<dyn Sliver>> = [6., 20.]
        .into_iter()
        .enumerate()
        .map(|(which, extent)| {
            let log = capture.clone();
            Box::new(SliverLayoutBuilder::new(move |c| {
                log.borrow_mut()[which] = Some((c.cross_axis_extent, c.remaining_paint_extent));
                fixed(which as u64, extent)
            })) as Box<dyn Sliver>
        })
        .collect();
    let host = Host::new(viewport(&ScrollController::new(), slivers, false));
    assert_eq!(*capture.borrow(), [Some((800., 600.)), Some((800., 594.))]);
    assert_eq!(
        [host.bounds(0).size.height, host.bounds(1).size.height],
        [6., 20.]
    );
    assert_eq!(host.bounds(1).origin.y, 6.);
}

#[test]
fn stateful_box_adapter_updates_measured_extent_and_scroll_range() {
    let value = Rc::new(Cell::new(Size::new(10., 20.)));
    let revision = Rc::new(Cell::new(0u64));
    let child = Widget::stateful_builder(revision.clone(), {
        let value = value.clone();
        move |_| {
            let size = value.get();
            Widget::from(SizedBox::new().width(size.width).height(size.height))
                .with_key(Key::Value(1))
        }
    });
    let scroll = ScrollController::new();
    let sliver: Box<dyn Sliver> = Box::new(SliverLayoutBuilder::new(move |_| child.clone()));
    let mut host = Host::new(viewport(&scroll, [sliver], false));
    let mut results = vec![(host.bounds(1).size, scroll.max_offset())];
    for (tick, geometry) in [(1, Size::new(100., 200.)), (2, Size::new(900., 900.))] {
        value.set(geometry);
        revision.set(tick);
        host.frame();
        results.push((host.bounds(1).size, scroll.max_offset()));
    }
    assert_eq!(
        results,
        [
            (Size::new(800., 20.), 0.),
            (Size::new(800., 200.), 0.),
            (Size::new(800., 900.), 300.)
        ]
    );
}

#[test]
fn no_read_of_media_query_does_not_reinvoke_sliver_builder() {
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let sliver: Box<dyn Sliver> = Box::new(SliverLayoutBuilder::new(move |_| {
        seen.set(seen.get() + 1);
        SizedBox::shrink()
    }));
    let target = viewport(&ScrollController::new(), [sliver], false);
    let context = |viewport| {
        Widget::from(MediaQuery::new(
            MediaQueryData {
                viewport,
                ..MediaQueryData::default()
            },
            target.clone(),
        ))
    };
    assert_eq!(count.get(), 0);
    let mut host = Host::new(context(Size::new(400., 300.)));
    assert_eq!(count.get(), 1);
    host.change(context(Size::new(300., 400.)));
    assert_eq!(count.get(), 1);
}

fn top_pair(tail: f32) -> ([f32; 2], [f32; 2], f32) {
    let scroll = ScrollController::new();
    let mut slivers = vec![
        adapter(fixed(0, 300.)),
        Box::new(SliverLayoutBuilder::new(|_| fixed(1, 200.))),
        adapter(fixed(2, 100.)),
    ];
    if tail > 0. {
        slivers.push(adapter(fixed(3, tail)));
    }
    let mut host = Host::new(viewport(&scroll, slivers, false));
    let pos = |host: &Host| {
        [
            host.bounds(1).origin.y + 100.,
            host.bounds(2).origin.y + 100.,
        ]
    };
    assert_eq!([host.bounds(1).origin.x, host.bounds(2).origin.x], [0., 0.]);
    let start = pos(&host);
    scroll.jump_to(100.);
    host.frame();
    assert_eq!([host.bounds(1).origin.x, host.bounds(2).origin.x], [0., 0.]);
    (start, pos(&host), scroll.offset())
}

#[test]
fn local_to_global_preserves_programmatic_reveal_even_at_equal_content_extent() {
    assert_eq!(
        top_pair(100.),
        ([400., 600.], [300., 500.], 100.),
        "in-range transform control"
    );
    let (start, after, offset) = top_pair(0.);
    assert_eq!(start, [400., 600.]);
    assert_eq!(after, [300., 500.], "actual controller offset {offset}");
}

fn tap_scene(
    scroll: &ScrollController,
    hits: &Rc<RefCell<Vec<usize>>>,
    h: f32,
    padded: bool,
) -> Widget {
    let child = |index| {
        let hits = hits.clone();
        Widget::from(
            GestureDetector::new(Widget::box_(Size::new(100., h), Color::WHITE))
                .on_tap(move || hits.borrow_mut().push(index)),
        )
    };
    let second = child(1);
    viewport(
        scroll,
        [
            adapter(child(0)),
            Box::new(SliverLayoutBuilder::new(move |_| second.clone())),
            adapter(child(2)),
        ],
        padded,
    )
}

#[test]
fn hits_reach_box_adapter_before_after_scroll_and_not_outside_viewport() {
    let scroll = ScrollController::new();
    let hits = Rc::new(RefCell::new(Vec::new()));
    let mut host = Host::new(tap_scene(&scroll, &hits, 200., true));
    for (p, y) in [150., 350., 460.].into_iter().enumerate() {
        host.tap(p as u64 + 1, Offset::new(300., y));
    }
    let first = hits.take();
    scroll.jump_to(100.);
    host.frame();
    for (p, y) in [50., 250., 450.].into_iter().enumerate() {
        host.tap(p as u64 + 4, Offset::new(300., y));
    }
    let scrolled = hits.take();
    for (p, point) in [
        Offset::new(300., 1.),
        Offset::new(300., 599.),
        Offset::new(1., 100.),
        Offset::new(799., 100.),
    ]
    .into_iter()
    .enumerate()
    {
        host.tap(p as u64 + 7, point);
    }
    let outside = hits.take();
    host.change(tap_scene(&scroll, &hits, 100., false));
    host.tap(11, Offset::new(300., 301.));
    assert_eq!(
        (first, scrolled, outside, hits.take()),
        (vec![0, 1, 2], vec![0, 1, 2], vec![], vec![])
    );
}

#[test]
fn moving_a_focused_layoutbuilder_offscreen_does_not_crash() {
    use incular_text::TextEditingController;
    use incular_widgets::{EditableText, LayoutBuilder, SliverChildListDelegate, SliverList};

    let ctl = TextEditingController::with_text("X");
    let target = Widget::from(LayoutBuilder::new({
        let ctl = ctl.clone();
        move |_, _| {
            Widget::from(
                EditableText::new(ctl.clone())
                    .text_direction(TextDirection::Ltr)
                    .size(Size::new(200., 40.)),
            )
            .with_key(Key::Value(72))
        }
    }))
    .with_key(Key::Value(73));
    let scroll = ScrollController::new();
    let scene = |lead| {
        let children = SliverChildListDelegate::new([fixed(70, lead), target.clone()])
            .add_repaint_boundaries(false)
            .add_semantic_indexes(false);
        let items: Box<dyn Sliver> = Box::new(SliverList::delegate(children));
        viewport(&scroll, [items], false)
    };
    let mut host = Host::new(scene(60.));
    let before = host.bounds(72);
    assert!(before.origin.y < VIEW.height);
    let editor = host
        .runtime
        .tree()
        .element_with_key(&Key::Value(72))
        .unwrap();
    host.tap(1, before.origin + Offset::new(10., 10.));
    assert_eq!(
        host.runtime.focused_element(),
        Some(editor),
        "actual focused visible editor control"
    );
    host.change(scene(6000.));
    host.frame();
    assert_eq!(ctl.text(), "X");
}
