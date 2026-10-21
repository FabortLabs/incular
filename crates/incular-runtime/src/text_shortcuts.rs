//! Flutter 3.41 `DefaultTextEditingShortcuts`: the keys that edit and move
//! the caret in a text field, per target platform.
//!
//! Flutter chooses one table per `defaultTargetPlatform`. The Windows, Linux
//! and Android tables share `_commonShortcuts` and `_clipboardShortcuts`; the
//! macOS table (reused for iOS) uses Option for words, Command for lines and
//! documents, and adds Emacs-style Control bindings. Activators match their
//! modifiers exactly, so a chord not listed here edits nothing.

use incular_core::{Code, Modifiers};

/// The platform whose `DefaultTextEditingShortcuts` table applies to text
/// fields (Flutter's `defaultTargetPlatform`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShortcutPlatform {
    Android,
    Ios,
    Linux,
    MacOs,
    Windows,
}

impl TextShortcutPlatform {
    /// The platform this build targets. In a browser it is the operating
    /// system the browser runs on, so a Mac uses Command shortcuts.
    #[must_use]
    pub fn current() -> Self {
        #[cfg(target_arch = "wasm32")]
        if let Some(platform) = Self::browser() {
            return platform;
        }
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "ios") {
            Self::Ios
        } else if cfg!(target_os = "android") {
            Self::Android
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Windows
        }
    }

    /// Reads `navigator.userAgentData.platform` where available, then the
    /// legacy `navigator.platform` and user agent strings.
    #[cfg(target_arch = "wasm32")]
    fn browser() -> Option<Self> {
        use wasm_bindgen::JsValue;

        let get = |target: &JsValue, key: &str| {
            js_sys::Reflect::get(target, &JsValue::from_str(key))
                .ok()
                .filter(|value| !value.is_undefined() && !value.is_null())
        };
        let navigator = get(&js_sys::global(), "navigator")?;
        let described = [
            get(&navigator, "userAgentData").and_then(|data| get(&data, "platform")),
            get(&navigator, "platform"),
            get(&navigator, "userAgent"),
        ]
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_string())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
        // Android user agents also mention Linux. iPadOS reports itself as a
        // Mac, whose table it shares.
        if described.contains("android") {
            Some(Self::Android)
        } else if ["iphone", "ipad", "ipod", "ios"]
            .iter()
            .any(|name| described.contains(name))
        {
            Some(Self::Ios)
        } else if described.contains("mac") {
            Some(Self::MacOs)
        } else if described.contains("win") {
            Some(Self::Windows)
        } else if described.contains("linux") || described.contains("cros") {
            Some(Self::Linux)
        } else {
            None
        }
    }

    const fn is_apple(self) -> bool {
        matches!(self, Self::MacOs | Self::Ios)
    }
}

/// What a text-editing chord asks the focused field to do. Each variant is the
/// Flutter intent of the same name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextIntent {
    DeleteCharacter {
        forward: bool,
    },
    DeleteToNextWordBoundary {
        forward: bool,
    },
    DeleteToLineBreak {
        forward: bool,
    },
    ExtendByCharacter {
        forward: bool,
        collapse: bool,
    },
    ExtendToNextWordBoundary {
        forward: bool,
        collapse: bool,
    },
    ExtendToNextWordBoundaryOrCaretLocation {
        forward: bool,
    },
    ExtendToLineBreak {
        forward: bool,
        collapse: bool,
        continues_at_wrap: bool,
    },
    ExtendToNextParagraphBoundary {
        forward: bool,
        collapse: bool,
    },
    ExtendToNextParagraphBoundaryOrCaretLocation {
        forward: bool,
    },
    ExtendToDocumentBoundary {
        forward: bool,
        collapse: bool,
    },
    ExpandToLineBreak {
        forward: bool,
    },
    ExpandToDocumentBoundary {
        forward: bool,
    },
    ExtendVerticallyToAdjacentLine {
        forward: bool,
        collapse: bool,
    },
    ExtendVerticallyToAdjacentPage {
        forward: bool,
        collapse: bool,
    },
    ScrollToDocumentBoundary {
        forward: bool,
    },
    ScrollPage {
        forward: bool,
    },
    Copy,
    Cut,
    Paste,
    SelectAll,
    Undo,
    Redo,
    TransposeCharacters,
    /// `DoNothingAndStopPropagationTextIntent`: the key belongs to the IME.
    DoNothing,
}

