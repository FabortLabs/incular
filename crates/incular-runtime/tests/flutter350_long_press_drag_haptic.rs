use std::{
    cell::{Cell, RefCell},
    future::{Future, ready},
    rc::Rc,
    time::{Duration, Instant},
};

use incular_config::Constraints;
use incular_core::{Color, InputEvent, Offset, PointerPhase, Size};
use incular_platform::{
    FlutterPlatformMessenger, HapticFeedback, HapticFeedbackType,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use incular_runtime::Runtime;
use incular_widgets::{DragDropContext, LongPressDraggable, Widget};

type Messages = Rc<RefCell<Vec<(&'static str, MethodCall<serde_json::Value>)>>>;

struct Messenger(Messages);

impl FlutterPlatformMessenger for Messenger {
    type Error = std::convert::Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        self.0.borrow_mut().push((
            channel,
            JsonMethodCodec::decode_method_call(&payload.expect("request")).unwrap(),
        ));
        ready(Ok(Some(b"[null]".to_vec())))
    }
}

#[test]
fn a_successfully_started_long_press_requests_only_the_selected_platform_haptic() {
    for setting in [None, Some(true), Some(false)] {
        let context = DragDropContext::new();
        let count = Rc::new(Cell::new(0));
        let mut source = LongPressDraggable::new(
            context.clone(),
            1,
            Widget::box_(Size::new(140., 50.), Color::BLACK),
        )
        .feedback(|_| Widget::box_(Size::new(30., 20.), Color::WHITE))
        .on_start({
            let count = count.clone();
            move |_| count.set(count.get() + 1)
        });
        if let Some(setting) = setting {
            source = source.haptic_feedback_on_start(setting);
        }
        let mut runtime = Runtime::new(source.into()).unwrap();
        let messages = Messages::default();
        let messenger = RefCell::new(HapticFeedback::new(Messenger(messages.clone())));
        runtime.set_haptic_feedback_handler(Some(Rc::new(move |effect| {
            assert_eq!(effect, HapticFeedbackType::SelectionClick);
            pollster::block_on(messenger.borrow_mut().request(Some(effect))).unwrap();
        })));
        let view = Constraints::tight(Size::new(140., 50.));
        let base = Instant::now();
        let frame = |runtime: &mut Runtime, millis| {
            runtime
                .run_frame_at(view, base + Duration::from_millis(millis))
                .unwrap();
        };
        let event = |runtime: &mut Runtime, phase, millis| {
            let _ = runtime.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: Offset::new(30., 20.),
                },
                base + Duration::from_millis(millis),
            );
            frame(runtime, millis);
        };
        frame(&mut runtime, 0);
        event(&mut runtime, PointerPhase::Down, 10);
        frame(&mut runtime, 509);
        assert!(messages.borrow().is_empty());
        assert_eq!(count.get(), 0);
        assert!(context.feedback().is_none());
        frame(&mut runtime, 510);
        assert_eq!(count.get(), 1);
        assert!(context.feedback().is_some());
        event(&mut runtime, PointerPhase::Up, 520);

        if setting == Some(false) {
            assert!(messages.borrow().is_empty());
        } else {
            assert_eq!(
                &*messages.borrow(),
                &[(
                    "flutter/platform",
                    MethodCall {
                        method: "HapticFeedback.vibrate".to_owned(),
                        arguments: serde_json::json!("HapticFeedbackType.selectionClick"),
                    }
                )]
            );
        }
    }
}
