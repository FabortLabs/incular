use incular_config::{Alignment, Constraints, TextDirection};
use incular_core::Size;
use incular_platform::{Clipboard, MemoryClipboard, PhysicalSize, PlatformEvent, WindowMetrics};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{TextEditingController, TextSelection};
use incular_widgets::{Align, EditableText, FocusNode, SizedBox, Widget, internal::Key};
use serde_json::json;

const MENU: &str = "neutral rotation toolbar";

fn rotate(runtime: &mut Runtime, width: u32, height: u32, dpr: f64) {
    let metrics = WindowMetrics::new(PhysicalSize::new(width, height), dpr);
    let _ = runtime.handle_platform_event(PlatformEvent::Metrics(metrics));
    runtime
        .run_frame(Constraints::tight(metrics.logical_size()))
        .unwrap();
}

#[test]
fn only_mobile_orientation_changes_close_the_toolbar_and_android_also_hides_handles() {
    for (platform, menu_after, handles_after) in [
        (TextShortcutPlatform::Android, false, false),
        (TextShortcutPlatform::Ios, false, true),
        (TextShortcutPlatform::Linux, true, true),
    ] {
        let text = TextEditingController::new();
        let focus = FocusNode::new();
        let field = EditableText::new(text.clone())
            .focus_node(focus.clone())
            .autofocus(true)
            .text_direction(TextDirection::Ltr)
            .size(Size::new(300., 50.))
            .context_menu_builder(|_, _| Widget::from(SizedBox::shrink()).with_key(MENU))
            .selection_handles_builder(|_, _| SizedBox::shrink().into());
        let root = Align::new(Alignment::TOP_LEFT, field);
        let mut runtime = Runtime::new(root.into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        let mut clipboard = MemoryClipboard::default();
        clipboard.set_text("paste remains available".into());
        runtime.set_clipboard(Box::new(clipboard));
        rotate(&mut runtime, 2400, 1800, 3.);
        let field = runtime.focused_element().unwrap();
        text.set_selection(TextSelection::collapsed(0));
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let geometry = runtime
            .tree()
            .text_field_selection_handle_geometry(field)
            .unwrap();
        let handles = runtime.tree().text_field_selection_handles(field).unwrap();
        handles.show(geometry);
        let client = runtime.flutter_text_input_configuration().unwrap().0;
        let command = serde_json::to_vec(&json!({
            "method":"TextInputClient.showToolbar", "args":[client.get()]
        }))
        .unwrap();
        assert_eq!(
            runtime.handle_flutter_text_toolbar_call(&command).unwrap(),
            Some(true)
        );
        runtime
            .run_frame(Constraints::tight(Size::new(800., 600.)))
            .unwrap();
        let visible =
            |runtime: &Runtime| runtime.tree().element_with_key(&Key::from(MENU)).is_some();
        assert!(visible(&runtime), "{platform:?}");
        assert!(handles.is_showing());

        rotate(&mut runtime, 2250, 1800, 3.);
        assert!(
            visible(&runtime),
            "landscape resize should keep the mounted toolbar"
        );
        assert!(handles.is_showing());
        rotate(&mut runtime, 1800, 2400, 3.);
        assert_eq!(visible(&runtime), menu_after, "{platform:?}");
        assert_eq!(handles.is_showing(), handles_after, "{platform:?}");
        assert_eq!(runtime.focused_element(), Some(field));
        assert_eq!(
            runtime.flutter_text_input_configuration().unwrap().0,
            client
        );
        assert!(focus.has_primary_focus());
        assert_eq!(text.selection(), TextSelection::collapsed(0));
    }
}
