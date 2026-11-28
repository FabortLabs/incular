use std::{cell::RefCell, rc::Rc};

use incular_config::{Constraints, Locale};
use incular_core::{Color, Offset, Rect, Size};
use incular_runtime::{Runtime, TextShortcutPlatform};
use incular_text::{
    TextAffinity, TextDecoration, TextEditingController, TextRange, TextSelection, TextStyle,
};
use incular_widgets::{
    EditableText, FocusNode, IgnorePointer, SizedBox, SpellCheckCallback, SpellCheckConfiguration,
    SpellCheckService, SuggestionSpan, TextContextMenu, TextDirection, Widget,
};

struct Checker;

impl SpellCheckService for Checker {
    fn fetch_spell_check_suggestions(&self, _: &Locale, text: &str, respond: SpellCheckCallback) {
        respond(Some(if text == "tset tst testt" {
            vec![
                SuggestionSpan::new(0..4, ["test"]),
                SuggestionSpan::new(5..8, ["test"]),
                SuggestionSpan::new(9..13, ["test"]),
            ]
        } else if text == "test tst testt" {
            vec![
                SuggestionSpan::new(5..8, ["test"]),
                SuggestionSpan::new(9..13, ["test"]),
            ]
        } else if text == "test test testt" {
            vec![SuggestionSpan::new(10..15, ["test"])]
        } else if text.starts_with("tset ") {
            vec![SuggestionSpan::new(0..4, ["test", "sets", "set"])]
        } else if text.starts_with("none ") {
            vec![SuggestionSpan::new(0..4, [] as [&str; 0])]
        } else {
            vec![]
        }));
    }
}

const NORMAL_SELECTION: Color = Color::rgba(17, 70, 176, 190);
const SUGGESTION_SELECTION: Color = Color::rgba(212, 36, 87, 183);

type BuiltMenus = Rc<RefCell<Vec<TextContextMenu>>>;

fn input(
    controller: &TextEditingController,
    focus: &FocusNode,
    shown: &BuiltMenus,
    obscure: bool,
    read_only: bool,
) -> Widget {
    input_with_suggestion_selection(controller, focus, shown, obscure, read_only, false)
}

fn input_with_suggestion_selection(
    controller: &TextEditingController,
    focus: &FocusNode,
    shown: &BuiltMenus,
    obscure: bool,
    read_only: bool,
    collapse_suggestion_selection: bool,
) -> Widget {
    EditableText::new(controller.clone())
        .platform_tap_selection(true)
        .size(Size::new(260.0, 30.0))
        .text_direction(TextDirection::Ltr)
        .focus_node(focus.clone())
        .selection_color(NORMAL_SELECTION)
        .read_only(read_only)
        .obscure_text(obscure)
        .context_menu_builder(|_, _| SizedBox::shrink().into())
        .selection_handles_builder(|_, _| SizedBox::shrink().into())
        .spell_check_configuration(
            SpellCheckConfiguration::new()
                .spell_check_service(Checker)
                .collapse_suggestion_selection(collapse_suggestion_selection)
                .misspelled_text_style(TextStyle::new().decoration(TextDecoration::UNDERLINE))
                .misspelled_selection_color(SUGGESTION_SELECTION)
                .spell_check_suggestions_toolbar_builder({
                    let shown = shown.clone();
                    move |_, menu| {
                        shown.borrow_mut().push(menu.clone());
                        IgnorePointer::new(SizedBox::shrink()).into()
                    }
                }),
        )
        .into()
}

fn layout(runtime: &mut Runtime) {
    runtime
        .run_frame(Constraints::tight(Size::new(300., 40.)))
        .unwrap();
}

fn last_menu(shown: &BuiltMenus) -> TextContextMenu {
    shown.borrow().last().cloned().expect("builder was invoked")
}

