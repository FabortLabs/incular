//! Flutter 3.50.0 PageView surface and PageController lifecycle.
use incular_animation::Curve;
use incular_config::Constraints;
use incular_core::{Rect, Size, time::Instant};
use incular_runtime::Runtime;
use incular_widgets::{
    Center, Directionality, PageController, PageView, SizedBox, Text, TextDirection, Widget,
    internal::Key,
};
use std::time::Duration;

const SURFACE: Size = Size {
    width: 800.,
    height: 600.,
};
const VIEW: u64 = 100;
const STATES: [&str; 15] = [
    "Alabama",
    "Alaska",
    "Arizona",
    "Arkansas",
    "California",
    "Colorado",
    "Connecticut",
    "Delaware",
    "Florida",
    "Georgia",
    "Hawaii",
    "Idaho",
    "Illinois",
    "Indiana",
    "Iowa",
];
fn content(c: Option<&PageController>, size: Size, expand: bool) -> Widget {
    let pages = STATES
        .into_iter()
        .enumerate()
        .map(|(i, label)| Widget::from(Text::new(label)).with_key(Key::Value(i as u64)));
    let mut view = PageView::new(pages);
    if let Some(c) = c {
        view = view.controller(c.clone()).on_page_changed(|_| {});
    }
    let view = Widget::from(view).with_key(Key::Value(VIEW));
    let size = if expand {
        SizedBox::expand()
    } else {
        SizedBox::from_size(size)
    };
    Directionality::new(TextDirection::Ltr, Center::new(size.child(view))).into()
}
struct Host {
    r: Runtime,
    now: Instant,
    size: Size,
}
impl Host {
    fn new(w: Widget, size: Size) -> Self {
        let mut h = Self {
            r: Runtime::new(w).unwrap(),
            now: Instant::now(),
            size,
        };
        h.pump();
        h
    }
    fn pump(&mut self) {
        self.r
            .run_frame_at(Constraints::tight(self.size), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }
    fn settle(&mut self) {
        for _ in 0..120 {
            self.pump();
        }
    }
    fn update(&mut self, w: Widget) {
        let root = self.r.tree().root().unwrap();
        self.r.schedule_update(root, w).unwrap();
        self.pump();
    }
    fn bounds(&self, key: u64) -> Option<Rect> {
        let t = self.r.tree();
        t.element_with_key(&Key::Value(key))
            .and_then(|id| t.element_bounds(id))
    }
    fn has(&self, key: u64) -> bool {
        self.r.tree().element_with_key(&Key::Value(key)).is_some()
    }
    fn visible(&self) -> Vec<u64> {
        let v = self.bounds(VIEW).expect("attached viewport");
        if v.size.width == 0. || v.size.height == 0. {
            return vec![];
        }
        (0..STATES.len() as u64)
            .filter(|i| {
                self.bounds(*i).is_some_and(|b| {
                    b.origin.x < v.origin.x + v.size.width
                        && b.origin.x + b.size.width > v.origin.x
                        && b.origin.y < v.origin.y + v.size.height
                        && b.origin.y + b.size.height > v.origin.y
                })
            })
            .collect()
    }
}
#[test]
fn initial_and_selected_page_survive_repeated_zero_size_resizes() {
    let c = PageController::new().with_initial_page(1);
    let build = |s| content(Some(&c), s, false);
    let small = Size::new(200., 200.);
    let mut h = Host::new(build(Size::ZERO), SURFACE);
    let initially_retained = h.has(0);
    let mut visible = vec![h.visible()];
    h.update(build(small));
    visible.push(h.visible());
    let _ = c.jump_to_page(14);
    h.pump();
    visible.push(h.visible());
    h.update(build(Size::ZERO));
    visible.push(h.visible());
    h.update(build(small));
    visible.push(h.visible());
    assert_eq!(
        (visible, initially_retained, c.page()),
        (
            vec![vec![], vec![1], vec![14], vec![], vec![14]],
            true,
            Some(14.)
        )
    );
}
#[test]
fn animate_then_jump_at_zero_width_records_latest_page_and_page_getter() {
    let c = PageController::new().with_initial_page(1);
    let build = |s| content(Some(&c), s, false);
    let mut h = Host::new(build(Size::ZERO), SURFACE);
    assert_eq!(h.bounds(VIEW).unwrap().size, Size::ZERO);
    assert!(h.visible().is_empty());
    let initially_retained = h.has(0);
    let _ = c.animate_to_page(14, Duration::from_millis(300), Curve::Ease);
    let animated = c.page();
    let _ = c.jump_to_page(12);
    let jumped = c.page();
    h.update(build(Size::new(200., 200.)));
    assert_eq!(
        (initially_retained, animated, jumped, c.page(), h.visible()),
        (true, Some(14.), Some(12.), Some(12.), vec![12])
    );
}
#[test]
fn expanding_viewport_can_resize_from_zero_to_nonzero_host_surface() {
    let c = PageController::new().with_initial_page(1);
    let w = content(Some(&c), Size::ZERO, true);
    let mut h = Host::new(w.clone(), Size::ZERO);
    assert_eq!(h.bounds(VIEW).unwrap().size, Size::ZERO);
    h.size = Size::new(500., 400.);
    h.update(w);
    assert_eq!(h.bounds(VIEW).unwrap().size, Size::new(500., 400.));
}
#[test]
fn unattached_controller_does_not_invent_a_page_number() {
    assert_eq!(PageController::new().page(), None);
}
#[test]
fn next_and_previous_page_survive_wide_to_narrow_resize() {
    let c = PageController::new().with_initial_page(4);
    let build = |width| content(Some(&c), Size::new(width, 400.), false);
    let mut h = Host::new(build(600.), SURFACE);
    let mut visible = vec![h.visible()];
    let _ = c.next_page(Duration::from_millis(150), Curve::Ease);
    h.settle();
    visible.push(h.visible());
    h.update(build(300.));
    visible.push(h.visible());
    let _ = c.previous_page(Duration::from_millis(150), Curve::Ease);
    h.settle();
    visible.push(h.visible());
    assert_eq!(visible, [vec![4], vec![5], vec![5], vec![4]]);
}
#[test]
fn controllerless_page_view_keeps_first_page_offstage_and_restores_visibility() {
    let mut h = Host::new(content(None, Size::ZERO, false), SURFACE);
    let initially_retained = h.has(0);
    let zero_view = h.visible();
    h.update(content(None, Size::new(200., 200.), false));
    assert_eq!(
        (initially_retained, zero_view, h.visible()),
        (true, vec![], vec![0])
    );
}

#[test]
fn collapsing_a_wide_fraction_uses_the_zero_extent_centering_origin() {
    use std::{cell::RefCell, rc::Rc};

    let controller = PageController::new().with_viewport_fraction(1.25);
    let notifications = Rc::new(RefCell::new(Vec::new()));
    let build = |width| {
        let notifications = notifications.clone();
        let page_view = PageView::new(STATES.map(Text::new))
            .controller(controller.clone())
            .page_snapping(false)
            .on_page_changed(move |page| notifications.borrow_mut().push(page));
        Directionality::new(
            TextDirection::Ltr,
            Center::new(SizedBox::from_size(Size::new(width, 200.)).child(page_view)),
        )
        .into()
    };
    let close = |observed: Option<f32>, expected: f32| {
        assert!(
            (observed.unwrap() - expected).abs() < 0.000_01,
            "{observed:?}"
        );
    };

    let mut host = Host::new(build(200.), SURFACE);
    assert!(controller.jump_to(425.));
    host.pump();
    close(controller.page(), 1.6);
    assert_eq!(&*notifications.borrow(), &[2]);

    host.update(build(0.));
    close(controller.page(), 1.7);
    host.update(build(300.));
    close(controller.page(), 1.7);
    assert_eq!(controller.offset(), 675.);
    assert_eq!(&*notifications.borrow(), &[2]);
}
