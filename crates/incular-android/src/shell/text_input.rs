//! The soft keyboard, hardware keys, the back button and the clipboard.
//!
//! Text input follows Flutter's Android embedding (`TextInputPlugin`,
//! `InputConnectionAdaptor`): the engine owns the field's value, and Kotlin
//! keeps a mirror `Editable` the input method edits. After each input method
//! edit Kotlin reports the mirror's whole state ([`native_editing_state`]),
//! and the runtime applies the difference as a user edit. The runtime's
//! [`TextInputCommand`]s attach the mirror to a field, update it after the
//! app changes the field, and show or hide the keyboard. Offsets cross JNI in
//! UTF-16 code units, as Java strings count them.

use super::java::{self, Arg};
use incular_core::{Code, InputEvent, KeyState, KeyboardEvent, KeyboardKey, Modifiers, NamedKey};
use incular_mobile::text_offsets::{utf8_offset, utf16_offset};
use incular_platform::{
    CapabilitySupport, Clipboard, ClipboardCapabilities, ClipboardError, ClipboardWriteReport,
    DataTransfer, PlatformEvent, TextCapitalization, TextInputAction, TextInputCommand,
    TextInputConfiguration, TextInputState, TextInputType, TransferData, TransferFormat,
    TransferFormatCapabilities, TransferReadRequest,
};
use jni_sys::{JNIEnv, jboolean, jclass, jint, jstring};
use std::cell::Cell;

thread_local! {
    /// Whether the attached field accepts typing, so `Show` may open the
    /// keyboard for it.
    static EDITABLE_CLIENT: Cell<bool> = const { Cell::new(false) };
}

/// Applies the runtime's queued text input commands to the Kotlin side.
pub(super) fn apply(commands: Vec<TextInputCommand>) {
    for command in commands {
        match command {
            TextInputCommand::SetClient {
                configuration,
                state,
                ..
            } => {
                let editable = configuration.enabled && !configuration.read_only;
                EDITABLE_CLIENT.set(editable);
                if !editable {
                    // Flutter opens no input connection for a read-only field.
                    hide(true);
                    continue;
                }
                let (text, selection_start, selection_end, composing_start, composing_end) =
                    utf16_state(&state);
                java::call_void(
                    c"showTextInput",
                    c"(IILjava/lang/String;IIII)V",
                    &[
                        Arg::Int(input_type(&configuration)),
                        Arg::Int(ime_options(&configuration)),
                        Arg::Str(text),
                        Arg::Int(selection_start),
                        Arg::Int(selection_end),
                        Arg::Int(composing_start),
                        Arg::Int(composing_end),
                    ],
                );
            }
            TextInputCommand::Update { state, .. } => {
                let (text, selection_start, selection_end, composing_start, composing_end) =
                    utf16_state(&state);
                java::call_void(
                    c"updateTextInput",
                    c"(Ljava/lang/String;IIII)V",
                    &[
                        Arg::Str(text),
                        Arg::Int(selection_start),
                        Arg::Int(selection_end),
                        Arg::Int(composing_start),
                        Arg::Int(composing_end),
                    ],
                );
            }
            TextInputCommand::Show { .. } => {
                if EDITABLE_CLIENT.get() {
                    java::call_void(c"showSoftInput", c"()V", &[]);
                }
            }
            TextInputCommand::SetCaretRect { .. } => {}
            TextInputCommand::FinishAutofillContext { action } => {
                java::call_void(c"finishAutofillContext", c"(Z)V", &[Arg::Bool(action.should_save())]);
            }
            TextInputCommand::Hide { .. } => hide(false),
            TextInputCommand::Clear { .. } => {
                EDITABLE_CLIENT.set(false);
                hide(true);
            }
        }
    }
}

/// Hides the keyboard; `detach` also drops the field from the input method.
fn hide(detach: bool) {
    java::call_void(c"hideTextInput", c"(Z)V", &[Arg::Bool(detach)]);
}

