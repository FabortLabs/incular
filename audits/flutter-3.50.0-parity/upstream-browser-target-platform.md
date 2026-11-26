# Browser adaptive target platform

The pinned `foundation/platform_web_test.dart:20` and `:27`
registrations ran unchanged in the Chrome test backend. They pass
through the browser-selected iOS value and use Android for unknown.

Incular used the compile target for adaptive scroll defaults, so every
browser inherited Android even on Windows, Linux, and macOS. A separate
text shortcut detector could disagree about the same page. A shared
cached TargetPlatform::current now reads the same Navigator signals and
uses the pinned Flutter precedence. Native builds still keep their
compile-target default. The current platform also supplies the default
scroll policy, initial focus-highlight mode, and editing shortcuts.
Explicit scroll and text-platform overrides remain available.

The compiled WASM probe exercises the real browser binding in headless
Chrome with seven Navigator profiles. It checks touch-capable Mac
identification at the exact two/three-touch boundary, explicitly
identified iOS, Android's Linux platform string, desktop Linux and
Windows, and unknown fallback. Repeated reads remain cached. These
profiles emulate Navigator values in Chrome, not seven physical hosts.

The Rust classifier and native default checks also passed. The browser
probe can be repeated with the wasm32 Rust target, Node, Chrome, and the
wasm-bindgen CLI matching the workspace lock:

```sh
bash audits/flutter-3.50.0-parity/native-probes/browser-platform/run.sh /path/to/chrome
```

The focused native consumers passed 37 checks across scroll defaults,
keyboard routing, and formatting. The unrelated legacy
control_y_is_not_a_redo_shortcut fixture failed before it reached the
Ctrl+Y assertion: its programmatic edit was still pending when it
tried Ctrl+Z. That explicit-Windows history test was excluded from
the native browser regression rerun and is being investigated
separately.
