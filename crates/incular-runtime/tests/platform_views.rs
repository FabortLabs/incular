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