/// The state's text with UTF-16 selection and composing offsets; -1 for no
/// composing range.
fn utf16_state(state: &TextInputState) -> (&str, jint, jint, jint, jint) {
    let text = state.text.as_str();
    let offset = |utf8: usize| jint::try_from(utf16_offset(text, utf8)).unwrap_or(jint::MAX);
    let (composing_start, composing_end) = state
        .composing
        .map_or((-1, -1), |(start, end)| (offset(start), offset(end)));
    (
        text,
        offset(state.selection_start),
        offset(state.selection_end),
        composing_start,
        composing_end,
    )
}

/// Flutter `TextInputPlugin.inputTypeFromTextInputType`, as
/// `android.text.InputType` bits.
fn input_type(configuration: &TextInputConfiguration) -> jint {
    const CLASS_TEXT: jint = 0x1;
    const CLASS_NUMBER: jint = 0x2;
    const CLASS_PHONE: jint = 0x3;
    const CLASS_DATETIME: jint = 0x4;
    const FLAG_NUMBER_SIGNED: jint = 0x1000;
    const FLAG_NUMBER_DECIMAL: jint = 0x2000;
    const VARIATION_URI: jint = 0x10;
    const VARIATION_EMAIL_ADDRESS: jint = 0x20;
    const VARIATION_PERSON_NAME: jint = 0x60;
    const VARIATION_POSTAL_ADDRESS: jint = 0x70;
    const VARIATION_PASSWORD: jint = 0x80;
    const VARIATION_VISIBLE_PASSWORD: jint = 0x90;
    const FLAG_CAP_CHARACTERS: jint = 0x1000;
    const FLAG_CAP_WORDS: jint = 0x2000;
    const FLAG_CAP_SENTENCES: jint = 0x4000;
    const FLAG_AUTO_CORRECT: jint = 0x8000;
    const FLAG_MULTI_LINE: jint = 0x2_0000;
    const FLAG_NO_SUGGESTIONS: jint = 0x8_0000;
    let mut bits = match configuration.input_type {
        TextInputType::Number { signed, decimal } => {
            let mut bits = CLASS_NUMBER;
            if signed {
                bits |= FLAG_NUMBER_SIGNED;
            }
            if decimal {
                bits |= FLAG_NUMBER_DECIMAL;
            }
            return bits;
        }
        TextInputType::Phone => return CLASS_PHONE,
        TextInputType::DateTime => return CLASS_DATETIME,
        TextInputType::Name => CLASS_TEXT | VARIATION_PERSON_NAME,
        TextInputType::StreetAddress => CLASS_TEXT | VARIATION_POSTAL_ADDRESS,
        TextInputType::Email => CLASS_TEXT | VARIATION_EMAIL_ADDRESS,
        TextInputType::Url => CLASS_TEXT | VARIATION_URI,
        TextInputType::Text | TextInputType::Multiline | TextInputType::Password => CLASS_TEXT,
    };
    if configuration.multiline {
        bits |= FLAG_MULTI_LINE;
    }
    let traits = &configuration.traits;
    if configuration.obscure_text || configuration.input_type == TextInputType::Password {
        bits |= FLAG_NO_SUGGESTIONS | VARIATION_PASSWORD;
    } else {
        if traits.autocorrect {
            bits |= FLAG_AUTO_CORRECT;
        }
        if !traits.enable_suggestions {
            // Flutter's workaround: keyboards ignore NO_SUGGESTIONS alone.
            bits |= FLAG_NO_SUGGESTIONS | VARIATION_VISIBLE_PASSWORD;
        }
    }
    bits | match traits.text_capitalization {
        TextCapitalization::Characters => FLAG_CAP_CHARACTERS,
        TextCapitalization::Words => FLAG_CAP_WORDS,
        TextCapitalization::Sentences => FLAG_CAP_SENTENCES,
        TextCapitalization::None => 0,
    }
}

/// Flutter `TextInputPlugin.createInputConnection`'s `imeOptions`.
fn ime_options(configuration: &TextInputConfiguration) -> jint {
    const FLAG_NO_PERSONALIZED_LEARNING: jint = 0x100_0000;
    const FLAG_NO_FULLSCREEN: jint = 0x200_0000;
    let action = match configuration.action {
        TextInputAction::Unspecified => 0,
        TextInputAction::None | TextInputAction::Newline => 1,
        TextInputAction::Go => 2,
        TextInputAction::Search => 3,
        TextInputAction::Send => 4,
        TextInputAction::Next => 5,
        TextInputAction::Done => 6,
        TextInputAction::Previous => 7,
    };
    let mut options = FLAG_NO_FULLSCREEN | action;
    if !configuration.traits.enable_ime_personalized_learning {
        options |= FLAG_NO_PERSONALIZED_LEARNING;
    }
    options
}

