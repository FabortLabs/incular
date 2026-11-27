//! Flutter's `TextSelectionGestureDetectorBuilder.onSecondaryTap` and
//! `EditableTextState.contextMenuButtonItems`: a secondary click on a field
//! with a `contextMenuBuilder` opens the platform's menu with the buttons the
//! selection and clipboard allow, and each button runs its action and closes
//! the menu.

use incular_config::Constraints;
use incular_core::{
    InputEvent, Offset, PointerDeviceKind, PointerPhase, PointerSampleMetadata,
    SECONDARY_POINTER_BUTTON, Size,
};
use incular_runtime::{Application, Runtime, TextShortcutPlatform};
use incular_widgets::{
    ContextMenuButtonType, EditableText, IgnorePointer, SizedBox, TextContextMenu, TextDirection,
    Widget,
    internal::{TextEditingController, TextSelection},
};
use std::{cell::RefCell, rc::Rc};

type Shown = Rc<RefCell<Vec<TextContextMenu>>>;

fn bounds() -> Constraints {
    Constraints::tight(Size::new(300., 40.))
}

fn mount(text: &str, platform: TextShortcutPlatform) -> (Runtime, TextEditingController, Shown) {
    mount_with(text, platform, |field| field)
}

fn mount_with(
    text: &str,
    platform: TextShortcutPlatform,
    configure: impl FnOnce(EditableText) -> EditableText,
) -> (Runtime, TextEditingController, Shown) {
    let controller = TextEditingController::with_text(text);
    let shown: Shown = Rc::default();
    let field = configure(EditableText::new(controller.clone()))
        .text_direction(TextDirection::Ltr)
        .size(Size::new(300., 32.))
        .context_menu_builder({
            let shown = shown.clone();
            move |_, menu| {
                shown.borrow_mut().push(menu.clone());
                // Like a real toolbar, the view-filling overlay itself is
                // not hit; only its content would be.
                IgnorePointer::new(SizedBox::shrink()).into()
            }
        });
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    (runtime, controller, shown)
}

fn secondary_click(runtime: &mut Runtime, position: Offset) {
    for (phase, buttons) in [
        (PointerPhase::Down, SECONDARY_POINTER_BUTTON),
        (PointerPhase::Up, 0),
    ] {
        let _ = runtime.handle_input(InputEvent::PointerWithMetadata {
            pointer: 1,
            device: 3,
            kind: PointerDeviceKind::Mouse,
            buttons,
            button: Some(SECONDARY_POINTER_BUTTON),
            sample: PointerSampleMetadata::default(),
            phase,
            position,
        });
    }
    runtime.run_frame(bounds()).unwrap();
}

fn types(menu: &TextContextMenu) -> Vec<ContextMenuButtonType> {
    menu.button_items
        .iter()
        .map(|item| item.button_type())
        .collect()
}

fn last(shown: &Shown) -> TextContextMenu {
    shown.borrow().last().cloned().expect("menu built")
}

/// Runs callbacks the menu's buttons queued for the UI owner.
fn process(runtime: Runtime) -> Runtime {
    let mut application = Application::from_runtime(runtime, |_| {});
    application.process_runtime_work();
    let mut runtime = application.into_runtime();
    runtime.run_frame(bounds()).unwrap();
    runtime
}

#[test]
fn windows_secondary_click_places_the_caret_and_offers_select_all_and_paste() {
    let (mut runtime, controller, shown) = mount("hello world", TextShortcutPlatform::Windows);
    runtime.set_clipboard_text("pasted ");
    secondary_click(&mut runtime, Offset::new(12., 10.));
    // An unfocused field collapses the selection at the click and takes focus.
    assert!(controller.selection().is_collapsed());
    assert!(runtime.focused_element().is_some());
    let menu = last(&shown);
    assert_eq!(menu.anchors.primary_anchor, Offset::new(12., 10.));
    // Nothing is selected, so there is nothing to cut or copy.
    assert_eq!(
        types(&menu),
        vec![
            ContextMenuButtonType::Paste,
            ContextMenuButtonType::SelectAll
        ]
    );
}