/// Resolves a key chord to its text-editing intent on `platform`.
pub(crate) fn resolve(
    platform: TextShortcutPlatform,
    code: Code,
    modifiers: Modifiers,
) -> Option<TextIntent> {
    let chord = Chord {
        ctrl: modifiers.ctrl(),
        alt: modifiers.alt(),
        shift: modifiers.shift(),
        meta: modifiers.meta(),
        num_lock: modifiers.contains(Modifiers::NUM_LOCK),
    };
    if platform.is_apple() {
        return mac(code, chord);
    }
    let table = match platform {
        TextShortcutPlatform::Linux => linux(code, chord),
        TextShortcutPlatform::Windows => windows(code, chord),
        _ => android(code, chord),
    };
    table
        .or_else(|| clipboard(code, chord))
        .or_else(|| common(code, chord))
}

#[derive(Clone, Copy)]
struct Chord {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
    num_lock: bool,
}

impl Chord {
    /// Whether exactly the given modifiers are down (shift is matched by the
    /// callers that accept both states).
    fn only(self, ctrl: bool, alt: bool, shift: bool, meta: bool) -> bool {
        self.ctrl == ctrl && self.alt == alt && self.shift == shift && self.meta == meta
    }

    fn plain(self) -> bool {
        self.only(false, false, false, false)
    }
}

/// `_commonShortcuts`.
fn common(code: Code, c: Chord) -> Option<TextIntent> {
    use TextIntent::*;
    if c.meta {
        return None;
    }
    let collapse = !c.shift;
    let forward_of = |code| matches!(code, Code::ArrowRight | Code::ArrowDown | Code::PageDown);
    match code {
        // Delete shortcuts, with shift in either state.
        Code::Backspace => match (c.ctrl, c.alt) {
            (false, false) => Some(DeleteCharacter { forward: false }),
            (true, false) => Some(DeleteToNextWordBoundary { forward: false }),
            (false, true) => Some(DeleteToLineBreak { forward: false }),
            (true, true) => None,
        },
        Code::Delete => match (c.ctrl, c.alt) {
            (true, false) => Some(DeleteToNextWordBoundary { forward: true }),
            (false, true) => Some(DeleteToLineBreak { forward: true }),
            (false, false) if !c.shift => Some(DeleteCharacter { forward: true }),
            _ => None,
        },
        Code::ArrowLeft | Code::ArrowRight => {
            let forward = forward_of(code);
            match (c.ctrl, c.alt) {
                (false, false) => Some(ExtendByCharacter { forward, collapse }),
                (false, true) => Some(ExtendToLineBreak {
                    forward,
                    collapse,
                    continues_at_wrap: false,
                }),
                (true, false) => Some(ExtendToNextWordBoundary { forward, collapse }),
                (true, true) => None,
            }
        }
        Code::ArrowUp | Code::ArrowDown => {
            let forward = forward_of(code);
            match (c.ctrl, c.alt) {
                (false, false) => Some(ExtendVerticallyToAdjacentLine { forward, collapse }),
                (false, true) => Some(ExtendToDocumentBoundary { forward, collapse }),
                (true, false) if c.shift => Some(ExtendToNextParagraphBoundary {
                    forward,
                    collapse: false,
                }),
                _ => None,
            }
        }
        Code::PageUp | Code::PageDown if !c.ctrl && !c.alt => {
            Some(ExtendVerticallyToAdjacentPage {
                forward: forward_of(code),
                collapse,
            })
        }
        _ => None,
    }
}

