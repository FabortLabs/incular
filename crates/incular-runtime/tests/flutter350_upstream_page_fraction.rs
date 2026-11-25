//! PageView viewport fractions, controller replacement, and cache-edge taps.
use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Container, Directionality, GestureDetector, PageController, PageView, SizedBox, Text,
    TextDirection, Widget, internal::Key,
};
use std::{cell::RefCell, rc::Rc, time::Duration};

const ROOT: Size = Size {
    width: 800.,
    height: 600.,
};
type Log = Rc<RefCell<Vec<usize>>>;
fn build(
    c: &PageController,
    total: usize,
    size: Size,
    taps: Option<Log>,
    changes: Option<Log>,
) -> Widget {
    let mut page = PageView::builder(total, move |i| {
        let text = Container::new()
            .height(200.)
            .color(Color::rgba(0, 0, 255, 255))
            .child(Text::new(i.to_string()));
        let w = if let Some(taps) = &taps {
            let taps = taps.clone();
            Widget::from(GestureDetector::new(text).on_tap(move || taps.borrow_mut().push(i)))
        } else {
            text.into()
        };
        w.with_key(Key::Value(i as u64))
    })
    .controller(c.clone());
    if let Some(log) = changes {
        page = page.on_page_changed(move |i| log.borrow_mut().push(i));
    }
    Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::from_size(size).child(page)),
    )
    .into()
}
struct Host {
    r: Runtime,
    now: Instant,
    pointer: u64,
}
impl Host {
    fn new(w: Widget) -> Self {
        let mut h = Self {
            r: Runtime::new(w).unwrap(),
            now: Instant::now(),
            pointer: 0,
        };
        h.pump();
        h
    }
    fn pump(&mut self) {
        self.r
            .run_frame_at(Constraints::tight(ROOT), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }
    fn update(&mut self, w: Widget) {
        let root = self.r.tree().root().unwrap();
        self.r.schedule_update(root, w).unwrap();
        self.pump();
    }
    fn rect(&self, i: u64) -> Option<Rect> {
        let t = self.r.tree();
        t.element_with_key(&Key::Value(i))
            .and_then(|id| t.element_bounds(id))
    }
    fn center(&self, i: u64) -> Option<Offset> {
        self.rect(i).map(|r| {
            Offset::new(
                r.origin.x + r.size.width / 2.,
                r.origin.y + r.size.height / 2.,
            )
        })
    }
    fn offsets<const N: usize>(&self, keys: [u64; N]) -> [Option<Offset>; N] {
        keys.map(|i| self.rect(i).map(|r| r.origin))
    }
    fn tap(&mut self, p: Offset) {
        self.pointer += 1;
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            self.r.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: self.pointer,
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
                    position: p,
                },
                self.now,
            );
            self.now += Duration::from_millis(8);
        }
        self.pump();
    }
}
#[test]
fn controller_replacement_keeps_selected_page_and_recenters_new_fraction() {
    let c = PageController::new().with_viewport_fraction(7. / 8.);
    let mut h = Host::new(build(&c, 50, ROOT, None, None));
    let initial = h.offsets([0, 1]);
    let _ = c.jump_to_page(10);
    h.pump();
    let jumped = h.offsets([9, 10, 11]);
    let replacement = PageController::new().with_viewport_fraction(39. / 40.);
    h.update(build(&replacement, 50, ROOT, None, None));
    let updated = h.offsets([9, 10, 11]);
    let reported = replacement.page();
    let page_zero = h.rect(0);
    assert_eq!(
        (initial, jumped, updated, reported),
        (
            [50., 750.].map(|x| Some(Offset::new(x, 0.))),
            [-650., 50., 750.].map(|x| Some(Offset::new(x, 0.))),
            [-770., 10., 790.].map(|x| Some(Offset::new(x, 0.))),
            Some(10.),
        ),
        "replacement view page-zero bounds: {page_zero:?}"
    );
}
#[test]
fn small_fraction_places_all_neighbor_pages_on_the_exact_grid() {
    let c = PageController::new().with_viewport_fraction(1. / 8.);
    let mut h = Host::new(build(&c, 50, ROOT, None, None));
    let initial = h.offsets([0, 1, 2, 3, 4]);
    let _ = c.jump_to_page(10);
    h.pump();
    let after = h.offsets([6, 7, 8, 9, 10, 11, 12, 13, 14]);
    assert_eq!(
        (initial, after),
        (
            [350., 450., 550., 650., 750.].map(|x| Some(Offset::new(x, 0.))),
            [-50., 50., 150., 250., 350., 450., 550., 650., 750.].map(|x| Some(Offset::new(x, 0.)))
        )
    );
}
#[test]
fn oversized_fraction_keeps_the_whole_page_centered_before_and_after_jump() {
    let c = PageController::new().with_viewport_fraction(5. / 4.);
    let mut h = Host::new(build(&c, 50, ROOT, None, None));
    let initial = h.rect(0);
    let _ = c.jump_to_page(10);
    h.pump();
    let want = Some(Rect::from_origin_size(
        Offset::new(-100., 0.),
        Size::new(1000., 600.),
    ));
    assert_eq!((initial, h.rect(10)), (want, want));
}
#[test]
fn replacing_the_oversized_fraction_then_jumping_recomputes_inset() {
    let c = PageController::new().with_viewport_fraction(5. / 4.);
    let mut h = Host::new(build(&c, 50, ROOT, None, None));
    let normal = h.rect(0);
    let new = PageController::new().with_viewport_fraction(4.);
    h.update(build(&new, 50, ROOT, None, None));
    let _ = new.jump_to_page(10);
    h.pump();
    assert_eq!(
        (normal, h.rect(10)),
        (
            Some(Rect::from_origin_size(
                Offset::new(-100., 0.),
                Size::new(1000., 600.)
            )),
            Some(Rect::from_origin_size(
                Offset::new(-1200., 0.),
                Size::new(3200., 600.)
            ))
        )
    );
}
#[test]
fn oversized_last_page_snaps_to_viewport_center() {
    let c = PageController::new().with_viewport_fraction(5. / 4.);
    let mut h = Host::new(build(&c, 3, ROOT, None, None));
    let initial = h.center(0);
    let _ = c.jump_to_page(2);
    for _ in 0..120 {
        h.pump();
    }
    assert_eq!([initial, h.center(2)], [Some(Offset::new(400., 300.)); 2]);
}
#[test]
fn touch_reaches_all_partially_visible_pages_at_both_scroll_ends() {
    let taps = Rc::new(RefCell::new(Vec::new()));
    let c = PageController::new().with_viewport_fraction(1. / 4.);
    let mut h = Host::new(build(&c, 20, ROOT, Some(taps.clone()), None));
    let mut missing = Vec::new();
    for index in [0, 1, 2] {
        if let Some(p) = h.center(index) {
            h.tap(p - Offset::new(3., 0.));
        } else {
            missing.push(index);
        }
    }
    let _ = c.jump_to_page(19);
    h.pump();
    for index in [17, 18, 19] {
        if let Some(p) = h.center(index) {
            h.tap(p);
        } else {
            missing.push(index);
        }
    }
    assert_eq!(
        (missing, taps.borrow().clone()),
        (vec![], vec![0, 1, 2, 17, 18, 19])
    );
}
#[test]
fn last_fractional_page_remains_centered_when_parent_constraints_narrow() {
    let c = PageController::new()
        .with_initial_page(49)
        .with_viewport_fraction(0.5);
    let mut h = Host::new(build(&c, 50, Size::new(300., 300.), None, None));
    let before = h.center(49);
    h.update(build(&c, 50, Size::new(200., 300.), None, None));
    assert_eq!([before, h.center(49)], [Some(Offset::new(400., 300.)); 2]);
}
#[test]
fn out_of_range_jump_does_not_emit_a_page_change_from_the_last_page() {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let c = PageController::new().with_initial_page(49);
    let mut h = Host::new(build(&c, 50, ROOT, None, Some(changes.clone())));
    assert_eq!(c.page(), Some(49.));
    let _ = c.jump_to_page(100);
    let immediate = changes.borrow().clone();
    h.pump();
    assert_eq!((immediate, changes.borrow().clone()), (vec![], vec![]));
}