#[test]
fn copy_button_writes_the_selection_and_closes_the_menu() {
    let (mut runtime, controller, shown) = mount("hello world", TextShortcutPlatform::Linux);
    secondary_click(&mut runtime, Offset::new(12., 10.));
    controller.set_selection(TextSelection::new(0, 5));
    // A second click on the open menu's field toggles it closed...
    secondary_click(&mut runtime, Offset::new(12., 10.));
    let builds = shown.borrow().len();
    // ...and a third opens it with the selection's buttons.
    secondary_click(&mut runtime, Offset::new(12., 10.));
    assert!(shown.borrow().len() > builds);
    let menu = last(&shown);
    assert_eq!(
        types(&menu),
        vec![
            ContextMenuButtonType::Cut,
            ContextMenuButtonType::Copy,
            ContextMenuButtonType::SelectAll,
        ]
    );
    let copy = menu
        .button_items
        .iter()
        .find(|item| item.button_type() == ContextMenuButtonType::Copy)
        .unwrap()
        .clone();
    copy.press();
    let mut runtime = process(runtime);
    let clipboard = runtime
        .read_clipboard(incular_platform::TransferReadRequest::format(
            incular_platform::TransferFormat::PlainText,
        ))
        .unwrap()
        .materialize(&incular_platform::TransferFormat::PlainText)
        .unwrap();
    assert_eq!(
        clipboard,
        Some(incular_platform::TransferData::text("hello"))
    );
    // The menu closed: no further builds while frames run.
    let builds = shown.borrow().len();
    runtime.run_frame(bounds()).unwrap();
    assert_eq!(shown.borrow().len(), builds);
}

#[test]
fn macos_secondary_click_selects_the_word_and_never_offers_select_all() {
    let (mut runtime, controller, shown) = mount("hello world", TextShortcutPlatform::MacOs);
    secondary_click(&mut runtime, Offset::new(12., 10.));
    assert_eq!(controller.selection(), TextSelection::new(0, 5));
    assert_eq!(
        types(&last(&shown)),
        vec![ContextMenuButtonType::Cut, ContextMenuButtonType::Copy]
    );
}

#[test]
fn a_field_without_a_builder_still_moves_the_caret() {
    let controller = TextEditingController::with_text("hello world");
    controller.set_selection(TextSelection::collapsed(11));
    let field = EditableText::new(controller.clone())
        .text_direction(TextDirection::Ltr)
        .size(Size::new(300., 32.));
    let mut runtime = Runtime::new(Widget::from(field)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Windows);
    runtime.run_frame(bounds()).unwrap();
    secondary_click(&mut runtime, Offset::new(12., 10.));
    assert!(controller.selection().base < 11);
}

fn mount_area(
    text: &str,
    platform: TextShortcutPlatform,
) -> (Runtime, Shown, incular_widgets::internal::ElementId) {
    let shown: Shown = Rc::default();
    let builder: incular_widgets::EditableTextContextMenuBuilder = Rc::new({
        let shown = shown.clone();
        move |_, menu| {
            shown.borrow_mut().push(menu.clone());
            IgnorePointer::new(SizedBox::shrink()).into()
        }
    });
    let mut runtime = Runtime::new(Widget::selection_area_with_context_menu(
        incular_widgets::internal::SelectionAreaController::new(),
        Widget::selectable_text_styled(
            text,
            incular_text::TextStyle::default(),
            incular_text::TextAlign::Start,
        ),
        builder,
    ))
    .unwrap();
    runtime.set_text_shortcut_platform(platform);
    runtime.run_frame(bounds()).unwrap();
    let label = runtime
        .tree()
        .selectable_text_at(Offset::new(4., 8.))
        .expect("selectable label");
    (runtime, shown, label)
}

#[test]
fn selection_area_right_click_on_windows_collapses_and_offers_select_all() {
    let (mut runtime, shown, label) = mount_area("hello world", TextShortcutPlatform::Windows);
    secondary_click(&mut runtime, Offset::new(12., 8.));
    assert_eq!(runtime.focused_element(), Some(label));
    assert_eq!(types(&last(&shown)), vec![ContextMenuButtonType::SelectAll]);
    let select_all = last(&shown).button_items[0].clone();
    select_all.press();
    let runtime = process(runtime);
    assert_eq!(
        runtime
            .tree()
            .selectable_text_selected_text(label)
            .as_deref(),
        Some("hello world")
    );
}

