//! Mounted primary-mouse counterparts from Flutter 3.50.0 tap_region_test.
//! Geometry stand-ins are keyed boxes, so clicks target the same regions
//! without assuming identical text measurement.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, GestureDetector, HitTestBehavior, Row, SizedBox, Stack, TapRegion,
    TapRegionSurface, TextDirection, Widget, internal::Key,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Spot {
    Outside,
    Solo,
    A,
    B,
    OutsideSurface,
}

impl Spot {
    fn key(self) -> Key {
        Key::from(match self {
            Self::Outside => "outside",
            Self::Solo => "ungrouped",
            Self::A => "group member a",
            Self::B => "group member b",
            Self::OutsideSurface => "outside surface",
        })
    }

    fn widget(self) -> Widget {
        Widget::box_(Size::new(70., 32.), Color::WHITE).with_key(self.key())
    }
}

#[derive(Clone, Copy)]
enum Report {
    OutsideDown,
    OutsideUp,
    InsideDown,
    InsideUp,
}

#[derive(Default)]
struct Scene {
    events: Rc<RefCell<Vec<Spot>>>,
    propagated: Rc<Cell<u32>>,
}

impl Scene {
    fn region(&self, spot: Spot, group: Option<u64>, report: Report, consume: bool) -> Widget {
        let mut region = TapRegion::new(spot.widget()).consume_outside_taps(consume);
        if let Some(group) = group {
            region = region.group_id(group);
        }
        let events = Rc::clone(&self.events);
        let report_spot = move |_| events.borrow_mut().push(spot);
        match report {
            Report::OutsideDown => region.on_tap_outside(report_spot),
            Report::OutsideUp => region.on_tap_up_outside(report_spot),
            Report::InsideDown => region.on_tap_inside(report_spot),
            Report::InsideUp => region.on_tap_up_inside(report_spot),
        }
        .into()
    }

    fn grouped_surface(
        &self,
        report: Report,
        include_solo: bool,
        group_b: u64,
        consume: bool,
    ) -> Widget {
        use Spot::{A, B, Outside, Solo};

        let propagated = Rc::clone(&self.propagated);
        let outside = GestureDetector::new(Outside.widget())
            .on_tap(move || propagated.set(propagated.get() + 1));
        let mut children = vec![outside.into()];
        if include_solo {
            children.push(self.region(Solo, None, report, consume));
        }
        children.push(self.region(A, Some(1), report, false));
        children.push(self.region(B, Some(group_b), report, consume));
        TapRegionSurface::new(Row::new(children)).into()
    }

    fn consume_scene(&self, consume: bool) -> Widget {
        Column::new([
            Spot::OutsideSurface.widget(),
            self.grouped_surface(Report::OutsideDown, true, 1, consume),
        ])
        .into()
    }

    fn events(&self) -> Vec<Spot> {
        let mut events = self.events.take();
        events.sort_unstable();
        events
    }
}

#[test]
fn tap_surface_notifies_outside_before_inside_on_down_and_up() {
    let trace = Rc::new(RefCell::new(Vec::new()));
    let record = |entry| {
        let trace = Rc::clone(&trace);
        move |_| trace.borrow_mut().push(entry)
    };
    let outside = TapRegion::new(Spot::A.widget())
        .on_tap_outside(record("A.outsideDown"))
        .on_tap_up_outside(record("A.outsideUp"));
    let inside = TapRegion::new(Spot::B.widget())
        .on_tap_inside(record("B.insideDown"))
        .on_tap_up_inside(record("B.insideUp"));
    let root = TapRegionSurface::new(Row::new([Widget::from(outside), Widget::from(inside)]));
    let mut host = Host::mount(root.into());
    host.click(Spot::B, &Scene::default());
    assert_eq!(
        trace.take(),
        ["A.outsideDown", "B.insideDown", "A.outsideUp", "B.insideUp"]
    );
}

struct Host {
    runtime: Runtime,
    now: Instant,
    next_pointer: u64,
}

