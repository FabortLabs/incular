//! Text input through a hidden page element.
//!
//! Winit's browser backend has no IME: composition never starts on a canvas,
//! and touch devices only show an on-screen keyboard for an editable element.
//! While a text field is focused, a transparent `<textarea>` therefore sits
//! at its caret and holds the page focus:
//!
//! - Ordinary key presses are forwarded to the window's canvas unchanged, so
//!   Winit and the application handle them exactly as without the element.
//! - Composition (`compositionupdate`, `compositionend`) becomes IME preedit
//!   and commit events.
//! - On-screen keyboards, which often report every key as "Unidentified"
//!   (key code 229), deliver text through `input` events, which become
//!   commits; deletions and line breaks become Backspace, Delete and Enter.
//!
//! The element keeps one placeholder character so a Backspace on an
//! otherwise empty field still produces an `input` event to translate.

use std::cell::RefCell;

use incular_core::{
    Code, ImeEvent, InputEvent, KeyState, KeyboardEvent, KeyboardKey, Location, Modifiers, NamedKey,
};
use incular_platform::{TextCapitalization, TextInputAction, TextInputCommand, TextInputType};
use wasm_bindgen::{JsCast as _, closure::Closure};
use winit::{platform::web::WindowExtWebSys as _, window::WindowId};

const PLACEHOLDER: &str = " ";

const ELEMENT_STYLE: &str = "position:fixed;margin:0;padding:0;border:0;outline:none;\
     opacity:0;color:transparent;caret-color:transparent;background:transparent;\
     resize:none;overflow:hidden;white-space:pre;pointer-events:none;z-index:-1;\
     font-size:16px;line-height:1;";

thread_local! {
    static EDITOR: RefCell<Option<Editor>> = const { RefCell::new(None) };
    static EVENTS: RefCell<Vec<(WindowId, InputEvent)>> = const { RefCell::new(Vec::new()) };
}

struct Editor {
    element: web_sys::HtmlTextAreaElement,
    /// The canvas and window of the focused text field, while one is.
    client: Option<(web_sys::HtmlCanvasElement, WindowId)>,
    composing: bool,
}

pub(crate) fn take_events() -> Vec<(WindowId, InputEvent)> {
    EVENTS.take()
}

/// Whether `target` is the editing element, which stands in for a canvas.
pub(crate) fn is_editor(target: &web_sys::EventTarget) -> bool {
    EDITOR.with_borrow(|editor| {
        editor
            .as_ref()
            .is_some_and(|editor| editor.element.unchecked_ref::<web_sys::EventTarget>() == target)
    })
}

pub(crate) fn apply(window: &winit::window::Window, command: &TextInputCommand) {
    let Some(canvas) = window.canvas() else {
        return;
    };
    match command {
        TextInputCommand::SetClient {
            configuration,
            caret_rect,
            ..
        } => {
            if !configuration.enabled || configuration.read_only {
                deactivate();
                return;
            }
            let Some(element) = editor_element() else {
                return;
            };
            configure(&element, configuration);
            EDITOR.with_borrow_mut(|editor| {
                if let Some(editor) = editor {
                    editor.client = Some((canvas.clone(), window.id()));
                }
            });
            place(&element, &canvas, caret_rect);
            focus(&element);
        }
        TextInputCommand::SetCaretRect { rect, .. } => {
            if let Some(element) = active_element(window.id()) {
                place(&element, &canvas, rect);
                focus(&element);
            }
        }
        TextInputCommand::Update { .. } => {
            // A click on the canvas moves the page focus there while the
            // field stays focused in the application.
            if let Some(element) = active_element(window.id()) {
                focus(&element);
            }
        }
        TextInputCommand::Hide { .. } | TextInputCommand::Clear { .. } => {
            if active_element(window.id()).is_some() {
                deactivate();
            }
        }
    }
}

fn active_element(window: WindowId) -> Option<web_sys::HtmlTextAreaElement> {
    EDITOR.with_borrow(|editor| {
        editor
            .as_ref()
            .filter(|editor| editor.client.as_ref().is_some_and(|(_, id)| *id == window))
            .map(|editor| editor.element.clone())
    })
}

/// Ends editing and returns the page focus to the field's canvas.
fn deactivate() {
    let finished = EDITOR.with_borrow_mut(|editor| {
        let editor = editor.as_mut()?;
        let client = editor.client.take()?;
        let composing = std::mem::take(&mut editor.composing);
        Some((editor.element.clone(), client, composing))
    });
    let Some((element, (canvas, window), composing)) = finished else {
        return;
    };
    if composing {
        push(window, InputEvent::Ime(ImeEvent::End));
    }
    if is_focused(&element) {
        let _ = canvas.focus();
    }
}