#[test]
fn selection_area_right_click_on_macos_selects_the_word_to_copy() {
    let (mut runtime, shown, label) = mount_area("hello world", TextShortcutPlatform::MacOs);
    secondary_click(&mut runtime, Offset::new(12., 8.));
    assert_eq!(
        runtime
            .tree()
            .selectable_text_selected_text(label)
            .as_deref(),
        Some("hello")
    );
    assert_eq!(
        types(&last(&shown)),
        vec![
            ContextMenuButtonType::Copy,
            ContextMenuButtonType::SelectAll
        ]
    );
}

#[test]
fn right_click_focus_keeps_the_hit_word_or_caret_and_preserves_both_selection_orders() {
    for (platform, word_on_first_click) in [
        (TextShortcutPlatform::Ios, true),
        (TextShortcutPlatform::MacOs, true),
        (TextShortcutPlatform::Android, false),
        (TextShortcutPlatform::Linux, false),
        (TextShortcutPlatform::Windows, false),
    ] {
        let (mut runtime, controller, _) = mount("one two three four five", platform);
        let field = runtime.tree().text_field_at(Offset::new(20., 10.)).unwrap();
        controller.set_selection(TextSelection::collapsed(10));
        runtime.run_frame(bounds()).unwrap();
        let glyph = runtime.tree().text_field_input_geometry(field).unwrap().caret_rect;
        let pos = Offset::new(glyph.origin.x, glyph.origin.y + glyph.size.height / 2.0);
        controller.set_selection(TextSelection::collapsed(0));
        runtime.run_frame(bounds()).unwrap();

        secondary_click(&mut runtime, pos);
        assert_eq!(runtime.focused_element(), Some(field), "{platform:?}");
        let selection = controller.selection();
        if word_on_first_click {
            assert_eq!(selection.range().as_range(), 8..13, "{platform:?}");
        } else {
            assert_eq!(selection, TextSelection::collapsed(10), "{platform:?}");
        }

        for selection in [TextSelection::new(3, 20), TextSelection::new(20, 3)] {
            controller.set_selection(selection);
            secondary_click(&mut runtime, pos);
            assert_eq!(controller.selection(), selection, "{platform:?}");
        }
    }
}

fn prepare_adaptive_menu(
    runtime: &mut Runtime,
    controller: &TextEditingController,
    selection: TextSelection,
) -> String {
    use incular_platform::TextInputCommand;

    secondary_click(runtime, Offset::new(12., 10.));
    let field = runtime.focused_element().expect("focused field");
    runtime
        .tree()
        .text_field_context_menu(field)
        .unwrap()
        .hide();
    let client = runtime
        .take_text_input_commands()
        .into_iter()
        .find_map(|command| match command {
            TextInputCommand::SetClient { configuration, .. } => Some(configuration.client),
            _ => None,
        })
        .expect("mounted input connection");
    controller.set_selection(selection);
    let request = serde_json::json!({
        "method":"TextInputClient.showToolbar", "args":[client.get(),0,1]
    })
    .to_string();
    show_adaptive_menu(runtime, &request);
    request
}

fn show_adaptive_menu(runtime: &mut Runtime, request: &str) {
    runtime.run_frame(bounds()).unwrap();
    assert_eq!(
        runtime
            .handle_flutter_text_toolbar_call(request.as_bytes())
            .unwrap(),
        Some(true)
    );
    runtime.run_frame(bounds()).unwrap();
}