impl Host {
    fn mount(root: Widget) -> Self {
        let mut host = Self {
            runtime: Runtime::new(Directionality::new(TextDirection::Ltr, root).into())
                .expect("mount tap region surface"),
            now: Instant::now(),
            next_pointer: 1,
        };
        host.frame();
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(600., 300.)), self.now)
            .expect("lay out tap targets");
    }

    fn update(&mut self, widget: Widget) {
        let root = self.runtime.tree().root().expect("retained root");
        self.runtime
            .tree_mut()
            .update(root, Directionality::new(TextDirection::Ltr, widget).into())
            .expect("update tap regions in place");
        self.frame();
    }

    fn click(&mut self, spot: Spot, scene: &Scene) -> [Vec<Spot>; 2] {
        self.contact(spot, scene, PointerPhase::Up)
    }

    fn contact(&mut self, spot: Spot, scene: &Scene, end: PointerPhase) -> [Vec<Spot>; 2] {
        let tree = self.runtime.tree();
        let id = tree.element_with_key(&spot.key()).expect("target mounted");
        let (transform, size) = tree.element_geometry(id).expect("target geometry");
        let rect = transform.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size));
        let position = Offset::new(
            rect.origin.x + rect.size.width / 2.,
            rect.origin.y + rect.size.height / 2.,
        );
        let pointer = self.next_pointer;
        self.next_pointer += 1;
        [PointerPhase::Down, end].map(|phase| {
            let _ = self.runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer,
                    device: 1,
                    kind: PointerDeviceKind::Mouse,
                    buttons: if phase == PointerPhase::Down {
                        PRIMARY_POINTER_BUTTON
                    } else {
                        0
                    },
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::default(),
                    phase,
                    position,
                },
                self.now,
            );
            self.now += Duration::from_millis(16);
            scene.events()
        })
    }
}

#[test]
fn tap_surface_reports_outside_on_primary_down_then_cancel() {
    use Spot::{A, B, Outside, OutsideSurface, Solo};
    let scene = Scene::default();
    let mut host = Host::mount(scene.consume_scene(false));
    assert!(scene.events().is_empty());
    for (hit, expected) in [
        (Solo, vec![A, B]),
        (A, vec![Solo]),
        (B, vec![Solo]),
        (Outside, vec![Solo, A, B]),
        (OutsideSurface, vec![]),
    ] {
        assert_eq!(
            host.contact(hit, &scene, PointerPhase::Cancel),
            [expected, vec![]]
        );
    }
}

#[test]
fn tap_surface_reports_inside_on_primary_down_then_cancel() {
    use Spot::{A, B, Outside, OutsideSurface, Solo};
    let scene = Scene::default();
    let mut host = Host::mount(
        Column::new([
            OutsideSurface.widget(),
            scene.grouped_surface(Report::InsideDown, true, 1, false),
        ])
        .into(),
    );
    assert!(scene.events().is_empty());
    for (hit, expected) in [
        (Solo, vec![Solo]),
        (A, vec![A, B]),
        (B, vec![A, B]),
        (Outside, vec![]),
        (OutsideSurface, vec![]),
    ] {
        assert_eq!(
            host.contact(hit, &scene, PointerPhase::Cancel),
            [expected, vec![]]
        );
    }
}