fn editor_element() -> Option<web_sys::HtmlTextAreaElement> {
    if let Some(element) =
        EDITOR.with_borrow(|editor| editor.as_ref().map(|editor| editor.element.clone()))
    {
        return Some(element);
    }
    let document = web_sys::window()?.document()?;
    let element = document
        .create_element("textarea")
        .ok()?
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .ok()?;
    element.set_attribute("style", ELEMENT_STYLE).ok()?;
    element.set_attribute("aria-hidden", "true").ok()?;
    element.set_attribute("tabindex", "-1").ok()?;
    reset_value(&element);
    document.body()?.append_child(&element).ok()?;
    listen(&element);
    EDITOR.set(Some(Editor {
        element: element.clone(),
        client: None,
        composing: false,
    }));
    Some(element)
}

/// Picks the on-screen keyboard and its action key, and keeps browser text
/// assistance out of fields that must not have it.
fn configure(
    element: &web_sys::HtmlTextAreaElement,
    configuration: &incular_platform::TextInputConfiguration,
) {
    let mode = match configuration.input_type {
        TextInputType::Number => "decimal",
        TextInputType::Phone => "tel",
        TextInputType::Email => "email",
        TextInputType::Url => "url",
        TextInputType::Text | TextInputType::Multiline | TextInputType::Password => "text",
    };
    let enter = match configuration.action {
        TextInputAction::Done => "done",
        TextInputAction::Go => "go",
        TextInputAction::Search => "search",
        TextInputAction::Send => "send",
        TextInputAction::Next => "next",
        TextInputAction::Previous => "previous",
        TextInputAction::Newline | TextInputAction::Unspecified | TextInputAction::None => {
            if configuration.multiline {
                "enter"
            } else {
                "done"
            }
        }
    };
    let assisted = !configuration.obscure_text
        && !matches!(
            configuration.input_type,
            TextInputType::Password | TextInputType::Email | TextInputType::Url
        );
    let traits = &configuration.traits;
    // Flutter web: the first autofill hint, `on` for an autofill client
    // without hints, and `off` when autofill is disabled.
    let autocomplete = match &traits.autofill_hints {
        Some(hints) => hints
            .first()
            .map_or("on", |hint| browser_autofill_hint(hint)),
        None => "off",
    };
    let autocapitalize = match traits.text_capitalization {
        TextCapitalization::Words => "words",
        TextCapitalization::Characters => "characters",
        TextCapitalization::Sentences => "sentences",
        TextCapitalization::None => "off",
    };
    for (name, value) in [
        ("inputmode", mode),
        ("enterkeyhint", enter),
        ("autocomplete", autocomplete),
        ("autocorrect", if traits.autocorrect { "on" } else { "off" }),
        ("autocapitalize", autocapitalize),
        ("spellcheck", if assisted { "true" } else { "false" }),
    ] {
        let _ = element.set_attribute(name, value);
    }
}

/// Flutter web's `BrowserAutofillHints`: Flutter `AutofillHints` names to
/// HTML `autocomplete` tokens; unknown names pass through unchanged.
fn browser_autofill_hint(hint: &str) -> &str {
    match hint {
        "birthday" => "bday",
        "birthdayDay" => "bday-day",
        "birthdayMonth" => "bday-month",
        "birthdayYear" => "bday-year",
        "countryCode" => "country",
        "countryName" => "country-name",
        "creditCardExpirationDate" => "cc-exp",
        "creditCardExpirationMonth" => "cc-exp-month",
        "creditCardExpirationYear" => "cc-exp-year",
        "creditCardFamilyName" => "cc-family-name",
        "creditCardGivenName" => "cc-given-name",
        "creditCardMiddleName" => "cc-additional-name",
        "creditCardName" => "cc-name",
        "creditCardNumber" => "cc-number",
        "creditCardSecurityCode" => "cc-csc",
        "creditCardType" => "cc-type",
        "email" => "email",
        "familyName" => "family-name",
        "fullStreetAddress" => "street-address",
        "gender" => "sex",
        "givenName" => "given-name",
        "impp" => "impp",
        "jobTitle" => "organization-title",
        "language" => "language",
        "middleName" => "additional-name",
        "name" => "name",
        "namePrefix" => "honorific-prefix",
        "nameSuffix" => "honorific-suffix",
        "newPassword" => "new-password",
        "nickname" => "nickname",
        "oneTimeCode" => "one-time-code",
        "organizationName" => "organization",
        "password" => "current-password",
        "photo" => "photo",
        "postalCode" => "postal-code",
        "streetAddressLevel1" => "address-level1",
        "streetAddressLevel2" => "address-level2",
        "streetAddressLevel3" => "address-level3",
        "streetAddressLevel4" => "address-level4",
        "streetAddressLine1" => "address-line1",
        "streetAddressLine2" => "address-line2",
        "streetAddressLine3" => "address-line3",
        "telephoneNumber" => "tel",
        "telephoneNumberAreaCode" => "tel-area-code",
        "telephoneNumberCountryCode" => "tel-country-code",
        "telephoneNumberExtension" => "tel-extension",
        "telephoneNumberLocal" => "tel-local",
        "telephoneNumberLocalPrefix" => "tel-local-prefix",
        "telephoneNumberLocalSuffix" => "tel-local-suffix",
        "telephoneNumberNational" => "tel-national",
        "transactionAmount" => "transaction-amount",
        "transactionCurrency" => "transaction-currency",
        "url" => "url",
        "username" => "username",
        other => other,
    }
}