/// `_clipboardShortcuts`.
fn clipboard(code: Code, c: Chord) -> Option<TextIntent> {
    use TextIntent::*;
    let ctrl_only = c.only(true, false, false, false);
    match code {
        Code::KeyX if ctrl_only => Some(Cut),
        Code::KeyC if ctrl_only => Some(Copy),
        Code::KeyV if ctrl_only => Some(Paste),
        Code::Delete if c.only(false, false, true, false) => Some(Cut),
        Code::Insert if ctrl_only => Some(Copy),
        Code::Insert if c.only(false, false, true, false) => Some(Paste),
        Code::KeyA if ctrl_only => Some(SelectAll),
        Code::KeyZ if ctrl_only => Some(Undo),
        Code::KeyZ if c.only(true, false, true, false) => Some(Redo),
        // These keys go to the IME when a field is focused.
        Code::Space | Code::Enter if c.plain() => Some(DoNothing),
        _ => None,
    }
}

/// Home/End on Windows and Android continue across soft wraps.
fn home_end(code: Code, c: Chord, continues_at_wrap: bool) -> Option<TextIntent> {
    use TextIntent::*;
    if c.alt || c.meta {
        return None;
    }
    let forward = match code {
        Code::Home => false,
        Code::End => true,
        _ => return None,
    };
    let collapse = !c.shift;
    Some(if c.ctrl {
        ExtendToDocumentBoundary { forward, collapse }
    } else {
        ExtendToLineBreak {
            forward,
            collapse,
            continues_at_wrap,
        }
    })
}

fn windows(code: Code, c: Chord) -> Option<TextIntent> {
    home_end(code, c, true)
}

fn android(code: Code, c: Chord) -> Option<TextIntent> {
    home_end(code, c, true)
}

/// `_linuxShortcuts` plus `_linuxNumpadShortcuts`.
fn linux(code: Code, c: Chord) -> Option<TextIntent> {
    use TextIntent::*;
    if let Some(intent) = home_end(code, c, false) {
        return Some(intent);
    }
    if c.alt || c.meta {
        return None;
    }
    // With NumLock on the numpad needs Shift to navigate, with it off Shift
    // must be released; Control selects the next word or paragraph.
    if c.shift != c.num_lock {
        return None;
    }
    let collapse = !c.num_lock;
    let word_or_paragraph = c.ctrl;
    match code {
        Code::Numpad6 | Code::Numpad4 => {
            let forward = code == Code::Numpad6;
            Some(if word_or_paragraph {
                ExtendToNextWordBoundary { forward, collapse }
            } else {
                ExtendByCharacter { forward, collapse }
            })
        }
        Code::Numpad8 | Code::Numpad2 => {
            let forward = code == Code::Numpad2;
            Some(if word_or_paragraph {
                ExtendToNextParagraphBoundary { forward, collapse }
            } else {
                ExtendVerticallyToAdjacentLine { forward, collapse }
            })
        }
        Code::Numpad9 | Code::Numpad3 if !word_or_paragraph => {
            Some(ExtendVerticallyToAdjacentPage {
                forward: code == Code::Numpad3,
                collapse,
            })
        }
        Code::Numpad7 | Code::Numpad1 if !word_or_paragraph => {
            Some(ExtendVerticallyToAdjacentLine {
                forward: code == Code::Numpad1,
                collapse,
            })
        }
        Code::NumpadDecimal => Some(if word_or_paragraph {
            DeleteToNextWordBoundary { forward: true }
        } else {
            DeleteCharacter { forward: true }
        }),
        _ => None,
    }
}

