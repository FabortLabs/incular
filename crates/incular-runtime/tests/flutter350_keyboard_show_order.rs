use std::time::{Duration, Instant};

use incular_config::{Constraints, TextDirection};
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::{FlutterTextInputControls, TextInputCommand};
use incular_runtime::Runtime;
use incular_text::TextEditingController;
use incular_widgets::{Column, EditableText, FocusNode, Widget};

#[test]
fn switching_editors_publishes_layout_and_editing_state_before_keyboard_show() {
    let [first, second] = [FocusNode::new(), FocusNode::new()];
    let field = |focus: &FocusNode, text: &str| {
        Widget::from(
            EditableText::new(TextEditingController::with_text(text))
                .focus_node(focus.clone())
                .text_direction(TextDirection::Ltr)
                .max_lines(Some(2))
                .input_type(incular_platform::TextInputType::Text)
                .size(Size::new(200., 40.)),
        )
    };
    let mut runtime =
        Runtime::new(Column::new([field(&first, "old"), field(&second, "new")]).into()).unwrap();
    let view = Constraints::tight(Size::new(200., 120.));
    let base = Instant::now();
    runtime.run_frame_at(view, base).unwrap();
    let tap = |runtime: &mut Runtime, at, point| {
        for (index, phase) in [PointerPhase::Down, PointerPhase::Up]
            .into_iter()
            .enumerate()
        {
            let _ = runtime.handle_input_at(
                InputEvent::Pointer {
                    phase,
                    position: point,
                },
                base + Duration::from_millis(at + index as u64),
            );
        }
        runtime
            .run_frame_at(view, base + Duration::from_millis(at + 2))
            .unwrap();
    };

    let mut controls = FlutterTextInputControls::new(false);
    tap(&mut runtime, 10, Offset::new(20., 10.));
    assert!(first.has_primary_focus() && !second.has_focus());
    let (previous, config) = runtime.flutter_text_input_configuration().unwrap();
    let commands = runtime.take_text_input_commands();
    assert!(
        commands
            .iter()
            .any(|command| *command == TextInputCommand::Show { client: previous })
    );
    assert!(controls.attach_existing(previous, config, |_| {}));
    assert!(
        runtime
            .show_flutter_text_input(previous, &mut controls)
            .unwrap()
    );
    controls.take_platform_calls();

    tap(&mut runtime, 30, Offset::new(20., 50.));
    assert!(!first.has_focus() && second.has_primary_focus());
    let (current, config) = runtime.flutter_text_input_configuration().unwrap();
    assert_ne!(current, previous);
    let commands = runtime.take_text_input_commands();
    let attach = commands
        .iter()
        .position(|command| {
            matches!(
                command, TextInputCommand::SetClient { configuration, state, .. }
                  if configuration.client == current && state.text == "new"
            )
        })
        .unwrap();
    let show = commands
        .iter()
        .position(|command| *command == TextInputCommand::Show { client: current })
        .unwrap();
    assert!(attach < show);

    assert!(controls.close(previous));
    assert!(controls.attach_existing(current, config, |_| {}));
    assert!(
        !runtime
            .show_flutter_text_input(previous, &mut controls)
            .unwrap()
    );
    assert!(
        runtime
            .show_flutter_text_input(current, &mut controls)
            .unwrap()
    );
    assert!(!controls.flush_pending_hide());
    let wire = controls.take_platform_calls();
    assert_eq!(
        wire.iter().map(|call| call.method).collect::<Vec<_>>(),
        [
            "TextInput.clearClient",
            "TextInput.setClient",
            "TextInput.setEditableSizeAndTransform",
            "TextInput.setMarkedTextRect",
            "TextInput.setStyle",
            "TextInput.setEditingState",
            "TextInput.show",
            "TextInput.requestAutofill",
            "TextInput.setCaretRect",
        ]
    );
    assert_eq!(wire[5].arguments["text"], "new");
    assert_eq!(wire[1].arguments[0], current.get());

    assert!(
        runtime
            .show_flutter_text_input(current, &mut controls)
            .unwrap()
    );
    let calls = controls.take_platform_calls();
    assert_eq!(
        calls.iter().map(|call| call.method).collect::<Vec<_>>(),
        ["TextInput.show"]
    );
}
