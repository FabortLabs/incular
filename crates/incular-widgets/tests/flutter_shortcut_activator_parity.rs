//! Public activator contracts from Flutter 3.50.0 widgets/shortcuts_test.dart.

use incular_core::{Code, KeyState, KeyboardEvent, KeyboardKey, Modifiers};
use incular_widgets::{CharacterActivator, SingleActivator};

fn pressed(character: &str, code: Code, modifiers: Modifiers) -> KeyboardEvent {
    KeyboardEvent {
        modifiers,
        ..KeyboardEvent::key_down(KeyboardKey::Character(character.into()), code)
    }
}

fn repeat(event: &KeyboardEvent) -> KeyboardEvent {
    KeyboardEvent {
        repeat: true,
        ..event.clone()
    }
}

fn released(event: &KeyboardEvent) -> KeyboardEvent {
    KeyboardEvent {
        state: KeyState::Up,
        ..event.clone()
    }
}

#[test]
fn single_activator_accepts_key_repeat_by_default_and_can_opt_out() {
    let down = pressed("a", Code::KeyA, Modifiers::CONTROL);
    let repeat = repeat(&down);
    let up = released(&down);
    let default = SingleActivator::new(down.key.clone(), Modifiers::CONTROL);
    let no_repeat = default.clone().with_repeats(false);

    assert_eq!(
        [&down, &repeat, &up].map(|event| default.accepts(event)),
        [true, true, false]
    );
    assert_eq!(
        [&down, &repeat, &up].map(|event| no_repeat.accepts(event)),
        [true, false, false]
    );
}

#[test]
fn single_activator_matches_shortcut_modifiers_exactly_and_ignores_num_lock() {
    let activate = |modifiers| {
        let event = pressed("4", Code::Numpad4, modifiers);
        SingleActivator::new(event.key.clone(), Modifiers::CONTROL).accepts(&event)
    };
    assert_eq!(
        [
            Modifiers::empty(),
            Modifiers::CONTROL,
            Modifiers::CONTROL | Modifiers::SHIFT,
            Modifiers::CONTROL | Modifiers::ALT,
            Modifiers::CONTROL | Modifiers::META,
            Modifiers::CONTROL | Modifiers::NUM_LOCK,
        ]
        .map(activate),
        [false, true, false, false, false, true]
    );
}

#[test]
fn character_activator_uses_produced_character_accepts_repeats_and_ignores_shift() {
    let down = pressed("?", Code::Slash, Modifiers::SHIFT);
    let repeat = repeat(&down);
    let up = released(&down);
    let default = CharacterActivator::new("?");
    let no_repeat = default.clone().include_repeats(false);

    assert_eq!(
        [&down, &repeat, &up].map(|event| default.accepts(event)),
        [true, true, false]
    );
    assert_eq!(
        [&down, &repeat, &up].map(|event| no_repeat.accepts(event)),
        [true, false, false]
    );
    assert!(!default.accepts(&pressed("/", Code::Slash, Modifiers::empty())));
}

#[test]
fn character_activator_distinguishes_lowercase_and_uppercase_output() {
    let lower = CharacterActivator::new("a");
    let upper = CharacterActivator::new("A");
    let unshifted = pressed("a", Code::KeyA, Modifiers::empty());
    let shifted = pressed("A", Code::KeyA, Modifiers::SHIFT);

    assert_eq!(
        [
            lower.accepts(&unshifted),
            upper.accepts(&shifted),
            lower.accepts(&shifted),
            upper.accepts(&unshifted),
        ],
        [true, true, false, false],
        "Shift is ignored as a modifier but its produced character is case-sensitive"
    );
}
