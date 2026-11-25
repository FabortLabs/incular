//! Flutter 3.50.0 SliverFillRemaining real drag and semantics boundary probes.
use incular_config::{Alignment, Constraints, MainAxisSize};
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, Size, time::Instant,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Center, Column, CustomScrollView, Directionality, Flexible, Padding, ScrollController,
    ScrollPhysics, Semantics, SizedBox, Sliver, SliverFillRemaining, SliverFixedExtentList,
    SliverToBoxAdapter, Text, TextDirection, Widget, internal::Key,
};
use std::time::Duration;

const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};
const FILL: u64 = 31;
const BUTTON: u64 = 32;
const LOGO: u64 = 33;
const LABEL: &str = "Text in SliverFillRemaining";
fn key(w: impl Into<Widget>, n: u64) -> Widget {
    w.into().with_key(Key::Value(n))
}
fn b(w: f32, h: f32) -> Widget {
    Widget::box_(Size::new(w, h), Color::WHITE)
}
fn preceding(height: f32) -> Box<dyn Sliver> {
    Box::new(SliverToBoxAdapter::new(b(150., height)))
}
fn list() -> Box<dyn Sliver> {
    Box::new(SliverFixedExtentList::new(5, 150., |i| {
        Semantics::new(b(150., 150.)).label(i.to_string())
    }))
}
fn plain_fill(child: Widget, overscroll: bool) -> Box<dyn Sliver> {
    Box::new(
        SliverFillRemaining::new(child)
            .has_scroll_body(false)
            .fill_overscroll(overscroll),
    )
}
struct Host {
    r: Runtime,
    c: ScrollController,
    now: Instant,
}
impl Host {
    fn new(lead: Box<dyn Sliver>, child: Widget, overscroll: bool, bounce: bool) -> Self {
        let c = ScrollController::new();
        let physics = if bounce {
            ScrollPhysics::BOUNCING
        } else {
            ScrollPhysics::CLAMPING
        };
        let viewport = CustomScrollView::new(vec![lead, plain_fill(child, overscroll)])
            .cache_extent(250.)
            .physics(physics.always_scrollable())
            .controller(c.clone());
        let r = Runtime::new(Directionality::new(TextDirection::Ltr, viewport).into()).unwrap();
        let mut host = Self {
            r,
            c,
            now: Instant::now(),
        };
        host.pump();
        host
    }
    fn pump(&mut self) {
        self.r
            .run_frame_at(Constraints::tight(VIEW), self.now)
            .unwrap();
        self.now += Duration::from_millis(16);
    }
    fn settle(&mut self) {
        for _ in 0..120 {
            self.pump();
        }
    }
    fn jump_end(&mut self) {
        let _ = self.c.jump_to(self.c.max_offset());
        self.pump();
    }
    fn swipe(&mut self) {
        for (phase, y) in [
            (PointerPhase::Down, 300.),
            (PointerPhase::Move, 280.),
            (PointerPhase::Move, 265.),
            (PointerPhase::Move, 250.),
            (PointerPhase::Up, 250.),
        ] {
            self.r.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: 1,
                    device: 0,
                    kind: PointerDeviceKind::Touch,
                    buttons: if phase == PointerPhase::Up {
                        0
                    } else {
                        PRIMARY_POINTER_BUTTON
                    },
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::EMPTY,
                    phase,
                    position: Offset::new(400., y),
                },
                self.now,
            );
            self.now += Duration::from_millis(16);
        }
        self.pump();
    }
    fn bounds(&self, k: u64) -> Option<Rect> {
        let t = self.r.tree();
        t.element_with_key(&Key::Value(k))
            .and_then(|i| t.element_bounds(i))
    }
    fn require(&self, k: u64) -> Rect {
        self.bounds(k).expect("fixture layout bounds")
    }
    fn height(&self) -> f32 {
        self.require(FILL).size.height
    }
    fn button_center_x_and_bottom(&self) -> (f32, f32) {
        let r = self.require(BUTTON);
        (r.origin.x + r.size.width / 2., r.origin.y + r.size.height)
    }
    fn semantics(&self, label: &str) -> Option<bool> {
        let mut matches = self
            .r
            .tree()
            .semantics()
            .iter()
            .filter(|(_, node)| node.label.as_deref() == Some(label));
        let present = matches.next().map(|(_, node)| node.state.hidden);
        assert!(
            matches.next().is_none(),
            "expected at most one node per label"
        );
        present
    }
    fn onscreen(&self, k: u64) -> bool {
        self.bounds(k).is_some_and(|r| {
            r.size.height > 0. && r.origin.y + r.size.height > 0. && r.origin.y < 600.
        })
    }
}
fn bottom_button(height: f32) -> Widget {
    key(
        Align::new(Alignment::BOTTOM_CENTER, key(b(116., height), BUTTON)),
        FILL,
    )
}
fn centered_button(height: f32) -> Widget {
    key(
        Align::new(
            Alignment::CENTER,
            Padding::all(50., key(b(116., height), BUTTON)),
        ),
        FILL,
    )
}
fn flex(height: f32) -> Widget {
    let logo = Center::new(key(b(100., 100.), LOGO));
    key(
        Column::new([
            Widget::from(Flexible::new(logo)),
            key(b(116., height), BUTTON),
        ])
        .main_axis_size(MainAxisSize::Min),
        FILL,
    )
}