/// Moves the element over the caret so candidate windows open beside it.
/// Caret rectangles are logical pixels in the canvas, which are CSS pixels.
fn place(
    element: &web_sys::HtmlTextAreaElement,
    canvas: &web_sys::HtmlCanvasElement,
    caret: &incular_core::Rect,
) {
    let bounds = canvas.get_bounding_client_rect();
    let style = element.style();
    let left = bounds.left() + f64::from(caret.origin.x);
    let top = bounds.top() + f64::from(caret.origin.y);
    let _ = style.set_property("left", &format!("{left}px"));
    let _ = style.set_property("top", &format!("{top}px"));
    let _ = style.set_property("width", &format!("{}px", caret.size.width.max(1.0)));
    let _ = style.set_property("height", &format!("{}px", caret.size.height.max(1.0)));
}

fn focus(element: &web_sys::HtmlTextAreaElement) {
    if !is_focused(element) {
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = element.focus_with_options(&options);
    }
}

fn is_focused(element: &web_sys::HtmlTextAreaElement) -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
        .is_some_and(|active| active == *element.unchecked_ref::<web_sys::Element>())
}

fn reset_value(element: &web_sys::HtmlTextAreaElement) {
    element.set_value(PLACEHOLDER);
    let end = PLACEHOLDER.len() as u32;
    let _ = element.set_selection_range(end, end);
}

fn client() -> Option<(web_sys::HtmlCanvasElement, WindowId, bool)> {
    EDITOR.with_borrow(|editor| {
        let editor = editor.as_ref()?;
        let (canvas, window) = editor.client.clone()?;
        Some((canvas, window, editor.composing))
    })
}

fn push(window: WindowId, event: InputEvent) {
    EVENTS.with_borrow_mut(|events| events.push((window, event)));
    crate::wake_host();
}

fn set_composing(composing: bool) {
    EDITOR.with_borrow_mut(|editor| {
        if let Some(editor) = editor {
            editor.composing = composing;
        }
    });
}