/// The mirror's state after an input method edit. Offsets are UTF-16; a
/// composing start of -1 means none.
pub(super) extern "system" fn native_editing_state(
    env: *mut JNIEnv,
    _class: jclass,
    text: jstring,
    selection_start: jint,
    selection_end: jint,
    composing_start: jint,
    composing_end: jint,
) {
    // SAFETY: JNI passes a valid env and a local `String` reference.
    let Some(text) = (unsafe { java::string_from_java(env, text) }) else {
        return;
    };
    let offset = |utf16: jint| utf8_offset(&text, usize::try_from(utf16).unwrap_or(0));
    let composing = (composing_start >= 0 && composing_end > composing_start)
        .then(|| (offset(composing_start), offset(composing_end)));
    let (selection_start, selection_end) = (offset(selection_start), offset(selection_end));
    let state = TextInputState {
        text,
        selection_start,
        selection_end,
        composing,
    };
    super::with_shell(|shell| {
        shell
            .engine
            .platform_event(PlatformEvent::TextEditingState(state));
    });
}

/// The keyboard's action key (`EditorInfo.IME_ACTION_*`), as Flutter's
/// `InputConnectionAdaptor.performEditorAction` maps it.
pub(super) extern "system" fn native_editor_action(
    _env: *mut JNIEnv,
    _class: jclass,
    action: jint,
) {
    let action = match action {
        1 => TextInputAction::Newline,
        2 => TextInputAction::Go,
        3 => TextInputAction::Search,
        4 => TextInputAction::Send,
        5 => TextInputAction::Next,
        6 => TextInputAction::Done,
        7 => TextInputAction::Previous,
        _ => TextInputAction::Unspecified,
    };
    super::with_shell(|shell| {
        shell
            .engine
            .platform_event(PlatformEvent::TextInputAction(action));
    });
}

/// A hardware key (`KeyEvent`). Returns whether Incular took it; keys it
/// does not know, such as volume, stay with the system. A printable key
/// press also types its character, as Winit's text events do on desktop.
pub(super) extern "system" fn native_key(
    _env: *mut JNIEnv,
    _class: jclass,
    down: jboolean,
    key_code: jint,
    unicode: jint,
    meta_state: jint,
    repeat: jboolean,
) -> jboolean {
    let character = u32::try_from(unicode)
        .ok()
        .and_then(char::from_u32)
        .filter(|character| !character.is_control());
    // A key without a physical mapping still types its character, as Flutter's
    // `InputConnectionAdaptor` does; a key with neither stays with the system.
    let (code, named) = match key_code_to_code(key_code) {
        Some(mapped) => mapped,
        None if character.is_some() => (Code::Unidentified, None),
        None => return 0,
    };
    let modifiers = modifiers(meta_state);
    let key = match (named, character) {
        (Some(named), _) => KeyboardKey::Named(named),
        (None, Some(character)) => KeyboardKey::Character(character.to_string()),
        (None, None) => KeyboardKey::Named(NamedKey::Unidentified),
    };
    let state = if down != 0 {
        KeyState::Down
    } else {
        KeyState::Up
    };
    let event = KeyboardEvent {
        state,
        key,
        code,
        modifiers,
        repeat: repeat != 0,
        ..KeyboardEvent::default()
    };
    let typed = character.filter(|_| {
        state == KeyState::Down
            && !modifiers.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::META)
    });
    super::with_shell(|shell| {
        shell
            .engine
            .platform_event(PlatformEvent::Input(InputEvent::Key(event)));
        if let Some(character) = typed {
            shell
                .engine
                .platform_event(PlatformEvent::Input(InputEvent::Text(
                    character.to_string(),
                )));
        }
    });
    1
}