#[test]
fn the_suggestions_overlay_requires_a_focused_current_word_and_applies_user_edits() {
    let controller = TextEditingController::with_text("tset again");
    let focus = FocusNode::new();
    let shown = BuiltMenus::default();
    let mut runtime = Runtime::new(input(&controller, &focus, &shown, false, false)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    layout(&mut runtime);
    let field = runtime.tree().text_field_at(Offset::new(15., 12.)).unwrap();
    assert!(!runtime.show_spell_check_suggestions_toolbar(field));
    focus.request_focus();
    layout(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(field));
    assert!(!runtime.show_spell_check_suggestions_toolbar(field));

    controller.move_end(false);
    controller.insert_text("!");
    controller.set_selection(TextSelection::collapsed(6));
    layout(&mut runtime);
    assert!(!runtime.show_spell_check_suggestions_toolbar(field));
    controller.set_selection(TextSelection::collapsed(2));
    controller.set_composing(Some(TextRange::new(0, 4)));
    layout(&mut runtime);
    assert!(runtime.show_spell_check_suggestions_toolbar(field));
    layout(&mut runtime);
    let menu = last_menu(&shown);
    assert_eq!(menu.spell_check_span().unwrap().range, 0..4);
    assert_eq!(
        menu.button_items
            .iter()
            .map(|item| item.resolved_label())
            .collect::<Vec<_>>(),
        ["test", "sets", "set"]
    );
    assert_eq!(runtime.focused_element(), Some(field));
    let old_button = menu.button_items[1].clone();
    menu.button_items[0].press();
    layout(&mut runtime);
    assert_eq!(controller.text(), "test again!");
    assert_eq!(controller.selection(), TextSelection::collapsed(2));
    assert_eq!(controller.composing(), Some(TextRange::new(0, 4)));
    assert!(
        !runtime
            .tree()
            .text_field_toolbar_state(field)
            .unwrap()
            .is_showing()
    );

    old_button.press();
    assert_eq!(controller.text(), "test again!");
    assert!(!runtime.show_spell_check_suggestions_toolbar(field));
}

#[test]
fn hiding_or_disabling_the_menu_revokes_visible_suggestion_callbacks() {
    let controller = TextEditingController::with_text("tset here");
    let focus = FocusNode::new();
    let shown = BuiltMenus::default();
    let mut runtime = Runtime::new(input(&controller, &focus, &shown, false, false)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    layout(&mut runtime);
    focus.request_focus();
    layout(&mut runtime);
    let field = runtime.focused_element().unwrap();
    controller.move_end(false);
    controller.insert_text("!");
    controller.set_selection(TextSelection::collapsed(2));
    layout(&mut runtime);

    assert!(runtime.show_spell_check_suggestions_toolbar(field));
    layout(&mut runtime);
    let first = last_menu(&shown);
    first.hide();
    assert!(runtime.show_spell_check_suggestions_toolbar(field));
    layout(&mut runtime);
    let second = last_menu(&shown);
    first.button_items[0].press();
    assert_eq!(controller.text(), "tset here!");

    let root = runtime.tree().root().unwrap();
    runtime
        .schedule_update(root, input(&controller, &focus, &shown, false, true))
        .unwrap();
    layout(&mut runtime);
    assert_eq!(runtime.focused_element(), Some(field));
    assert!(!runtime.show_spell_check_suggestions_toolbar(field));
    second.button_items[0].press();
    assert_eq!(controller.text(), "tset here!");
    assert!(
        !runtime
            .tree()
            .text_field_toolbar_state(field)
            .unwrap()
            .is_showing()
    );
}

#[test]
fn a_span_with_no_replacements_still_builds_the_custom_toolbar() {
    let controller = TextEditingController::with_text("none here");
    let focus = FocusNode::new();
    let shown = BuiltMenus::default();
    let mut runtime = Runtime::new(input(&controller, &focus, &shown, false, false)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    layout(&mut runtime);
    focus.request_focus();
    layout(&mut runtime);
    let field = runtime.focused_element().unwrap();
    controller.move_end(false);
    controller.insert_text("!");
    // Flutter still permits a toolbar for these saved results once the text
    // changes programmatically, provided the span still contains the caret.
    controller.set_text("none again!");
    controller.set_selection(TextSelection::collapsed(2));
    layout(&mut runtime);
    assert!(runtime.show_spell_check_suggestions_toolbar(field));
    layout(&mut runtime);
    let menu = last_menu(&shown);
    assert!(menu.spell_check_span().is_some());
    assert!(menu.button_items.is_empty());
}

fn draw_selections(runtime: &mut Runtime) -> (Vec<Rect>, Vec<Rect>) {
    use incular_rendering::PaintCommand;
    let (list, _) = runtime
        .run_frame(Constraints::tight(Size::new(300., 40.)))
        .unwrap();
    let mut defaults = Vec::new();
    let mut suggestions = Vec::new();
    for command in list.commands() {
        if let PaintCommand::Rect { rect, color } = command {
            match *color {
                NORMAL_SELECTION => defaults.push(*rect),
                SUGGESTION_SELECTION => suggestions.push(*rect),
                _ => {}
            }
        }
    }
    (defaults, suggestions)
}

#[test]
fn suggestion_selection_color_repaints_and_restores_the_same_rectangles() {
    let controller = TextEditingController::with_text("tset here");
    let focus = FocusNode::new();
    let shown = BuiltMenus::default();
    let mut runtime = Runtime::new(input(&controller, &focus, &shown, false, false)).unwrap();
    layout(&mut runtime);
    focus.request_focus();
    layout(&mut runtime);
    let field = runtime.focused_element().unwrap();
    controller.move_end(false);
    controller.insert_text("!");
    controller.set_selection(TextSelection::new(0, 2));
    let (normal, special) = draw_selections(&mut runtime);
    assert!(!normal.is_empty());
    assert!(special.is_empty());

    assert!(runtime.show_spell_check_suggestions_toolbar(field));
    let (default_during, special_during) = draw_selections(&mut runtime);
    assert!(default_during.is_empty());
    assert_eq!(special_during, normal);
    last_menu(&shown).hide();
    let (default_after, special_after) = draw_selections(&mut runtime);
    assert_eq!(default_after, normal);
    assert!(special_after.is_empty());
    assert_eq!(controller.selection(), TextSelection::new(0, 2));
}

#[test]
fn android_single_taps_switch_between_suggestions_and_selection_handles() {
    use incular_core::{
        InputEvent, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, PointerSampleMetadata,
    };
    use std::time::{Duration, Instant};

    let controller = TextEditingController::with_text("tset here");
    let focus = FocusNode::new();
    let shown = BuiltMenus::default();
    let mut runtime = Runtime::new(input(&controller, &focus, &shown, false, false)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Android);
    layout(&mut runtime);
    let field = runtime.tree().text_field_at(Offset::new(15., 12.)).unwrap();
    controller.move_end(false);
    controller.insert_text("!");
    let mut position_at = |offset| {
        controller.set_selection(TextSelection::collapsed(offset));
        layout(&mut runtime);
        let rect = runtime
            .tree()
            .text_field_input_geometry(field)
            .unwrap()
            .caret_rect;
        Offset::new(rect.origin.x, rect.origin.y + rect.size.height / 2.0)
    };
    let good = position_at(7);
    let misspelled = position_at(2);
    controller.set_selection(TextSelection::collapsed(0));
    layout(&mut runtime);
    assert_eq!(runtime.focused_element(), None);

    let start = Instant::now();
    for (index, (position, spelling)) in [(good, false), (misspelled, true), (good, false)]
        .into_iter()
        .enumerate()
    {
        let at = start + Duration::from_millis(600 * index as u64);
        for (phase, buttons) in [
            (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
            (PointerPhase::Up, 0),
        ] {
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: 1,
                    device: 3,
                    kind: PointerDeviceKind::Touch,
                    buttons,
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::default(),
                    phase,
                    position,
                },
                at,
            );
        }
        runtime
            .run_frame_at(Constraints::tight(Size::new(300., 40.)), at)
            .unwrap();
        assert_eq!(runtime.focused_element(), Some(field));
        assert_eq!(
            runtime
                .tree()
                .text_field_toolbar_state(field)
                .unwrap()
                .is_spell_check_showing(),
            spelling
        );
        assert_eq!(
            runtime
                .tree()
                .text_field_selection_handles(field)
                .unwrap()
                .is_showing(),
            !spelling
        );
    }
}

#[test]
fn ios_touch_spelling_tap_selects_the_word_and_toggles_but_mouse_only_moves_the_caret() {
    use incular_core::{
        InputEvent, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, PointerSampleMetadata,
    };
    use std::time::{Duration, Instant};

    let controller = TextEditingController::with_text("tset here");
    let focus = FocusNode::new();
    let shown = BuiltMenus::default();
    let mut runtime = Runtime::new(input(&controller, &focus, &shown, false, false)).unwrap();
    runtime.set_text_shortcut_platform(TextShortcutPlatform::Ios);
    layout(&mut runtime);
    focus.request_focus();
    layout(&mut runtime);
    let field = runtime.focused_element().unwrap();
    controller.move_end(false);
    controller.insert_text("!");
    controller.set_selection(TextSelection::collapsed(2));
    layout(&mut runtime);
    let rect = runtime
        .tree()
        .text_field_input_geometry(field)
        .unwrap()
        .caret_rect;
    let wrong = Offset::new(rect.origin.x, rect.origin.y + rect.size.height / 2.0);
    controller.set_selection(TextSelection::collapsed(7));
    layout(&mut runtime);

    let start = Instant::now();
    for (index, (kind, showing)) in [
        (PointerDeviceKind::Touch, true),
        (PointerDeviceKind::Touch, false),
        (PointerDeviceKind::Mouse, false),
    ]
    .into_iter()
    .enumerate()
    {
        let at = start + Duration::from_millis(600 * index as u64);
        for (phase, buttons) in [
            (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
            (PointerPhase::Up, 0),
        ] {
            let _ = runtime.handle_input_at(
                InputEvent::PointerWithMetadata {
                    pointer: 1,
                    device: 3,
                    kind,
                    buttons,
                    button: Some(PRIMARY_POINTER_BUTTON),
                    sample: PointerSampleMetadata::default(),
                    phase,
                    position: wrong,
                },
                at,
            );
            if index == 0 && phase == PointerPhase::Down {
                assert_eq!(
                    controller.selection(),
                    TextSelection::collapsed(7),
                    "touch-down has not confirmed a spelling tap"
                );
            }
        }
        runtime
            .run_frame_at(Constraints::tight(Size::new(300., 40.)), at)
            .unwrap();
        let state = runtime.tree().text_field_toolbar_state(field).unwrap();
        assert_eq!(state.is_spell_check_showing(), showing);
        if !showing {
            assert!(!state.is_showing());
        }
        if kind == PointerDeviceKind::Touch {
            assert_eq!(controller.selection().range().as_range(), 0..4);
        } else {
            assert_eq!(controller.selection(), TextSelection::collapsed(2));
        }
    }
}

#[test]
fn suggestion_replacements_can_place_the_caret_at_the_end_of_the_new_word_on_mobile() {
    use incular_core::{
        InputEvent, PRIMARY_POINTER_BUTTON, PointerDeviceKind, PointerPhase, PointerSampleMetadata,
    };
    use std::time::{Duration, Instant};

    for platform in [TextShortcutPlatform::Ios, TextShortcutPlatform::Android] {
        let controller = TextEditingController::with_text("tset tst test");
        let focus = FocusNode::new();
        let shown = BuiltMenus::default();
        let mut runtime = Runtime::new(input_with_suggestion_selection(
            &controller,
            &focus,
            &shown,
            false,
            false,
            true,
        ))
        .unwrap();
        runtime.set_text_shortcut_platform(platform);
        focus.request_focus();
        layout(&mut runtime);
        let field = runtime.focused_element().unwrap();
        // Spell-check results arrive through the mounted field's ordinary
        // user-edit subscription, including after each accepted suggestion.
        controller.set_selection(TextSelection::collapsed(13));
        controller.insert_text("t");
        controller.set_selection(TextSelection::new(0, 4));
        layout(&mut runtime);
        assert_eq!(controller.text(), "tset tst testt");

        let start = Instant::now();
        for (index, (word_start, word_end, expected_text, expected_caret)) in [
            (0, 4, "test tst testt", 4),
            (5, 8, "test test testt", 9),
            (10, 15, "test test test", 14),
        ]
        .into_iter()
        .enumerate()
        {
            let marker = runtime
                .tree()
                .text_field_offset_rect(field, word_start, TextAffinity::Downstream)
                .unwrap();
            let position =
                marker.origin + Offset::new(marker.size.width * 0.5, marker.size.height * 0.5);
            let at = start + Duration::from_millis(600 * index as u64);
            // A precise pointer leaves presentation to the custom toolbar;
            // mobile touch auto-presentation is covered independently above.
            for (phase, buttons) in [
                (PointerPhase::Down, PRIMARY_POINTER_BUTTON),
                (PointerPhase::Up, 0),
            ] {
                let _ = runtime.handle_input_at(
                    InputEvent::PointerWithMetadata {
                        pointer: 1,
                        device: 3,
                        kind: PointerDeviceKind::Mouse,
                        buttons,
                        button: Some(PRIMARY_POINTER_BUTTON),
                        sample: PointerSampleMetadata::default(),
                        phase,
                        position,
                    },
                    at,
                );
            }
            layout(&mut runtime);
            assert_eq!(controller.selection(), TextSelection::collapsed(word_start));
            assert!(runtime.show_spell_check_suggestions_toolbar(field));
            layout(&mut runtime);
            let menu = last_menu(&shown);
            assert_eq!(menu.spell_check_span().unwrap().range, word_start..word_end);
            assert_eq!(menu.button_items.len(), 1);
            assert_eq!(menu.button_items[0].resolved_label(), "test");

            menu.button_items[0].press();
            layout(&mut runtime);
            assert_eq!(controller.text(), expected_text);
            assert_eq!(
                controller.selection(),
                TextSelection::collapsed(expected_caret)
            );
            assert_eq!(controller.caret_affinity(), TextAffinity::Downstream);
            assert!(focus.has_primary_focus());
            assert!(
                !runtime
                    .tree()
                    .text_field_toolbar_state(field)
                    .unwrap()
                    .is_showing()
            );
        }
    }
}
