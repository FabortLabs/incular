use incular_config::Constraints;
use incular_core::Size;
use incular_platform::{
    TextInputCommand,
    autofill::{AutofillContextAction, FlutterAutofillScope},
    message_codecs::JsonMethodCodec,
};
use incular_runtime::{Runtime, Signal};
use incular_text::TextEditingController;
use incular_widgets::{AutofillGroup, Builder, Column, EditableText, TextDirection, Widget};

fn field() -> Widget {
    EditableText::new(TextEditingController::new())
        .text_direction(TextDirection::Ltr)
        .autofill_hints(Some(["name"]))
        .into()
}

fn group(id: u32, inner: bool, action: AutofillContextAction) -> Widget {
    let mut child = field();
    if inner {
        child = AutofillGroup::new(child).into();
    }
    Widget::from(AutofillGroup::new(child).on_dispose_action(action)).with_key(u64::from(id))
}

#[test]
fn disposing_only_top_level_groups_emits_their_current_commit_or_cancel() {
    use AutofillContextAction::{Cancel, Commit};
    let state = Signal::new(0);
    let mut runtime = Runtime::new(
        Builder::new({
            let state = state.clone();
            move |_| {
                let page = state.get();
                let mut groups = Vec::new();
                if page == 0 {
                    groups.push(group(1, true, Commit));
                }
                if page < 2 {
                    groups.push(group(2, false, Cancel));
                }
                if page < 4 {
                    groups.push(group(3, page < 3, Commit));
                }
                Column::new(groups).into()
            }
        })
        .into(),
    )
    .unwrap();
    let limits = Constraints::loose(Size::new(300.0, 400.0));
    runtime.run_frame(limits).unwrap();
    assert!(runtime.take_text_input_commands().is_empty());
    for (page, expected) in [
        (1, vec![true]),
        (2, vec![false]),
        (3, vec![]),
        (4, vec![true]),
    ] {
        state.set(page);
        runtime.run_frame(limits).unwrap();
        let actual: Vec<_> = runtime
            .take_text_input_commands()
            .into_iter()
            .map(|command| {
                let TextInputCommand::FinishAutofillContext { action } = command else {
                    panic!("unexpected command {command:?}");
                };
                let wire = FlutterAutofillScope::encode_finish(action.should_save()).unwrap();
                let call = JsonMethodCodec::decode_method_call(&wire).unwrap();
                assert_eq!(call.method, "TextInput.finishAutofillContext");
                call.arguments.as_bool().unwrap()
            })
            .collect();
        assert_eq!(actual, expected);
    }
    assert!(runtime.take_text_input_commands().is_empty());
}
