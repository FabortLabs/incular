use std::{
    convert::Infallible,
    future::{Future, ready},
    pin::pin,
    task::{Context, Poll, Waker},
};

use incular_config::{Alignment, Constraints, EdgeInsets, TextDirection};
use incular_core::Size;
use incular_platform::{
    FlutterPlatformMessenger, SystemContextMenuCommand, SystemContextMenuController,
    SystemContextMenuError, SystemContextMenus, TextInputCommand,
    message_codecs::{JsonMethodCodec, MethodCall},
};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{
    TextAlign, TextEditingController, TextSelection, TextStyle, register_application_font_as,
};
use incular_widgets::{Align, EditableText, FocusNode, Padding, SizedBox};
use serde_json::Value;

#[derive(Default)]
struct Wire(Vec<MethodCall<Value>>);

impl FlutterPlatformMessenger for Wire {
    type Error = Infallible;

    fn send_platform_message(
        &mut self,
        channel: &'static str,
        payload: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
        assert_eq!(channel, "flutter/platform");
        self.0
            .push(JsonMethodCodec::decode_method_call(&payload.expect("method call bytes")).unwrap());
        ready(Ok(None))
    }
}

impl Wire {
    fn send(&mut self, command: SystemContextMenuCommand) {
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(
            pin!(command.send(self)).poll(&mut cx),
            Poll::Ready(Ok(()))
        ));
    }

    fn check_show(&mut self, expected: [f64; 4]) {
        let calls = std::mem::take(&mut self.0);
        let [call] = &calls[..] else {
            panic!("expected exactly one platform menu command: {calls:?}")
        };
        assert_eq!(call.method, "ContextMenu.showSystemContextMenu");
        let types: Vec<_> = call.arguments["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["type"].as_str().unwrap())
            .collect();
        assert_eq!(types, ["cut", "copy", "paste", "selectAll"]);
        for (key, value) in ["x", "y", "width", "height"].into_iter().zip(expected) {
            let found = call.arguments["targetRect"][key].as_f64().unwrap();
            assert!(
                (found - value).abs() < 0.002,
                "{key}: want {value}, got {found}. {call:?}"
            );
        }
    }
}

fn frame(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
}

#[test]
fn focused_editor_supplies_legacy_default_items_with_an_attached_client() {
    let value = TextEditingController::with_text("test");
    let focus = FocusNode::new();
    let editor = EditableText::new(value.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .max_lines(Some(10))
        .text_direction(TextDirection::Ltr)
        .text_align(TextAlign::Right);
    let mut runtime = Runtime::new(
        Align::new(
            Alignment::TOP_LEFT,
            SizedBox::from_size(Size::new(400., 30.)).child(editor),
        )
        .into(),
    )
    .unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    runtime.set_clipboard_text("Clipboard data");
    let mut wire = Wire::default();
    let mut menus = SystemContextMenus::default();
    let controller = SystemContextMenuController::new();
    let mut no_editor = Runtime::new(SizedBox::shrink().into()).unwrap();
    assert!(matches!(
        no_editor.show_focused_system_context_menu(&mut menus, &controller),
        Err(SystemContextMenuError::NoTextInput)
    ));
    assert!(!menus.is_visible(&controller));
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    assert!(
        runtime
            .take_text_input_commands()
            .iter()
            .any(|command| matches!(command, TextInputCommand::SetClient { .. }))
    );

    let field = runtime.focused_element().unwrap();
    let expected = runtime.tree().editable_selection_menu_rect(field).unwrap();
    let command = runtime
        .show_focused_system_context_menu(&mut menus, &controller)
        .unwrap()
        .unwrap();
    assert!(menus.is_visible(&controller));
    wire.send(command);
    wire.check_show([
        expected.origin.x as f64,
        expected.origin.y as f64,
        expected.size.width as f64,
        expected.size.height as f64,
    ]);
    assert!(focus.has_primary_focus());
}

#[test]
fn ahem_menu_targets_caret_single_line_and_multiline_selection_exactly() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    const FAMILY: &str = "Pinned system menu Ahem";
    register_application_font_as(FAMILY, std::fs::read(path).expect("read pinned Ahem"));
    let value = TextEditingController::with_text("test\nwide");
    let focus = FocusNode::new();
    let editor = EditableText::new(value.clone())
        .focus_node(focus.clone())
        .autofocus(true)
        .max_lines(Some(10))
        .text_direction(TextDirection::Ltr)
        .text_align(TextAlign::Right)
        .style(TextStyle::new().font_family(FAMILY).font_size(10.));
    let widget = Align::new(
        Alignment::TOP_LEFT,
        Padding::new(
            EdgeInsets::only(13., 17., 0., 0.),
            SizedBox::from_size(Size::new(400., 20.)).child(editor),
        ),
    );
    let mut runtime = Runtime::new(widget.into()).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    frame(&mut runtime);
    assert!(focus.has_primary_focus());
    let mut wire = Wire::default();
    let mut menus = SystemContextMenus::default();
    let controller = SystemContextMenuController::new();
    for (selection, expected) in [
        (
            TextSelection::new(2, 2),
            [390.0000305175781, 17.0, 0.0, 10.0],
        ),
        (
            TextSelection::new(1, 3),
            [380.00006103515625, 17.0, 19.99993896484375, 10.0],
        ),
        (TextSelection::new(1, 7), [13.0, 17.0, 400.0, 20.0]),
        (TextSelection::new(7, 1), [13.0, 17.0, 400.0, 20.0]),
    ] {
        value.set_selection(selection);
        frame(&mut runtime);
        if let Some(hide) = menus.hide(&controller) {
            wire.send(hide);
            wire.0.clear();
        }
        wire.send(
            runtime
                .show_focused_system_context_menu(&mut menus, &controller)
                .unwrap()
                .unwrap(),
        );
        wire.check_show(expected);
    }
}