#[test]
fn bouncing_drag_stretches_expand_child_and_springs_back() {
    let mut h = Host::new(
        preceding(150.),
        SizedBox::expand().child(key(b(0., 0.), FILL)).into(),
        true,
        true,
    );
    let before = h.height();
    h.swipe();
    let (offset, height) = (h.c.offset(), h.height());
    h.settle();
    assert_eq!((before, h.height()), (450., 450.));
    assert!(
        offset > 0. && height > 450.,
        "drag offset={offset}, child height={height}"
    );
}
#[test]
fn bouncing_drag_keeps_bottom_aligned_child_at_viewport_bottom() {
    let mut h = Host::new(preceding(150.), bottom_button(48.), true, true);
    assert_eq!(h.height(), 450.);
    h.swipe();
    let (offset, height, button) = (h.c.offset(), h.height(), h.button_center_x_and_bottom());
    h.settle();
    assert_eq!(h.height(), 450.);
    assert!(
        offset > 0. && height > 450. && button == (400., 600.),
        "offset={offset}, height={height}, button={button:?}"
    );
}
#[test]
fn bouncing_drag_grows_intrinsic_child_past_a_long_list() {
    let actual = [48., 40.].map(|base| {
        let mut h = Host::new(list(), centered_button(base), true, true);
        let max = h.c.max_offset();
        assert_eq!(max, 150. + 100. + base);
        h.jump_end();
        let before = (h.height(), h.button_center_x_and_bottom());
        h.swipe();
        let during = (
            h.c.offset() - h.c.max_offset(),
            h.height(),
            h.button_center_x_and_bottom(),
        );
        assert_eq!(h.c.max_offset(), max, "stretch must not change scroll range");
        h.settle();
        assert_eq!(h.c.offset(), max);
        assert_eq!(h.c.max_offset(), max);
        (before, during, h.height())
    });
    let mut mismatches = Vec::new();
    for (button_height, (before, (offset, height, button), settled)) in
        [48., 40.].into_iter().zip(actual)
    {
        if !(before == (100. + button_height, (400., 550.))
            && offset > 0.
            && height > 148.
            && button.0 == 400.
            && button.1 < 550.
            && settled == 100. + button_height)
        {
            mismatches.push((button_height, before, offset, height, button, settled));
        }
    }
    assert!(
        mismatches.is_empty(),
        "button height, before, overscroll, extent, button, settled: {mismatches:?}"
    );
}
fn zero_fill(bounce: bool) -> [(bool, bool); 4] {
    let mut h = Host::new(list(), key(b(0., 0.), FILL), true, bounce);
    let observe = |h: &Host| (h.onscreen(FILL), h.semantics("4") == Some(false));
    let a = observe(&h);
    h.jump_end();
    let b = observe(&h);
    h.swipe();
    let c = observe(&h);
    h.settle();
    [a, b, c, observe(&h)]
}
#[test]
fn zero_natural_fill_respects_last_row_semantics_before_and_after_scroll() {
    let actual = [true, false].map(zero_fill);
    for (i, observations) in actual.iter().enumerate() {
        let [initial, at_end, dragged, settled] = *observations;
        assert!(!initial.0 && !at_end.0 && !settled.0);
        if i == 1 {
            assert!(!dragged.0, "clamping must not expose extra fill area");
        }
    }
    // The Flutter finder also checks element presence while bouncing. Do
    // not equate that finder with positive paint geometry for a zero-sized box.
    assert_eq!(
        actual.map(|phases| phases.map(|(_, visible)| visible)),
        [[false, true, true, true]; 2]
    );
}
#[test]
fn clamping_ignores_fill_overscroll_for_zero_and_small_children() {
    for child in [key(b(0., 0.), FILL), bottom_button(48.)] {
        let mut h = Host::new(preceding(150.), child, true, false);
        let before = h.height();
        let button = h.bounds(BUTTON).map(|_| h.button_center_x_and_bottom());
        h.swipe();
        let during = h.height();
        let after_button = h.bounds(BUTTON).map(|_| h.button_center_x_and_bottom());
        assert_eq!(
            (before, during, h.c.offset(), button, after_button),
            (450., 450., 0., button, button)
        );
        if let Some(button) = button {
            assert_eq!(button, (400., 600.));
        }
    }
}
#[test]
fn clamping_preserves_the_intrinsic_child_end_position_during_drag() {
    let mut h = Host::new(list(), centered_button(48.), true, false);
    h.jump_end();
    let before = (h.height(), h.button_center_x_and_bottom());
    h.swipe();
    assert_eq!(
        (before, (h.height(), h.button_center_x_and_bottom())),
        ((148., (400., 550.)), (148., (400., 550.)))
    );
}
#[test]
fn fill_overscroll_semantics_still_hide_only_while_in_cache() {
    let actual = [500., 600., 850.].map(|lead| {
        let child = key(SizedBox::new().height(100.).child(Text::new(LABEL)), FILL);
        let h = Host::new(preceding(lead), child, true, true);
        if let Some(r) = h.bounds(FILL) {
            assert_eq!(r.origin.y, lead);
        }
        h.semantics(LABEL)
    });
    assert_eq!(actual, [Some(false), Some(true), None]);
}
#[test]
fn rest_alignment_of_flexible_child_is_independent_of_fill_overscroll() {
    for overscroll in [false, true] {
        for height in [48., 40.] {
            let h = Host::new(preceding(150.), flex(height), overscroll, true);
            let logo = h.require(LOGO);
            let button = h.require(BUTTON);
            assert_eq!(
                (
                    h.height(),
                    logo.size,
                    logo.origin.y + 50.,
                    button.size,
                    h.button_center_x_and_bottom()
                ),
                (
                    450.,
                    Size::new(100., 100.),
                    150. + (450. - height) / 2.,
                    Size::new(116., height),
                    (400., 600.)
                )
            );
        }
    }
}
#[test]
fn flexible_child_moves_with_bouncing_stretch_without_resizing_contents() {
    let mut mismatches = Vec::new();
    for height in [48., 40.] {
        let mut h = Host::new(preceding(150.), flex(height), true, true);
        h.swipe();
        let logo = h.require(LOGO);
        let button = h.require(BUTTON);
        let (offset, extent, button_pos) =
            (h.c.offset(), h.height(), h.button_center_x_and_bottom());
        h.settle();
        let restored = h.require(LOGO);
        if !(offset > 0.
            && extent > 450.
            && logo.size == Size::new(100., 100.)
            && logo.origin.y + 50. < 351.
            && button.size == Size::new(116., height)
            && button_pos == (400., 600.)
            && h.height() == 450.
            && restored.origin.y + 50. == 150. + (450. - height) / 2.)
        {
            mismatches.push((height, offset, extent, logo, button, restored));
        }
    }
    assert!(
        mismatches.is_empty(),
        "button height, offset, extent, logo, button, restored: {mismatches:?}"
    );
}
