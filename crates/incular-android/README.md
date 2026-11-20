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
- The Kotlin shell lives in `android/src/main/kotlin`; the CLI generates the
  Gradle project and manifest around it.
- `AndroidAccessibilityAdapter` exposes the incremental mobile semantics
  update/action contract for an Android View or Compose host.

See `system-design/MOBILE.md` for the full design.
