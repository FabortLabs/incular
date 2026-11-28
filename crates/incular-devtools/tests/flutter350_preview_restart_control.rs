use std::{
    future::{Future, ready},
    pin::pin,
    task::{Context, Poll, Waker},
    time::Duration,
};

use incular_config::{Constraints, TextDirection};
use incular_controls::widget_previews::previewer_restart_button;
use incular_core::{InputEvent, Offset, PointerPhase, Size, time::Instant};
use incular_devtools::preview_execution::{PreviewExecution, PreviewExecutionKind};
use incular_runtime::Runtime;
use incular_semantics::{Role, SemanticAction};
use incular_widgets::Directionality;

#[test]
fn mounted_previewer_control_requests_a_full_restart_through_the_host() {
    let mut execution = PreviewExecution::new();
    let requests = execution.requests();
    assert!(requests.connected());
    let mut runtime = Runtime::new(
        Directionality::new(
            TextDirection::Ltr,
            previewer_restart_button(move || {
                assert!(requests.request(PreviewExecutionKind::HotRestart));
            }),
        )
        .into(),
    )
    .unwrap();
    runtime.set_platform_semantics_enabled(true);
    let mut now = Instant::now();
    runtime
        .run_frame_at(Constraints::loose(Size::new(440., 60.)), now)
        .unwrap();

    let (button_id, button) = runtime
        .tree()
        .semantics()
        .iter()
        .find(|(_, node)| node.role == Role::Button)
        .unwrap();
    assert_eq!(
        button.label.as_deref(),
        Some("Restart the Widget Previewer")
    );
    let bounds = button.bounds;
    let point = bounds.origin + Offset::new(bounds.size.width / 2., bounds.size.height / 2.);

    let mut cx = Context::from_waker(Waker::noop());
    {
        let mut next = pin!(execution.perform_next(ready));
        assert_eq!(next.as_mut().poll(&mut cx), Poll::Pending);
        for phase in [PointerPhase::Down, PointerPhase::Up] {
            now += Duration::from_millis(1);
            let _ = runtime.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: point,
                },
                now,
            );
            let expectation = if phase == PointerPhase::Down {
                Poll::Pending
            } else {
                Poll::Ready(Some(PreviewExecutionKind::HotRestart))
            };
            assert_eq!(next.as_mut().poll(&mut cx), expectation);
        }
    }
    {
        let mut next = pin!(execution.perform_next(ready));
        assert_eq!(next.as_mut().poll(&mut cx), Poll::Pending);
        assert!(runtime.dispatch_semantic_action(button_id, SemanticAction::Activate));
        assert_eq!(
            next.as_mut().poll(&mut cx),
            Poll::Ready(Some(PreviewExecutionKind::HotRestart)),
        );
    }
    let mut next = pin!(execution.perform_next(ready));
    assert_eq!(next.as_mut().poll(&mut cx), Poll::Pending);
}