#[test]
fn adaptive_editable_menu_orders_and_dispatches_mobile_actions_without_hiding() {
    use ContextMenuButtonType::{Copy, Cut, LookUp, SearchWeb, SelectAll, Share};
    use incular_platform::{TextSelectionAction, TextSelectionActionRequest};

    for (platform, expected) in [
        (
            TextShortcutPlatform::Android,
            vec![Cut, Copy, Share, SelectAll],
        ),
        (
            TextShortcutPlatform::Ios,
            vec![Cut, Copy, LookUp, SearchWeb, Share],
        ),
        (TextShortcutPlatform::Linux, vec![Cut, Copy, SelectAll]),
    ] {
        let (mut runtime, editor, shown) = mount("  word  rest", platform);
        let selection = TextSelection::new(8, 0);
        let request = prepare_adaptive_menu(&mut runtime, &editor, selection);
        let field = runtime.focused_element().unwrap();
        let menu = last(&shown);
        assert_eq!(types(&menu), expected);
        let mut actions = Vec::new();
        for (kind, action) in [
            (LookUp, TextSelectionAction::LookUp),
            (SearchWeb, TextSelectionAction::SearchWeb),
            (Share, TextSelectionAction::Share),
        ] {
            if let Some(item) = menu
                .button_items
                .iter()
                .find(|item| item.button_type() == kind)
            {
                assert_eq!(item.label.as_deref(), kind.default_label());
                item.press();
                actions.push(TextSelectionActionRequest {
                    action,
                    text: "  word  ".into(),
                });
            }
        }
        let mut runtime = process(runtime);
        assert_eq!(runtime.take_text_selection_actions(), actions);
        assert!(runtime.take_text_selection_actions().is_empty());
        assert_eq!(runtime.focused_element(), Some(field));
        assert_eq!(editor.selection(), selection);
        assert!(
            runtime
                .tree()
                .text_field_context_menu(field)
                .unwrap()
                .is_showing()
        );

        // Showing a new menu must not revive callbacks to its predecessor.
        menu.hide();
        show_adaptive_menu(&mut runtime, &request);
        for item in menu
            .button_items
            .iter()
            .filter(|item| matches!(item.button_type(), LookUp | SearchWeb | Share))
        {
            item.press();
        }
        let mut runtime = process(runtime);
        assert!(runtime.take_text_selection_actions().is_empty());
        assert!(
            runtime
                .tree()
                .text_field_context_menu(field)
                .unwrap()
                .is_showing()
        );
    }
}

#[test]
fn adaptive_editable_menu_rejects_blank_collapsed_obscured_and_legacy_selections() {
    use ContextMenuButtonType::{LookUp, SearchWeb, Share};
    use incular_widgets::ToolbarOptions;

    // UTF-8 storage uses byte offsets: U+FEFF (3 bytes), U+00A0 (2 bytes).
    for (selection, obscure, options) in [
        (TextSelection::new(0, 5), false, None),
        (TextSelection::collapsed(6), false, None),
        (TextSelection::new(5, 9), true, None),
        (
            TextSelection::new(5, 9),
            false,
            Some(ToolbarOptions::for_field(false, false)),
        ),
    ] {
        let (mut runtime, editor, shown) =
            mount_with("\u{feff}\u{a0}word", TextShortcutPlatform::Ios, |field| {
                let field = field.obscure_text(obscure);
                match options {
                    Some(options) => field.toolbar_options(options),
                    None => field,
                }
            });
        // Keep one ordinary button visible when an obscured iOS
        // selection disables copy, cut, and select-all.
        runtime.set_clipboard_text("clipboard");
        prepare_adaptive_menu(&mut runtime, &editor, selection);
        assert!(
            !types(&last(&shown))
                .iter()
                .any(|kind| matches!(kind, LookUp | SearchWeb | Share))
        );
        assert!(runtime.take_text_selection_actions().is_empty());
    }
}

#[test]
fn adaptive_editable_menu_can_share_from_a_read_only_field() {
    use ContextMenuButtonType::{Copy, LookUp, SearchWeb, Share};

    let (mut runtime, editor, shown) =
        mount_with("word rest", TextShortcutPlatform::Ios, |field| {
            field.read_only(true)
        });
    secondary_click(&mut runtime, Offset::new(12., 10.));
    assert_eq!(editor.selection(), TextSelection::new(0, 4));
    let menu = last(&shown);
    assert_eq!(types(&menu), vec![Copy, LookUp, SearchWeb, Share]);
    menu.button_items.last().unwrap().press();
    let mut runtime = process(runtime);
    let [request] = runtime.take_text_selection_actions().try_into().unwrap();
    assert_eq!(request.action, incular_platform::TextSelectionAction::Share);
    assert_eq!(request.text, "word");
}