/// The back button or a committed back gesture. Returns whether the app
/// used it; otherwise the activity finishes, as Flutter's
/// `SystemNavigator.pop` does.
pub(super) extern "system" fn native_back(_env: *mut JNIEnv, _class: jclass) -> jboolean {
    let mut handled = false;
    super::with_shell(|shell| handled = shell.engine.back_button());
    jboolean::from(handled)
}

/// `KeyEvent.META_*` bits.
fn modifiers(meta_state: jint) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    for (bit, modifier) in [
        (0x1, Modifiers::SHIFT),
        (0x2, Modifiers::ALT),
        (0x1000, Modifiers::CONTROL),
        (0x1_0000, Modifiers::META),
        (0x10_0000, Modifiers::CAPS_LOCK),
    ] {
        if meta_state & bit != 0 {
            modifiers |= modifier;
        }
    }
    modifiers
}

/// The physical key of an Android `KeyEvent.KEYCODE_*`, with its named key
/// when it types no character.
fn key_code_to_code(key_code: jint) -> Option<(Code, Option<NamedKey>)> {
    const LETTERS: [Code; 26] = [
        Code::KeyA,
        Code::KeyB,
        Code::KeyC,
        Code::KeyD,
        Code::KeyE,
        Code::KeyF,
        Code::KeyG,
        Code::KeyH,
        Code::KeyI,
        Code::KeyJ,
        Code::KeyK,
        Code::KeyL,
        Code::KeyM,
        Code::KeyN,
        Code::KeyO,
        Code::KeyP,
        Code::KeyQ,
        Code::KeyR,
        Code::KeyS,
        Code::KeyT,
        Code::KeyU,
        Code::KeyV,
        Code::KeyW,
        Code::KeyX,
        Code::KeyY,
        Code::KeyZ,
    ];
    const DIGITS: [Code; 10] = [
        Code::Digit0,
        Code::Digit1,
        Code::Digit2,
        Code::Digit3,
        Code::Digit4,
        Code::Digit5,
        Code::Digit6,
        Code::Digit7,
        Code::Digit8,
        Code::Digit9,
    ];
    const NUMPAD_DIGITS: [Code; 10] = [
        Code::Numpad0,
        Code::Numpad1,
        Code::Numpad2,
        Code::Numpad3,
        Code::Numpad4,
        Code::Numpad5,
        Code::Numpad6,
        Code::Numpad7,
        Code::Numpad8,
        Code::Numpad9,
    ];
    const FUNCTION: [(Code, NamedKey); 12] = [
        (Code::F1, NamedKey::F1),
        (Code::F2, NamedKey::F2),
        (Code::F3, NamedKey::F3),
        (Code::F4, NamedKey::F4),
        (Code::F5, NamedKey::F5),
        (Code::F6, NamedKey::F6),
        (Code::F7, NamedKey::F7),
        (Code::F8, NamedKey::F8),
        (Code::F9, NamedKey::F9),
        (Code::F10, NamedKey::F10),
        (Code::F11, NamedKey::F11),
        (Code::F12, NamedKey::F12),
    ];
    let index = |first: jint| usize::try_from(key_code - first).ok();
    let named = |code, named| Some((code, Some(named)));
    match key_code {
        7..=16 => Some((DIGITS[index(7)?], None)),
        29..=54 => Some((LETTERS[index(29)?], None)),
        131..=142 => FUNCTION
            .get(index(131)?)
            .map(|&(code, key)| (code, Some(key))),
        19 => named(Code::ArrowUp, NamedKey::ArrowUp),
        20 => named(Code::ArrowDown, NamedKey::ArrowDown),
        21 => named(Code::ArrowLeft, NamedKey::ArrowLeft),
        22 => named(Code::ArrowRight, NamedKey::ArrowRight),
        144..=153 => Some((NUMPAD_DIGITS[index(144)?], None)),
        154 => Some((Code::NumpadDivide, None)),
        155 => Some((Code::NumpadMultiply, None)),
        156 => Some((Code::NumpadSubtract, None)),
        157 => Some((Code::NumpadAdd, None)),
        158 => Some((Code::NumpadDecimal, None)),
        159 => Some((Code::NumpadComma, None)),
        161 => Some((Code::NumpadEqual, None)),
        162 => Some((Code::NumpadParenLeft, None)),
        163 => Some((Code::NumpadParenRight, None)),
        55 => Some((Code::Comma, None)),
        56 => Some((Code::Period, None)),
        57 => named(Code::AltLeft, NamedKey::Alt),
        58 => named(Code::AltRight, NamedKey::Alt),
        59 => named(Code::ShiftLeft, NamedKey::Shift),
        60 => named(Code::ShiftRight, NamedKey::Shift),
        61 => named(Code::Tab, NamedKey::Tab),
        62 => Some((Code::Space, None)),
        66 => named(Code::Enter, NamedKey::Enter),
        67 => named(Code::Backspace, NamedKey::Backspace),
        68 => Some((Code::Backquote, None)),
        69 => Some((Code::Minus, None)),
        70 => Some((Code::Equal, None)),
        71 => Some((Code::BracketLeft, None)),
        72 => Some((Code::BracketRight, None)),
        73 => Some((Code::Backslash, None)),
        74 => Some((Code::Semicolon, None)),
        75 => Some((Code::Quote, None)),
        76 => Some((Code::Slash, None)),
        85 => named(Code::MediaPlayPause, NamedKey::MediaPlayPause),
        92 => named(Code::PageUp, NamedKey::PageUp),
        93 => named(Code::PageDown, NamedKey::PageDown),
        111 => named(Code::Escape, NamedKey::Escape),
        112 => named(Code::Delete, NamedKey::Delete),
        113 => named(Code::ControlLeft, NamedKey::Control),
        114 => named(Code::ControlRight, NamedKey::Control),
        117 => named(Code::MetaLeft, NamedKey::Meta),
        118 => named(Code::MetaRight, NamedKey::Meta),
        122 => named(Code::Home, NamedKey::Home),
        123 => named(Code::End, NamedKey::End),
        124 => named(Code::Insert, NamedKey::Insert),
        160 => named(Code::NumpadEnter, NamedKey::Enter),
        _ => None,
    }
}

