//! Button-mask helpers for Incular's portable raw pointer events.

/// The bit of the one-based mouse button, or None outside the portable mask.
#[must_use]
pub const fn nth_mouse_button(number: u32) -> Option<u32> {
    match number.checked_sub(1) {
        Some(bit) => 1_u32.checked_shl(bit),
        None => None,
    }
}

/// The bit of the one-based stylus side button. The lowest bit is reserved
/// for contact with the stylus point; side buttons begin at bit one.
#[must_use]
pub const fn nth_stylus_button(number: u32) -> Option<u32> {
    if number == 0 {
        None
    } else {
        1_u32.checked_shl(number)
    }
}

/// Least significant set bit, or zero for an empty button chord.
#[must_use]
pub const fn smallest_button(buttons: u32) -> u32 {
    buttons & buttons.wrapping_neg()
}

/// Whether exactly one button is down. Zero and chords both return false.
#[must_use]
pub const fn is_single_button(buttons: u32) -> bool {
    buttons.is_power_of_two()
}