#[test]
fn flutter_text_selection_actions_send_the_unmodified_string_on_platform() {
    use incular_platform::{
        FlutterPlatformMessenger, TextSelectionAction, TextSelectionActionRequest,
        message_codecs::{JsonMethodCodec, MethodCall},
    };
    use std::{
        convert::Infallible,
        future::{Future, ready},
        pin::pin,
        task::{Context, Poll, Waker},
    };

    #[derive(Default)]
    struct Messenger(Vec<(&'static str, MethodCall<serde_json::Value>)>);
    impl FlutterPlatformMessenger for Messenger {
        type Error = Infallible;

        fn send_platform_message(
            &mut self,
            channel: &'static str,
            payload: Option<Vec<u8>>,
        ) -> impl Future<Output = Result<Option<Vec<u8>>, Self::Error>> {
            self.0.push((
                channel,
                JsonMethodCodec::decode_method_call(&payload.unwrap()).unwrap(),
            ));
            ready(Ok(None))
        }
    }
    let mut transport = Messenger::default();
    for (action, name) in [
        (TextSelectionAction::LookUp, "LookUp.invoke"),
        (TextSelectionAction::SearchWeb, "SearchWeb.invoke"),
        (TextSelectionAction::Share, "Share.invoke"),
    ] {
        let request = TextSelectionActionRequest {
            action,
            text: " café 🦀 ".to_owned(),
        };
        assert!(matches!(
            pin!(request.send_flutter(&mut transport))
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Ok(()))
        ));
        assert_eq!(
            transport.0.pop(),
            Some((
                "flutter/platform",
                MethodCall {
                    method: name.to_owned(),
                    arguments: serde_json::json!(" café 🦀 "),
                }
            ))
        );
    }
}

#[test]
fn selectable_region_share_matches_android_long_press_and_clears_selection() {
    use ContextMenuButtonType::{Copy, SelectAll, Share};
    use incular_core::PRIMARY_POINTER_BUTTON;
    use incular_platform::{TextSelectionAction, TextSelectionActionRequest};
    use std::time::{Duration, Instant};

    for platform in [
        TextShortcutPlatform::Android,
        TextShortcutPlatform::Ios,
        TextShortcutPlatform::Linux,
        TextShortcutPlatform::MacOs,
        TextShortcutPlatform::Windows,
    ] {
        use incular_widgets::{Directionality, SelectableRegion, Text};
        let shown: Shown = Rc::default();
        let region = SelectableRegion::new(Text::new("How are you?")).context_menu_builder({
            let shown = shown.clone();
            move |_, menu| {
                shown.borrow_mut().push(menu.clone());
                IgnorePointer::new(SizedBox::shrink()).into()
            }
        });
        let mut runtime = Runtime::new(Directionality::new(TextDirection::Ltr, region).into()).unwrap();
        runtime.set_text_shortcut_platform(platform);
        runtime.run_frame(bounds()).unwrap();
        let label = runtime
            .tree()
            .selectable_text_at(Offset::new(4., 8.))
            .unwrap();
        let point = (0..200)
            .map(|x| Offset::new(x as f32, 8.))
            .find(|point| runtime.tree().selectable_text_position_at(label, *point) == Some(6))
            .expect("point inside ‘are’");
        let start = Instant::now();
        let released = start + Duration::from_millis(500);
        for (phase, buttons, time) in [
            (PointerPhase::Down, PRIMARY_POINTER_BUTTON, start),
            (PointerPhase::Up, 0, released),
        ] {
            runtime.run_frame_at(bounds(), time).unwrap();
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: 1,
                    device: 3,
                    kind: PointerDeviceKind::Touch,
                    buttons,
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::default(),
                    phase,
                    position: point,
                },
                time,
            );
        }
        runtime.run_frame_at(bounds(), released).unwrap();
        assert_eq!(
            runtime
                .tree()
                .selectable_text_editing_value(label)
                .unwrap()
                .1,
            TextSelection::new(4, 7),
            "{platform:?}"
        );
        let menu = last(&shown);
        if platform == TextShortcutPlatform::Android {
            assert_eq!(types(&menu), vec![Copy, Share, SelectAll]);
            menu.button_items[1].press();
            let mut runtime = process(runtime);
            assert_eq!(
                runtime.take_text_selection_actions(),
                vec![TextSelectionActionRequest {
                    action: TextSelectionAction::Share,
                    text: "are".into()
                }]
            );
            assert_eq!(
                runtime
                    .tree()
                    .selectable_text_selected_text(label)
                    .as_deref(),
                Some("")
            );
            assert!(
                !runtime
                    .tree()
                    .selectable_text_context_menu(label)
                    .unwrap()
                    .is_showing()
            );
            assert_eq!(runtime.focused_element(), Some(label));
            menu.button_items[1].press();
            assert!(process(runtime).take_text_selection_actions().is_empty());
        } else {
            assert_eq!(types(&menu), vec![Copy, SelectAll], "{platform:?}");
            assert!(runtime.take_text_selection_actions().is_empty());
        }
    }
}