#[test]
fn tap_surface_distinguishes_empty_defer_opaque_and_translucent_regions() {
    use Spot::{A, B, OutsideSurface, Solo};
    let scene = Scene::default();
    let widget = |spot: Spot, group, behavior| {
        let stack = Widget::from(Stack::new(Vec::<Widget>::new())).with_key(spot.key());
        let trace = Rc::clone(&scene.events);
        let mut region = TapRegion::new(stack)
            .behavior(behavior)
            .on_tap_inside(move |_| trace.borrow_mut().push(spot));
        if let Some(key) = group {
            region = region.group_id(key);
        }
        Widget::from(SizedBox::from_dimensions(
            Some(100.),
            Some(100.),
            Some(region),
        ))
    };
    let row = Row::new([
        widget(Solo, None, HitTestBehavior::DeferToChild),
        widget(A, Some(1_u64), HitTestBehavior::Opaque),
        widget(B, Some(1_u64), HitTestBehavior::Translucent),
    ]);
    let mut host = Host::mount(
        Column::new([OutsideSurface.widget(), TapRegionSurface::new(row).into()]).into(),
    );
    assert!(scene.events().is_empty());
    assert_eq!(
        [Solo, A, B].map(|spot| host.click(spot, &scene)),
        [[vec![], vec![]], [vec![A, B], vec![]], [vec![], vec![]]],
    );
}

#[test]
fn tap_surface_reports_outside_only_on_primary_mouse_up() {
    use Spot::{A, B, Outside, Solo};

    let scene = Scene::default();
    let mut host = Host::mount(scene.grouped_surface(Report::OutsideUp, true, 1, false));
    assert!(scene.events().is_empty());
    for (hit, expected) in [
        (Solo, vec![A, B]),
        (A, vec![Solo]),
        (B, vec![Solo]),
        (Outside, vec![Solo, A, B]),
    ] {
        assert_eq!(host.click(hit, &scene), [vec![], expected], "{hit:?}");
    }
}

#[test]
fn tap_surface_reports_inside_only_on_primary_mouse_up() {
    use Spot::{A, B, Outside, Solo};

    let scene = Scene::default();
    let mut host = Host::mount(scene.grouped_surface(Report::InsideUp, true, 1, false));
    assert!(scene.events().is_empty());
    for (hit, expected) in [
        (Solo, vec![Solo]),
        (A, vec![A, B]),
        (B, vec![A, B]),
        (Outside, vec![]),
    ] {
        assert_eq!(host.click(hit, &scene), [vec![], expected], "{hit:?}");
    }
}

#[test]
fn consuming_outside_regions_prevent_the_underlying_primary_tap() {
    use Spot::{A, B, Outside, OutsideSurface, Solo};

    let scene = Scene::default();
    let mut host = Host::mount(scene.consume_scene(true));
    assert!(scene.events().is_empty());
    for (hit, expected) in [
        (Solo, vec![A, B]),
        (A, vec![Solo]),
        (B, vec![Solo]),
        (Outside, vec![Solo, A, B]),
        (OutsideSurface, vec![]),
    ] {
        assert_eq!(host.click(hit, &scene), [expected, vec![]], "{hit:?}");
        assert_eq!(scene.propagated.get(), 0, "{hit:?}");
    }
    host.update(scene.consume_scene(false));
    assert_eq!(host.click(Outside, &scene), [vec![Solo, A, B], vec![]]);
    assert_eq!(
        scene.propagated.get(),
        1,
        "control: outside detector handles an unconsumed tap"
    );
}

#[test]
fn retained_tap_region_group_update_changes_outside_callbacks() {
    use Spot::{A, B, Outside};

    let scene = Scene::default();
    let mut host = Host::mount(scene.grouped_surface(Report::OutsideDown, false, 1, false));
    let member = host
        .runtime
        .tree()
        .element_with_key(&B.key())
        .expect("group member mounted");
    assert_eq!(host.click(A, &scene), [vec![], vec![]]);
    assert_eq!(host.click(B, &scene), [vec![], vec![]]);
    assert_eq!(host.click(Outside, &scene), [vec![A, B], vec![]]);

    host.update(scene.grouped_surface(Report::OutsideDown, false, 2, false));
    assert_eq!(host.runtime.tree().element_with_key(&B.key()), Some(member));
    assert_eq!(host.click(A, &scene), [vec![B], vec![]]);
    assert_eq!(host.click(B, &scene), [vec![A], vec![]]);
    assert_eq!(host.click(Outside, &scene), [vec![A, B], vec![]]);
}