/// The system clipboard (`ClipboardManager`), plain text only, as Flutter's
/// `Clipboard` on Android.
pub(super) struct AndroidClipboard;

impl Clipboard for AndroidClipboard {
    fn capabilities(&self) -> ClipboardCapabilities {
        let formats = TransferFormatCapabilities {
            plain_text: CapabilitySupport::Supported,
            ..TransferFormatCapabilities::all(CapabilitySupport::Unsupported)
        };
        ClipboardCapabilities {
            read: formats,
            write: formats,
            lazy_write: CapabilitySupport::Unsupported,
        }
    }

    fn read(&mut self, request: TransferReadRequest) -> Result<DataTransfer, ClipboardError> {
        let text = java::call_string(c"clipboardText", c"()Ljava/lang/String;", &[])
            .ok_or(ClipboardError::Unavailable)?;
        let filtered = DataTransfer::plain_text(text).filtered(&request);
        if filtered.is_empty() {
            Err(ClipboardError::Unavailable)
        } else {
            Ok(filtered)
        }
    }

    fn write(&mut self, transfer: DataTransfer) -> Result<ClipboardWriteReport, ClipboardError> {
        let Some(TransferData::Text(text)) = transfer.materialize(&TransferFormat::PlainText)?
        else {
            return Err(transfer.formats().into_iter().next().map_or(
                ClipboardError::Unavailable,
                ClipboardError::UnsupportedFormat,
            ));
        };
        java::call_void(
            c"setClipboardText",
            c"(Ljava/lang/String;)V",
            &[Arg::Str(&text)],
        );
        let mut report = ClipboardWriteReport::all_written(vec![TransferFormat::PlainText]);
        report.skipped = transfer
            .formats()
            .into_iter()
            .filter(|format| *format != TransferFormat::PlainText)
            .collect();
        Ok(report)
    }

    /// `ClipboardManager.hasPrimaryClip`, which Android does not report to
    /// the user as a read.
    fn has_text(&mut self) -> bool {
        java::call_bool(c"clipboardHasText", c"()Z", &[])
    }
}