#[test]
fn toolbar_copy_preserves_desktop_and_ios_selection_but_collapses_android_at_the_end() {
    use ContextMenuButtonType::Copy;
    use incular_platform::{TransferFormat, TransferReadRequest};

    for (platform, collapse) in [
        (TextShortcutPlatform::Ios, false),
        (TextShortcutPlatform::Linux, false),
        (TextShortcutPlatform::MacOs, false),
        (TextShortcutPlatform::Windows, false),
        (TextShortcutPlatform::Android, true),
    ] {
        let (mut runtime, controller, shown) = mount("Hello world", platform);
        let reversed = TextSelection::new(5, 0);
        prepare_adaptive_menu(&mut runtime, &controller, reversed);
        let field = runtime.focused_element().unwrap();
        let toolbar = runtime.tree().text_field_toolbar_state(field).unwrap();
        assert!(toolbar.is_showing());
        let menu = last(&shown);
        menu.button_items
            .iter()
            .find(|item| item.button_type() == Copy)
            .expect("selection offers Copy")
            .press();
        let mut runtime = process(runtime);
        assert!(!toolbar.is_showing(), "{platform:?}");
        assert_eq!(runtime.focused_element(), Some(field), "{platform:?}");
        assert_eq!(controller.text(), "Hello world", "{platform:?}");
        assert_eq!(
            controller.selection(),
            if collapse {
                TextSelection::collapsed(5)
            } else {
                reversed
            },
            "{platform:?}",
        );
        let transfer = runtime
            .read_clipboard(TransferReadRequest::format(TransferFormat::PlainText))
            .unwrap();
        assert_eq!(
            transfer.plain_text_value().unwrap().as_deref(),
            Some("Hello"),
            "{platform:?}"
        );
    }
}

#[test]
fn paste_button_replaces_selection_closes_the_live_toolbar_and_keeps_focus() {
    let (mut runtime, controller, shown) = mount("Hello world", TextShortcutPlatform::Windows);
    runtime.set_clipboard_text("Rust");
    prepare_adaptive_menu(&mut runtime, &controller, TextSelection::new(6, 11));
    let field = runtime.focused_element().unwrap();
    let toolbar = runtime.tree().text_field_toolbar_state(field).unwrap();
    let render = runtime.tree().render_id(field);
    assert!(toolbar.is_showing());
    last(&shown)
        .button_items
        .iter()
        .find(|item| item.button_type() == ContextMenuButtonType::Paste)
        .expect("nonempty clipboard enables Paste")
        .press();
    let runtime = process(runtime);
    assert_eq!(controller.text(), "Hello Rust");
    assert_eq!(controller.selection(), TextSelection::collapsed(10));
    assert!(!toolbar.is_showing());
    assert_eq!(runtime.focused_element(), Some(field));
    assert_eq!(runtime.tree().render_id(field), render);
}
