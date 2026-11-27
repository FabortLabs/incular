# Keyboard type flags and value diagnostics

The pinned services `text_input_test.dart:640` registration pairs.
All 13 keyboard roles use the original Flutter indices and names.
Numeric variants independently preserve signed, decimal and password
options in equality, hashing, diagnostics and JSON; nonnumeric roles
retain null flags. The wire editing value uses the exact empty
selection and composing diagnostics. This is an explicit formatter;
ordinary Rust Debug continues to redact the text.

The new custom-keyboard role maps to Android TYPE_NULL and does not
open soft input. Web uses inputmode=none. Android maps Web Search
and Twitter to the pinned URI and email variations, respectively;
the web falls back to its ordinary text mode for both. The native
tests check these shared mappings. Ten focused platform tests and
the platform Wasm compile check passed. The full browser adapter
compile was blocked by an uncached dependency, and Android device
behavior was not exercised.
