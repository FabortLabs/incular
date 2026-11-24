//! Public retained OverlayPortal comparisons with Flutter 3.50.0.
//! Incular uses a declarative show flag and a view-local transient rather
//! than Flutter's imperative controller attached to an explicit OverlayEntry.

use incular_config::{Constraints, EdgeInsets};
use incular_core::{Color, InputEvent, Offset, PointerPhase, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Directionality, GestureDetector, OverlayPortal, SizedBox, TextDirection,
    TransientPlacement, TransientPresentation, TransientSide, Widget,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

struct Host {
    runtime: Runtime,
    now: Instant,
}

impl Host {
    fn mount(widget: Widget) -> Self {
        let mut host = Self {
            runtime: Runtime::new(widget).expect("mount portal scene"),
            now: Instant::now(),
        };
        host.frame();
        host
    }

    fn frame(&mut self) {
        self.runtime
            .run_frame_at(Constraints::tight(Size::new(800., 600.)), self.now)
            .expect("run portal frame");
        self.now += Duration::from_millis(16);
    }

    fn tap(&mut self, position: Offset) {
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            let _ = self
                .runtime
                .handle_input_at(InputEvent::Pointer { phase, position }, self.now);
            self.now += Duration::from_millis(1);
        }
    }
}

fn portal(child: impl Into<Widget>) -> OverlayPortal {
    OverlayPortal::new(child).presentation(TransientPresentation::Overlay)
}

/// overlay_portal_test.dart:765. Observe the subtree only after each pump;
/// the revision advances the retained host without replacing the root.
#[test]
fn portal_mounts_and_unmounts_its_child_after_show_and_hide() {
    let show = Rc::new(Cell::new(false));
    let revision = Rc::new(Cell::new(0_u64));
    let root = Widget::stateful_builder(revision.clone(), {
        let show = show.clone();
        move |_| {
            portal(SizedBox::shrink())
                .overlay_child(
                    Widget::box_(Size::new(1., 1.), Color::WHITE).with_key("portal target"),
                )
                .show(show.get())
                .into()
        }
    });
    let mut host = Host::mount(root);
    let observe = |host: &Host| {
        (
            host.runtime
                .tree()
                .element_with_key(&"portal target".into())
                .is_some(),
            host.runtime.transient_surfaces().len(),
        )
    };
    assert_eq!(observe(&host), (false, 0));
    host.frame();
    assert_eq!(observe(&host), (false, 0));
    let mut observed = Vec::new();
    for visible in [true, false, true] {
        show.set(visible);
        revision.set(revision.get() + 1);
        host.frame();
        observed.push(observe(&host));
    }
    assert_eq!(observed, [(true, 1), (false, 0), (true, 1)]);
}

/// overlay_portal_test.dart:810. The overlay subtree builder is a public
/// Builder passed as a child, not an identically named callback on the portal.
#[test]
fn hidden_portal_does_not_evaluate_its_overlay_subtree_builder() {
    let show = Rc::new(Cell::new(false));
    let revision = Rc::new(Cell::new(0_u64));
    let built = Rc::new(Cell::new(0));
    let root = Widget::stateful_builder(revision.clone(), {
        let (show, built) = (show.clone(), built.clone());
        move |_| {
            let built = built.clone();
            portal(SizedBox::shrink())
                .overlay_child(Builder::new(move |_| {
                    built.set(built.get() + 1);
                    SizedBox::shrink().into()
                }))
                .show(show.get())
                .into()
        }
    });
    let mut host = Host::mount(root);
    assert_eq!(built.get(), 0);
    host.frame();
    assert_eq!(built.get(), 0);
    show.set(true);
    revision.set(1);
    host.frame();
    assert_eq!(
        built.get(),
        1,
        "positive control: visible builder is called"
    );
}

/// overlay_portal_test.dart:104. The overlay child builds in the portal's
/// declaration context, inside the local Directionality, not the outer one.
#[test]
fn portal_child_reads_updated_directionality_at_declaration_site() {
    let direction = Rc::new(Cell::new(TextDirection::Rtl));
    let revision = Rc::new(Cell::new(0_u64));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let inner = Widget::stateful_builder(revision.clone(), {
        let (direction, seen) = (direction.clone(), seen.clone());
        move |_| {
            let seen = seen.clone();
            Directionality::new(
                direction.get(),
                portal(SizedBox::shrink())
                    .overlay_child(Builder::new(move |context| {
                        seen.borrow_mut().push(context.text_direction());
                        SizedBox::shrink().into()
                    }))
                    .show(true),
            )
            .into()
        }
    });
    let mut host = Host::mount(Directionality::new(TextDirection::Ltr, inner).into());
    assert_eq!(*seen.borrow(), [Some(TextDirection::Rtl)]);
    direction.set(TextDirection::Ltr);
    revision.set(1);
    host.frame();
    assert_eq!(
        *seen.borrow(),
        [Some(TextDirection::Rtl), Some(TextDirection::Ltr)]
    );
}

/// overlay_portal_test.dart:893. Both targets are 1x1. Use a zero-margin
/// explicit anchor point to place it at Flutter's two absolute coordinates;
/// the test compares hit routing, not default anchored placement.
#[test]
fn portal_child_hit_moves_to_its_new_position_on_the_next_frame() {
    let offset = Rc::new(Cell::new(0.));
    let revision = Rc::new(Cell::new(0_u64));
    let taps = Rc::new(Cell::new(0));
    let root = Widget::stateful_builder(revision.clone(), {
        let (offset, taps) = (offset.clone(), taps.clone());
        move |_| {
            let taps = taps.clone();
            portal(SizedBox::shrink())
                .overlay_child(
                    GestureDetector::new(Widget::box_(Size::new(1., 1.), Color::WHITE))
                        .on_tap(move || taps.set(taps.get() + 1)),
                )
                .anchor_point(Offset::new(offset.get(), offset.get()))
                .placement(
                    TransientPlacement::new()
                        .side(TransientSide::Bottom)
                        .safe_margin(EdgeInsets::ZERO),
                )
                .show(true)
                .into()
        }
    });
    let mut host = Host::mount(root);
    let rect = |host: &Host| host.runtime.transient_surfaces()[0].content_rect;
    assert_eq!(
        rect(&host),
        Rect::from_origin_size(Offset::ZERO, Size::new(1., 1.))
    );
    host.tap(Offset::new(0.5, 0.5));
    assert_eq!(taps.get(), 1);
    offset.set(50.);
    revision.set(1);
    host.frame();
    assert_eq!(
        rect(&host),
        Rect::from_origin_size(Offset::new(50., 50.), Size::new(1., 1.))
    );
    host.tap(Offset::new(0.5, 0.5));
    assert_eq!(taps.get(), 1);
    host.tap(Offset::new(50.5, 50.5));
    assert_eq!(taps.get(), 2);
}
