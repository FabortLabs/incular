use incular_config::Constraints;
use incular_core::{InputEvent, Offset, PointerPhase, Size};
use incular_platform::{PlatformEvent, TextInputAction, TextInputState};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextRange};
use incular_widgets::{Column, EditableText, TextDirection, Widget};
use std::{cell::RefCell, rc::Rc};

fn traverse(reverse: bool, custom_complete: bool) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let controllers = ["left", "right"].map(TextEditingController::with_text);
    let start = usize::from(reverse);
    let fields: [Widget; 2] = std::array::from_fn(|i| {
        let mut field = EditableText::new(controllers[i].clone())
            .text_direction(TextDirection::Ltr)
            .size(Size::new(180., 40.))
            .on_submit({
                let calls = calls.clone();
                move |text| calls.borrow_mut().push(format!("submit:{text}"))
            });
        if i == start && custom_complete {
            let calls = calls.clone();
            field = field.on_editing_complete(move || calls.borrow_mut().push("complete".into()));
        }
        field.into()
    });
    let mut runtime = Runtime::new(Column::new(fields).into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    let constraints = Constraints::tight(Size::new(180., 80.));
    runtime.run_frame(constraints).unwrap();
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        let _ = runtime.handle_input(InputEvent::Pointer {
            phase,
            position: Offset::new(130., if reverse { 60. } else { 20. }),
        });
    }
    let original = runtime.focused_element().unwrap();
    let text = controllers[start].text();
    let _ = runtime.handle_platform_event(PlatformEvent::TextEditingState(TextInputState {
        selection_start: text.len(),
        selection_end: text.len(),
        text: text.clone(),
        composing: Some((0, 2)),
    }));
    assert_eq!(controllers[start].composing(), Some(TextRange::new(0, 2)));
    let _ = runtime.handle_platform_event(PlatformEvent::TextInputAction(if reverse {
        TextInputAction::Previous
    } else {
        TextInputAction::Next
    }));
    runtime.run_frame(constraints).unwrap();
    let submitted = format!("submit:{text}");
    if custom_complete {
        assert_eq!(calls.take(), ["complete", submitted.as_str()]);
        assert_eq!(runtime.focused_element(), Some(original));
        assert_eq!(controllers[start].composing(), Some(TextRange::new(0, 2)));
    } else {
        assert_eq!(calls.take(), [submitted]);
        assert!(
            runtime
                .focused_element()
                .is_some_and(|next| next != original)
        );
        assert_eq!(controllers[start].composing(), None);
        let _ = runtime.handle_input(InputEvent::Text("!".into()));
        assert!(
            controllers[1 - start].text().contains('!'),
            "focus actually receives typing"
        );
    }
}

#[test]
fn custom_editing_complete_replaces_a_next_or_previous_focus_move() {
    for reverse in [false, true] {
        traverse(reverse, true);
    }
}

#[test]
fn default_next_and_previous_clear_composition_and_route_to_the_other_editor() {
    for reverse in [false, true] {
        traverse(reverse, false);
    }
}
