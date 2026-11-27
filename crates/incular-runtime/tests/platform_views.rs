//! Flutter's platform view placement (support-001): after each frame the
//! runtime reports where every mounted platform view is, with the commands
//! its controller queued, and hides views that leave the tree.

use incular_config::{Constraints, EdgeInsets, TextDirection};
use incular_core::{Offset, PointerDeviceKind, PointerPhase, Rect, Size};
use incular_platform::{PlatformViewCommand, PlatformViewPointerEvent};
use incular_runtime::Runtime;
use incular_widgets::{Padding, PlatformViewController, PlatformViewSurface, SizedBox, Widget};

fn constraints() -> Constraints {
    Constraints::tight(Size::new(200., 120.))
}

fn padded(controller: &PlatformViewController, padding: f32) -> Widget {
    Padding::new(
        EdgeInsets::all(padding),
        PlatformViewSurface::new(controller.clone()),
    )
    .into()
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_origin_size(Offset::new(x, y), Size::new(width, height))
}

fn place(controller: &PlatformViewController, bounds: Rect, visible: bool) -> PlatformViewCommand {
    PlatformViewCommand::Place {
        id: controller.id(),
        bounds,
        visible_bounds: bounds,
        visible,
    }
}

#[test]
fn mounted_views_are_created_placed_moved_hidden_and_disposed() {
    let view = PlatformViewController::new("web", Vec::new(), TextDirection::Ltr);
    let mut runtime = Runtime::new(padded(&view, 10.)).unwrap();
    runtime.run_frame(constraints()).unwrap();
    let commands = runtime.take_platform_view_commands();
    assert!(matches!(&commands[0], PlatformViewCommand::Create { id, .. } if *id == view.id()));
    let first = rect(10., 10., 180., 100.);
    assert_eq!(commands[1..], [place(&view, first, true)]);

    // An unchanged frame sends nothing; a move sends the new placement.
    runtime.request_frame();
    runtime.run_frame(constraints()).unwrap();
    assert!(runtime.take_platform_view_commands().is_empty());
    let root = runtime.tree().root().expect("root");
    runtime.schedule_update(root, padded(&view, 20.)).unwrap();
    runtime.run_frame(constraints()).unwrap();
    assert_eq!(
        runtime.take_platform_view_commands(),
        [place(&view, rect(20., 20., 160., 80.), true)]
    );

    // Pointers forwarded between frames reach the host on the next take.
    view.dispatch_pointer_event(PlatformViewPointerEvent {
        pointer: 1,
        kind: PointerDeviceKind::Touch,
        phase: PointerPhase::Down,
        local_position: Offset::new(3., 4.),
        buttons: 1,
    });
    assert!(matches!(
        runtime.take_platform_view_commands()[..],
        [PlatformViewCommand::Pointer { .. }]
    ));

    // A view that leaves the tree is hidden, then disposed by its owner.
    runtime
        .schedule_update(
            root,
            Padding::new(EdgeInsets::all(20.), SizedBox::new()).into(),
        )
        .unwrap();
    runtime.run_frame(constraints()).unwrap();
    assert_eq!(
        runtime.take_platform_view_commands(),
        [place(&view, rect(20., 20., 160., 80.), false)]
    );
    view.dispose();
    assert_eq!(
        runtime.take_platform_view_commands(),
        [PlatformViewCommand::Dispose { id: view.id() }]
    );
    view.clear_focus();
    assert!(runtime.take_platform_view_commands().is_empty());
}

fn native_view() -> PlatformViewController {
    PlatformViewController::new("fixture view", Vec::new(), TextDirection::Ltr)
}

fn mount_view(view: &PlatformViewController) -> Runtime {
    Runtime::new(PlatformViewSurface::new(view.clone()).into()).unwrap()
}

#[test]
fn platform_view_fills_largest_allowed_size_after_constraints_change() {
    let view = native_view();
    let mut runtime = mount_view(&view);
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = runtime.take_platform_view_commands();
    runtime
        .run_frame(Constraints::new(50., 100., 50., 100.))
        .unwrap();
    let expected = rect(0., 0., 100., 100.);
    assert_eq!(
        runtime.tree().platform_view_placements()[0].bounds,
        expected
    );
    assert_eq!(
        runtime.take_platform_view_commands(),
        [place(&view, expected, true)]
    );
}

#[test]
fn platform_view_hover_dispatches_through_the_mounted_controller() {
    use incular_core::{InputEvent, PointerSampleMetadata};
    use incular_platform::{HoverDeviceChange, HoverDeviceEvent, PlatformEvent};
    let view = native_view();
    let mut runtime = mount_view(&view);
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let _ = runtime.take_platform_view_commands();
    let hover = Offset::new(10., 10.);
    for (position, change) in [
        (Offset::ZERO, HoverDeviceChange::Added),
        (hover, HoverDeviceChange::Removed),
    ] {
        if change == HoverDeviceChange::Removed {
            let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
                pointer: 0,
                device: 0,
                kind: PointerDeviceKind::Mouse,
                buttons: 0,
                button: None,
                sample: PointerSampleMetadata::default(),
                phase: PointerPhase::Move,
                position: hover,
            });
        }
        let _ = runtime.handle_platform_event(PlatformEvent::HoverDevice(HoverDeviceEvent {
            pointer: 0,
            device: 0,
            kind: PointerDeviceKind::Mouse,
            position,
            change,
        }));
    }
    assert_eq!(
        runtime.take_platform_view_commands(),
        [PlatformViewCommand::Pointer {
            id: view.id(),
            event: PlatformViewPointerEvent {
                pointer: 0,
                kind: PointerDeviceKind::Mouse,
                phase: PointerPhase::Move,
                local_position: hover,
                buttons: 0,
            }
        }]
    );
}

#[test]
fn attached_but_unlaid_view_does_not_report_a_screen_offset() {
    let view = native_view();
    let mut runtime = mount_view(&view);
    assert!(runtime.take_platform_view_commands().is_empty());

    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let commands = runtime.take_platform_view_commands();
    assert!(matches!(
        commands.as_slice(),
        [
            PlatformViewCommand::Create { .. },
            PlatformViewCommand::Place { .. }
        ]
    ));
    assert_eq!(commands[1], place(&view, rect(0., 0., 800., 600.), true));
}

#[test]
fn swapping_controller_before_first_layout_reports_no_premature_size() {
    let old = native_view();
    let view = native_view();
    let mut runtime = mount_view(&old);
    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, PlatformViewSurface::new(view.clone()).into())
        .unwrap();
    assert!(runtime.take_platform_view_commands().is_empty());

    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    let commands = runtime.take_platform_view_commands();
    assert!(
        matches!(commands.as_slice(), [PlatformViewCommand::Create { id, .. }, PlatformViewCommand::Place { .. }] if *id == view.id())
    );
    assert_eq!(commands[1], place(&view, rect(0., 0., 800., 600.), true));
}
