# incular-cli

The `incular` command builds Incular apps for Android and iOS, installs them on
an emulator or simulator, and saves screenshots. It drives Cargo, Gradle, the
Android SDK tools and Xcode's command-line tools; it doesn't replace them.

```text
cargo run -p incular-cli -- build android --example counter --abi x86_64
cargo run -p incular-cli -- run   android --example counter --abi x86_64 --screenshot shots/counter.png
cargo run -p incular-cli -- build ios     --example counter
cargo run -p incular-cli -- run   ios     --example counter --screenshot shots/counter.png
```

## Android

- Needs `ANDROID_HOME` (or `%LOCALAPPDATA%/Android/Sdk`), an NDK under it or
  in `ANDROID_NDK_HOME`, the Rust target for the ABI, JDK 17 and `gradle` on
  `PATH` (or `INCULAR_GRADLE`).
- The app's binary is linked as a shared object. `incular-android`'s
  `JNI_OnLoad` registers the shell's native methods, and the activity calls
  the binary's C `main` once it starts, so apps keep an ordinary `fn main`.
- The tool writes a Gradle project to `target/incular/android/<name>` around
  the Kotlin shell shipped in `incular-android/android/`, then runs
  `assembleDebug`. `run` installs it with `adb`, launches it, waits, and saves
  a screenshot plus the app's logcat next to it.

## iOS

- Needs macOS with Xcode and the `aarch64-apple-ios-sim` (or
  `aarch64-apple-ios` for `--device`) Rust target.
- The tool builds the binary, wraps it in `target/incular/ios/<name>.app`
  with a generated `Info.plist` (UIScene manifest pointing at
  `IncularSceneDelegate`), and signs it ad hoc. `run` installs it on the booted
  simulator, launches it with its output captured, and takes a screenshot.

## Contract

| Contract | Status |
| --- | --- |
| Ownership | Developer tool that builds, installs and screenshots Incular apps on Android and iOS. |
| API class | application; this package is a tool, not a framework re-export surface. |
| Dependencies | `serde_json` for `cargo metadata`; no framework crates. |
| Support | Android APKs and iOS simulator bundles; device signing and store packaging are not yet supported. |