fn narrow_page_view(controller: &PageController, width: f32, snapping: bool) -> Widget {
    let pages = PageView::new((0..6).map(|page| Text::new(format!("page {page}"))))
        .controller(controller.clone())
        .page_snapping(snapping);
    Directionality::new(
        TextDirection::Ltr,
        Center::new(SizedBox::from_size(Size::new(width, 100.)).child(pages)),
    )
    .into()
}

#[test]
fn replacing_the_controller_at_zero_width_retains_the_collapsed_position() {
    let first = PageController::new().with_initial_page(2);
    let second = PageController::new()
        .with_initial_page(4)
        .with_viewport_fraction(0.75);
    let mut h = Host::new(narrow_page_view(&first, 200., true));
    assert_eq!(first.page(), Some(2.));
    h.update(narrow_page_view(&first, 0., true));
    assert_eq!(first.page(), Some(2.));
    h.update(narrow_page_view(&second, 0., true));
    assert_eq!(first.page(), None);
    assert_eq!(second.page(), Some(2.));
    h.update(narrow_page_view(&second, 300., true));
    assert_eq!(second.page(), Some(2.));
    assert_eq!(second.offset(), 450.);
}

#[test]
fn a_wide_fraction_uses_the_new_centering_origin_to_decode_the_old_pixels() {
    for snapping in [true, false] {
        let c = PageController::new().with_viewport_fraction(1.25);
        let mut h = Host::new(narrow_page_view(&c, 200., snapping));
        assert!(c.jump_to(425.));
        assert_eq!(c.page(), Some(1.6));
        h.update(narrow_page_view(&c, 300., snapping));
        assert_eq!(c.page(), Some(1.55), "snapping={snapping}");
        assert_eq!(c.offset(), 618.75, "snapping={snapping}");
    }
}
