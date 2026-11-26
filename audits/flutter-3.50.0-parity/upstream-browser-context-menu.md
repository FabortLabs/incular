# Browser context menu

All four pinned `services/browser_context_menu_test.dart`
registrations passed: two in the native Flutter tester and two
with the original source running in Chrome. The native Rust
tests verify neither non-web assertion reaches transport or
changes the default state.

The Incular WASM probe in Chrome uses the real DOM adapter and
checks both exact flutter/contextmenu JSON commands with null
arguments. Without any unrelated handlers, a right-click event is
initially allowed. Disabling prevents the event's default action
on the view and its canvas child, not on a sibling page control.
Repeated calls leave one listener; enabling or dropping removes
it. Browser-side state follows the completed reply. Five targeted
native platform and codec checks passed in addition to the WASM
probe.

Reproduce the upstream web half with `run-reference-chrome.sh`
and the Incular half with `native-probes/browser-context-menu/run.sh`,
passing a Chrome binary. The native runner also needs the
wasm32 target and matching wasm-bindgen CLI.
