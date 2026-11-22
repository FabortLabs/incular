# incular-android

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Android embedder shell: JNI activity bridge, surfaces, touch, insets, lifecycle and semantics adapter. |
| API class | backend; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Partial; surfaces, touch, insets and lifecycle run on Android 8.0+ (API 26). Text input, the accessibility bridge and native services follow the mobile spec phases. |

Android platform integration for Incular.

- The app's binary is linked as a shared object (the `incular` CLI does this).
  `JNI_OnLoad` registers the native methods of `dev.incular.IncularNative`, and
  `IncularActivity` calls the binary's C `main` once it is created, so apps
  keep an ordinary `fn main` that calls `incular::run`.
- `incular::run` hands the application to `run_application`, which installs it
  for the shell. The shell drives an `incular_mobile::MobileEngine` from a
  `SurfaceView`, `AChoreographer` frame callbacks and a looper-registered wake
  pipe, and forwards logs to logcat under the `Incular` tag.
- Before `main` runs, the shell publishes the `JavaVM` and the application
  context through [`ndk-context`](https://docs.rs/ndk-context). Crates that
  call into Android read them from there, for example `webbrowser` to open a
  link.
- The `platform-verifier` feature (`incular/platform-verifier`) also
  initializes [`rustls-platform-verifier`](https://docs.rs/rustls-platform-verifier).
  HTTPS clients that verify through it, such as reqwest's default `rustls`
  backend, then use the device's trust store with no app code. Without it
  they panic on the first request. When the crate is in the dependency
  graph, `incular build android` adds its Kotlin component to the generated
  Gradle project.
- System settings follow Flutter's Android embedder: `nativeSettings` reports
  the night-mode flag as brightness, `fontScale` as text scale, bold text from
  the font weight adjustment (API 31+), reduced motion from
  `TRANSITION_ANIMATION_SCALE` of 0, and the `LocaleList` as BCP-47 tags. The
  values are sent at start, on resume and on `onConfigurationChanged`.
- Text input follows Flutter's `TextInputPlugin`. `IncularInputView` opens
  the soft keyboard for the focused field and reports the input method's
  whole editing state; hardware keys go to key handlers and fields; copy and
  paste use the system clipboard.
- The back button reaches the focused key handlers as `Code::BrowserBack`,
  then closes popups, then asks the pop-route observers, newest first.
  `Navigator` and `WidgetsApp` register themselves, so their routes pop
  without app code, and the predictive back gesture follows the top route.
  If nothing takes it, the activity finishes. Apps that keep their own route
  stack should register a `BackGestureObserver` rather than rely on a key
  handler, because a key handler hears back only while it is on the focus
  path.
- A finished activity keeps the Rust application alive in the process, so
  relaunching from the launcher resumes the same state, as Flutter does with
  a cached engine. There is no detached state to restore.
- The Kotlin shell lives in `android/src/main/kotlin`; the CLI generates the
  Gradle project and manifest around it.
- `AndroidAccessibilityAdapter` exposes the incremental mobile semantics
  update/action contract for an Android View or Compose host.

See `system-design/MOBILE.md` for the full design.
