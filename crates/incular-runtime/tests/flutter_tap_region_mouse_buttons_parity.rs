//! Mounted counterparts of Flutter 3.50.0's inside/outside right- and
//! middle-click TapRegionSurface tests. Button data starts normalized.

use incular_config::Constraints;
use incular_core::{
    Color, InputEvent, Offset, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase,
    PointerSampleMetadata, Rect, SECONDARY_POINTER_BUTTON, Size, TERTIARY_POINTER_BUTTON,
};
use incular_runtime::Runtime;
use incular_widgets::{
    Column, Directionality, Listener, Row, TapRegion, TapRegionSurface, TextDirection, Widget,
    internal::Key,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Spot {
    Solo,
    A,
    B,
    Outside,
    OutsideSurface,
}

impl Spot {
    fn key(self) -> Key {
        Key::from(match self {
            Self::Solo => "ungrouped",
            Self::A => "group member a",
            Self::B => "group member b",
            Self::Outside => "outside",
            Self::OutsideSurface => "outside surface",
        })
    }

    fn widget(self) -> Widget {
        Widget::box_(Size::new(70., 32.), Color::WHITE).with_key(self.key())
    }
}

#[derive(Clone, Copy)]
enum Callback {
    Inside,
    Outside,
}

fn assert_mouse_button_classification(callback: Callback, button: u32) {
    use Spot::{A, B, Outside, OutsideSurface, Solo};

    let reported = Rc::new(RefCell::new(Vec::new()));
    let raw = Rc::new(RefCell::new(Vec::new()));
    let region = |spot, grouped| {
        let mut region = TapRegion::new(Spot::widget(spot));
        if grouped {
            region = region.group_id(1_u64);
        }
        let observed = Rc::clone(&reported);
        let record = move |_| observed.borrow_mut().push(spot);
        Widget::from(match callback {
            Callback::Inside => region.on_tap_inside(record),
            Callback::Outside => region.on_tap_outside(record),
        })
    };
    let surface = TapRegionSurface::new(Row::new([
        Outside.widget(),
        region(Solo, false),
        region(A, true),
        region(B, true),
    ]));
    let root = Column::new([OutsideSurface.widget(), surface.into()]);
    let raw_down = Rc::clone(&raw);
    let raw_up = Rc::clone(&raw);
    let root = Listener::new(root)
        .on_pointer_down(move |event| {
            raw_down
                .borrow_mut()
                .push((event.phase, event.button, event.buttons))
        })
        .on_pointer_up(move |event| {
            raw_up
                .borrow_mut()
                .push((event.phase, event.button, event.buttons))
        });
    let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, root).into())
        .expect("mount neutral tap regions");
    let mut now = Instant::now();
    runtime
        .run_frame_at(Constraints::tight(Size::new(600., 300.)), now)
        .expect("lay out tap regions");

    let spots = [Solo, A, B, Outside, OutsideSurface];
    let expected = match callback {
        Callback::Inside => [vec![Solo], vec![A, B], vec![A, B], vec![], vec![]],
        Callback::Outside => [vec![A, B], vec![Solo], vec![Solo], vec![Solo, A, B], vec![]],
    };
    let mut pointer = 0;
    let mut click = |spot: Spot, button| {
        let tree = runtime.tree();
        let id = tree.element_with_key(&spot.key()).expect("mounted target");
        let (transform, size) = tree.element_geometry(id).expect("target geometry");
        let rect = transform.transform_rect_bbox(Rect::from_origin_size(Offset::ZERO, size));
        let position = Offset::new(
            rect.origin.x + rect.size.width / 2.,
            rect.origin.y + rect.size.height / 2.,
        );
        pointer += 1;
        for (phase, buttons) in [(PointerPhase::Down, button), (PointerPhase::Up, 0)] {
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer,
                    device: 1,
                    kind: PointerDeviceKind::Mouse,
                    buttons,
                    button: Some(button),
                    sample: PointerSampleMetadata::default(),
                    phase,
                    position,
                },
                now,
            );
            now += Duration::from_millis(16);
        }
        assert_eq!(
            raw.take(),
            [
                (PointerPhase::Down, Some(button), button),
                (PointerPhase::Up, Some(button), 0)
            ],
            "raw mouse metadata reaches the same mounted pointer listener"
        );
        let mut events = reported.take();
        events.sort_unstable();
        events
    };

    let primary = spots.map(|spot| click(spot, PRIMARY_POINTER_BUTTON));
    assert_eq!(primary, expected, "primary click control");
    let nonprimary = spots.map(|spot| click(spot, button));
    assert_eq!(
        nonprimary, expected,
        "the region classifies mouse button {button}"
    );
}

#[test]
fn tap_surface_reports_outside_on_secondary_button() {
    assert_mouse_button_classification(Callback::Outside, SECONDARY_POINTER_BUTTON);
}

#[test]
fn tap_surface_reports_outside_on_middle_button() {
    assert_mouse_button_classification(Callback::Outside, TERTIARY_POINTER_BUTTON);
}

#[test]
fn tap_surface_reports_inside_on_secondary_button() {
    assert_mouse_button_classification(Callback::Inside, SECONDARY_POINTER_BUTTON);
}

#[test]
fn tap_surface_reports_inside_on_middle_button() {
    assert_mouse_button_classification(Callback::Inside, TERTIARY_POINTER_BUTTON);
}