fn listen(element: &web_sys::HtmlTextAreaElement) {
    fn add<E: wasm_bindgen::convert::FromWasmAbi + 'static>(
        element: &web_sys::HtmlTextAreaElement,
        kind: &str,
        handler: impl FnMut(E) + 'static,
    ) {
        let closure = Closure::<dyn FnMut(E)>::new(handler);
        let _ = element.add_event_listener_with_callback(kind, closure.as_ref().unchecked_ref());
        // The element lives as long as the page.
        closure.forget();
    }

    add(element, "keydown", |event: web_sys::KeyboardEvent| {
        forward_key(&event)
    });
    add(element, "keyup", |event: web_sys::KeyboardEvent| {
        forward_key(&event)
    });
    add(
        element,
        "compositionstart",
        |_: web_sys::CompositionEvent| set_composing(true),
    );
    add(
        element,
        "compositionupdate",
        |event: web_sys::CompositionEvent| {
            if let Some((_, window, _)) = client() {
                let text = event.data().unwrap_or_default();
                let end = text.len();
                push(
                    window,
                    InputEvent::Ime(ImeEvent::Preedit {
                        text,
                        selection: Some((end, end)),
                    }),
                );
            }
        },
    );
    add(
        element,
        "compositionend",
        |event: web_sys::CompositionEvent| {
            set_composing(false);
            if let Some((_, window, _)) = client() {
                push(
                    window,
                    InputEvent::Ime(ImeEvent::Commit(event.data().unwrap_or_default())),
                );
            }
            if let Some(element) = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
            {
                reset_value(&element);
            }
        },
    );
    add(element, "input", |event: web_sys::InputEvent| {
        let Some(element) = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        else {
            return;
        };
        let Some((_, window, composing)) = client() else {
            reset_value(&element);
            return;
        };
        if composing || event.is_composing() {
            return;
        }
        match event.input_type().as_str() {
            "insertText" | "insertReplacementText" | "insertFromPaste" => {
                if let Some(text) = event.data().filter(|text| !text.is_empty()) {
                    push(window, InputEvent::Ime(ImeEvent::Commit(text)));
                }
            }
            "deleteContentBackward" | "deleteWordBackward" => {
                press(window, NamedKey::Backspace, Code::Backspace);
            }
            "deleteContentForward" | "deleteWordForward" => {
                press(window, NamedKey::Delete, Code::Delete);
            }
            "insertLineBreak" | "insertParagraph" => press(window, NamedKey::Enter, Code::Enter),
            _ => {}
        }
        reset_value(&element);
    });
    add(element, "blur", |event: web_sys::FocusEvent| {
        // Losing the page focus to anything but the field's canvas is the
        // window losing focus; Winit only listens on the canvas.
        let Some((canvas, window, composing)) = client() else {
            return;
        };
        let to_canvas = event
            .related_target()
            .is_some_and(|target| target == *canvas.unchecked_ref::<web_sys::EventTarget>());
        if composing {
            set_composing(false);
            push(window, InputEvent::Ime(ImeEvent::End));
        }
        if !to_canvas && let Ok(blur) = web_sys::FocusEvent::new("blur") {
            let _ = canvas.dispatch_event(&blur);
        }
    });
}

/// Hands a key press to Winit on the canvas. Keys that belong to a
/// composition, or that an on-screen keyboard leaves unidentified (229),
/// stay with the element and arrive as composition or `input` events.
fn forward_key(event: &web_sys::KeyboardEvent) {
    if event.is_composing() || event.key_code() == 229 {
        return;
    }
    let Some((canvas, _, composing)) = client() else {
        return;
    };
    if composing {
        return;
    }
    // The element must not insert the text itself; Winit's key event does.
    event.prevent_default();
    dispatch_key_copy(event, &canvas);
}

/// Dispatches a copy of a key event to `canvas`, where Winit receives it.
pub(crate) fn dispatch_key_copy(
    event: &web_sys::KeyboardEvent,
    canvas: &web_sys::HtmlCanvasElement,
) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(&event.key());
    init.set_code(&event.code());
    init.set_location(event.location());
    init.set_repeat(event.repeat());
    init.set_ctrl_key(event.ctrl_key());
    init.set_shift_key(event.shift_key());
    init.set_alt_key(event.alt_key());
    init.set_meta_key(event.meta_key());
    init.set_bubbles(true);
    init.set_cancelable(true);
    if let Ok(forwarded) =
        web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(&event.type_(), &init)
    {
        let _ = canvas.dispatch_event(&forwarded);
    }
}

/// A key press synthesized for an on-screen keyboard edit. It is queued
/// behind the commits before it rather than handed to Winit, which would
/// deliver it first.
fn press(window: WindowId, key: NamedKey, code: Code) {
    for state in [KeyState::Down, KeyState::Up] {
        push(
            window,
            InputEvent::Key(KeyboardEvent {
                state,
                key: KeyboardKey::Named(key),
                code,
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                repeat: false,
                is_composing: false,
            }),
        );
    }
}

/// Keeps Winit from seeing the canvas lose focus to the editing element or a
/// semantic element (and regain it), which the application would treat as the
/// window losing focus.
pub(crate) fn install() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let hide_handoff =
        Closure::<dyn FnMut(web_sys::FocusEvent)>::new(|event: web_sys::FocusEvent| {
            let on_canvas = event
                .target()
                .is_some_and(|target| target.dyn_ref::<web_sys::HtmlCanvasElement>().is_some());
            if on_canvas
                && event.related_target().is_some_and(|other| {
                    is_editor(&other) || crate::semantics::is_semantics_element(&other)
                })
            {
                event.stop_immediate_propagation();
            }
        });
    for kind in ["blur", "focus"] {
        let _ = window.add_event_listener_with_callback_and_bool(
            kind,
            hide_handoff.as_ref().unchecked_ref(),
            true,
        );
    }
    hide_handoff.forget();
}