/// `_macShortcuts` (also `_iOSShortcuts`).
fn mac(code: Code, c: Chord) -> Option<TextIntent> {
    use TextIntent::*;
    let collapse = !c.shift;
    let forward_of = |code| matches!(code, Code::ArrowRight | Code::ArrowDown | Code::PageDown);
    let mods = (c.ctrl, c.alt, c.meta);
    match code {
        // Delete shortcuts, with shift in either state.
        Code::Backspace | Code::Delete => {
            let forward = code == Code::Delete;
            match mods {
                (false, false, false) => Some(DeleteCharacter { forward }),
                (false, true, false) => Some(DeleteToNextWordBoundary { forward }),
                (false, false, true) => Some(DeleteToLineBreak { forward }),
                _ => None,
            }
        }
        Code::ArrowLeft | Code::ArrowRight => {
            let forward = forward_of(code);
            match (mods, c.shift) {
                ((false, false, false), _) => Some(ExtendByCharacter { forward, collapse }),
                ((false, true, false), false) => Some(ExtendToNextWordBoundary {
                    forward,
                    collapse: true,
                }),
                ((false, true, false), true) => {
                    Some(ExtendToNextWordBoundaryOrCaretLocation { forward })
                }
                ((false, false, true), false) => Some(ExtendToLineBreak {
                    forward,
                    collapse: true,
                    continues_at_wrap: false,
                }),
                ((false, false, true), true) => Some(ExpandToLineBreak { forward }),
                _ => None,
            }
        }
        Code::ArrowUp | Code::ArrowDown => {
            let forward = forward_of(code);
            match (mods, c.shift) {
                ((false, false, false), _) => {
                    Some(ExtendVerticallyToAdjacentLine { forward, collapse })
                }
                ((false, true, false), false) => Some(ExtendToLineBreak {
                    forward,
                    collapse: true,
                    continues_at_wrap: false,
                }),
                ((false, true, false), true) => {
                    Some(ExtendToNextParagraphBoundaryOrCaretLocation { forward })
                }
                ((false, false, true), false) => Some(ExtendToDocumentBoundary {
                    forward,
                    collapse: true,
                }),
                ((false, false, true), true) => Some(ExpandToDocumentBoundary { forward }),
                _ => None,
            }
        }
        Code::Home | Code::End if mods == (false, false, false) => {
            let forward = code == Code::End;
            Some(if c.shift {
                ExpandToDocumentBoundary { forward }
            } else {
                ScrollToDocumentBoundary { forward }
            })
        }
        Code::PageUp | Code::PageDown if mods == (false, false, false) => {
            let forward = code == Code::PageDown;
            Some(if c.shift {
                ExtendVerticallyToAdjacentPage {
                    forward,
                    collapse: false,
                }
            } else {
                ScrollPage { forward }
            })
        }
        Code::KeyX if c.only(false, false, false, true) => Some(Cut),
        Code::KeyC if c.only(false, false, false, true) => Some(Copy),
        Code::KeyV if c.only(false, false, false, true) => Some(Paste),
        Code::KeyA if c.only(false, false, false, true) => Some(SelectAll),
        Code::KeyZ if c.only(false, false, false, true) => Some(Undo),
        Code::KeyZ if c.only(false, false, true, true) => Some(Redo),
        // Emacs-style bindings.
        Code::KeyE if c.only(true, false, false, false) => Some(ExtendToLineBreak {
            forward: true,
            collapse: true,
            continues_at_wrap: false,
        }),
        Code::KeyA if c.only(true, false, false, false) => Some(ExtendToLineBreak {
            forward: false,
            collapse: true,
            continues_at_wrap: false,
        }),
        Code::KeyF if c.only(true, false, false, false) => Some(ExtendByCharacter {
            forward: true,
            collapse: true,
        }),
        Code::KeyB if c.only(true, false, false, false) => Some(ExtendByCharacter {
            forward: false,
            collapse: true,
        }),
        Code::KeyN if c.only(true, false, false, false) => Some(ExtendVerticallyToAdjacentLine {
            forward: true,
            collapse: true,
        }),
        Code::KeyP if c.only(true, false, false, false) => Some(ExtendVerticallyToAdjacentLine {
            forward: false,
            collapse: true,
        }),
        Code::KeyT if c.only(true, false, false, false) => Some(TransposeCharacters),
        Code::Space | Code::Enter if c.plain() => Some(DoNothing),
        _ => None,
    }
}
