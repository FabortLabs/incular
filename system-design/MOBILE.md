# Incular on Android and iOS

Status: **proposed** (2026-11-21). Nothing here is implemented beyond the
semantic adapters in `incular-android` and `incular-ios`. When a phase lands,
move the parts it settles into [ARCHITECTURE.md](ARCHITECTURE.md),
`specs/architecture.json` and the crate READMEs, and delete them from here.

This spec covers how Incular runs on phones and tablets: hosting, rendering,
input, text editing, accessibility, lifecycle, native interop, permissions,
the manifest and `Info.plist` content we generate, the project layout, the
build tool, testing and the order of work. Platform facts that change every
year (SDK levels, store deadlines) are collected in
[Appendix B](#appendix-b-platform-facts-to-re-verify) so they can be checked
in one place before each phase starts.

## 1. Goals and non-goals

Goals:

1. An Incular application builds, runs and ships to Google Play and the App
   Store from one Rust codebase, with the same widget tree as desktop.
2. Flutter-level fidelity on mobile: 120 Hz rendering, touch and stylus
   input, full IME and autofill, platform accessibility, safe areas and the
   keyboard inset, predictive back, system UI styling, state restoration.
3. One declaration of what the app needs (permissions, deep links,
   background modes, icons) from which every native file is generated.
   Developers should not have to learn `AndroidManifest.xml` or `Info.plist`
   for ordinary apps, and should still be able to override anything.
4. Native interop without a message bus: Rust plugin crates call Kotlin and
   Swift directly, and ship their native sources inside the crate.
5. Add-to-app: an existing Android or iOS app can host an Incular view.

Non-goals for this spec:

- Reproducing Flutter's Dart-side APIs or its method-channel codec. Flutter is
  the reference for behavior, not for architecture (hard rule 8).
- watchOS, tvOS, Wear OS, Android TV, Android Auto, CarPlay, visionOS.
  The design should not block them, but nothing here targets them.
- Building native projects without Gradle or Xcode. Both are required for
  store submission and for the third-party native ecosystem.

## 2. Decisions at a glance

| Question | Decision | Section |
| --- | --- | --- |
| Who owns the event loop on mobile? | The platform. Incular is an embedded engine driven by a thin Kotlin/Swift shell, like Flutter's embedder. Not Winit. | §3 |
| Threads | One: the platform main thread runs runtime, layout, paint and GPU submission (Flutter merged its UI and platform threads on mobile in 3.29 for the same reasons). Async work stays on Tokio. | §4.2 |
| GPU | WGPU: Vulkan on Android with GLES 3 fallback, Metal on iOS. | §5 |
| Native shells | Kotlin library (`IncularActivity`, `IncularFragment`, `IncularView`) and Swift package (`IncularViewController`, `IncularView`, SwiftUI wrapper). All policy stays in Rust; shells translate. | §4 |
| Rust ↔ native | JNI (`RegisterNatives`, no mangled exports) on Android; a small C ABI called from Swift on iOS. Plugins call platform APIs directly through `jni` / `objc2` or their own Kotlin/Swift. | §11 |
| App configuration | `Incular.toml` next to the app's `Cargo.toml`. Plugin crates use `[package.metadata.incular]`. | §14 |
| Permissions | Declared once as portable capabilities with a user-facing reason; the CLI generates manifest entries, `Info.plist` usage strings, entitlements and the privacy manifest; a runtime API requests them and refuses undeclared ones with a typed error. | §12 |
| Native projects | Generated into `target/incular/<platform>/` on every build, never hand-edited. Overrides live in `platform/android/` and `platform/ios/` overlays that are merged in. | §14.3 |
| Package managers | Gradle with Maven dependencies on Android; Swift Package Manager only on iOS (no CocoaPods). | §15 |
| Build tool | New `incular` CLI (`tools/incular-cli`): `create`, `doctor`, `run`, `build`, `devices`, `logs`. | §15 |
| Minimum OS | Android 8.0 (API 26); iOS 16. | §15.4 |

## 3. Hosting model

### 3.1 Why not Winit on mobile

Hard rule 2 ("one Winit host") works on desktop and the web because the
window system there is a passive surface plus an event stream. On Android and
iOS the platform owns far more of the contract, and Winit cannot expose it:

- **Text input.** Android IMEs talk to a focused `View` through an
  `InputConnection` (composing regions, `deleteSurroundingText`, batch edits,
  autofill structure). iOS keyboards talk to a `UIView` conforming to
  `UITextInput` (marked text, selection rects, autocorrect, dictation,
  Scribble). Winit's Android backend runs on `NativeActivity`/`GameActivity`
  and its iOS backend exposes only basic IME events; neither can implement
  these protocols on our behalf.
- **Accessibility.** TalkBack needs an `AccessibilityNodeProvider` on a real
  `View`; VoiceOver needs `UIAccessibilityElement` containers on a real
  `UIView`. Our existing adapters already produce exactly that data for a
  native bridge.
- **Add-to-app and platform views.** Embedding in an existing app, or putting
  a native map or web view next to Incular content, requires Incular to be a
  `View`/`UIView` inside someone else's hierarchy. Winit owns
  `UIApplicationMain` and the whole `Activity`.
- **Lifecycle details**: surface destruction separate from activity stop,
  `onTrimMemory`, `UIScene` sessions, predictive back callbacks, window
  insets animations, Choreographer and `CADisplayLink` frame timing.

So mobile uses the model Flutter uses: the platform shell owns the process and
the views, and Incular is an engine it drives.

### 3.2 One host core, two shells

To keep one frame loop, one input path and one renderer path (the intent of
hard rule 2), the parts of `incular-desktop` that do not depend on Winit move
into a new crate that both shells drive:

```text
application
  -> incular facade
     -> runtime, widgets, controls/material

host        -> runtime + incular-wgpu + accessibility + platform   (NEW, extracted)
desktop     -> host + winit                                        (Winit shell)
mobile      -> host                                                (NEW, embedder shell)
windows / macos / linux / web -> desktop
android / ios -> mobile
```

| Crate | Owns |
| --- | --- |
| `incular-host` (new, extracted from `incular-desktop`) | Driving the runtime for a set of views: frame scheduling and pacing input, presentation and surface reconfiguration through `incular-wgpu`, semantics sync hand-off, devtools overlay, input normalization from portable events. No Winit, no OS APIs. |
| `incular-desktop` | The Winit event loop, Winit → portable event translation, desktop native services. Same public role as today. |
| `incular-mobile` (new) | The mobile embedder state machine shared by Android and iOS: engine and view registry, lifecycle mapping, vsync-driven frame pump, touch/stylus normalization, text-input client protocol, insets and keyboard animation, permission broker, plugin registry, platform-view bookkeeping, panic containment at the FFI boundary. Portable Rust, testable on the host machine. |
| `incular-android` | JNI entry points and the Kotlin library under `crates/incular-android/android/`; `ANativeWindow` surfaces; Choreographer vsync; `AAssetManager` asset bundle; logcat; Android services (clipboard, haptics, share, URL launch, permissions, insets). Keeps the existing `AndroidAccessibilityAdapter`. |
| `incular-ios` | The C ABI used by the Swift package under `crates/incular-ios/apple/`; `CAMetalLayer` surfaces; `CADisplayLink`; `UIScene` lifecycle; `os_log`; iOS services. Keeps the existing `IosAccessibilityAdapter`. |
| `incular-cli` (new tool, `tools/incular-cli`, binary `incular`) | Project creation, configuration validation, native project generation, Rust cross-compilation, Gradle/Xcode invocation, device deployment, logs. Not a library dependency of apps. |

The extraction of `incular-host` is the first piece of work (phase 0) and is
worth doing on its own: it makes the Winit dependency a thin shell and lets
the frame loop be tested headlessly.

Changes this requires in the contract, made in the same change as the code:

- ARCHITECTURE.md hard rule 2 becomes: *"One host core. Desktop and browser
  entry paths share the Winit shell; Android and iOS share the embedder shell.
  Both shells drive `incular-host`, so there is still one frame loop, one input
  normalization path, one renderer path and one semantics path."*
- `specs/architecture.json` gains `incular-host`, `incular-mobile` and the
  `incular-cli` tool, and the edges above. `incular-android` and `incular-ios`
  change owner text from "semantic adapter" to "Android/iOS embedder".
- `incular-platform::NativeWindowSystem` gains `Android` and `UiKit`.

### 3.3 Facade and entry point

New facade features, mirroring `desktop`/`web`:

```toml
mobile-host = ["runner", "dep:incular-platform", "dep:incular-wgpu", "dep:incular-android", "dep:incular-ios"]
mobile = ["mobile-host", "vulkan", "gl", "metal"]   # only the target's adapter compiles
```

`incular-android` and `incular-ios` are target-gated dependencies of the
facade (`cfg(target_os = "android")`, `cfg(target_os = "ios")`), as the desktop
adapters are today.

Apps keep the ordinary binary entry point they use on desktop and the web:
`fn main() { incular::run(app) }`. No macro, no library crate and no crate
type settings are needed (implemented in phase 1):

- On Android, the CLI links the app's binary as a shared object
  (`cargo rustc --example|--bin … -- -C link-arg=-shared -C
  link-arg=-Wl,-u,JNI_OnLoad`). `incular-android` exports `JNI_OnLoad`,
  which registers the engine's native methods on `dev.incular.IncularNative`
  with `RegisterNatives`. When `IncularActivity` is created it calls the
  binary's C `main` symbol; `incular::run` then installs the `Application`
  with the shell instead of starting an event loop, and returns.
- On iOS, the binary is the bundle executable. `incular::run` calls
  `UIApplicationMain` with Rust-defined (`objc2`) `IncularAppDelegate` and
  `IncularSceneDelegate` classes; the generated `Info.plist` names the scene
  delegate. A Swift package for add-to-app hosts (§4, phase 4) will call the
  same engine through a small C ABI.

> Phase 1 note: the iOS shell is Rust/objc2 only, so the simulator and device
> builds need no Xcode project or Swift. The Swift package described below is
> still the plan for embedding into existing apps.

## 4. The embedder contract

### 4.1 Objects

| Concept | Android | iOS | Rust (`incular-mobile`) |
| --- | --- | --- | --- |
| Engine: one `Application`, runtime, GPU device, plugin registry | `IncularEngine` (can be pre-warmed and cached in an `IncularEngineCache`, as Flutter does for add-to-app) | `IncularEngine` | `Engine` |
| View: one runtime window and one surface | `IncularView` (a `FrameLayout` containing a `SurfaceView`, or a `TextureView` when transparency or animation inside another layout is required) | `IncularView` (a `UIView` backed by `CAMetalLayer`) | `View` → `WindowId` |
| Screen-level host | `IncularActivity`, `IncularFragment` | `IncularViewController`, `IncularSceneDelegate`, SwiftUI `IncularRepresentable` | — |

A full-screen Incular app is an `IncularActivity` / `IncularSceneDelegate`
generated by the CLI. Add-to-app uses `IncularFragment` / `IncularView` /
`IncularViewController` directly. One engine can serve several views (iPad
multiple scenes, Android multi-window, or several Incular panes in a native
app); each view maps to one runtime `WindowId`. Desktop window-control
capabilities (move, resize, decorations, levels) report `Unsupported`.

### 4.2 Threading

Everything that touches the runtime runs on the platform main thread:
build, layout, paint, compositing, GPU submission, semantics projection and
native callbacks. This matches the desktop host, avoids cross-thread copies
of the semantics and text-input state that the platform reads synchronously
(TalkBack and `UITextInput` call back on the main thread and expect immediate
answers), and is the model Flutter moved to on mobile.

- Long work goes to Tokio (already the runtime's async executor) and returns
  through the existing task completion path.
- Frame budget enforcement is a performance concern, not a threading one:
  the steady-state no-allocation rule and dirty-flag skipping matter more on
  phones. If profiling later shows GPU submission must leave the main thread,
  that is a contained change inside `incular-host`.
- Native callbacks never mutate runtime state across the FFI stack (the
  existing rule in `DesktopPlatformServices`). They enqueue a typed event and
  request a frame; the frame pump drains the queue. The exceptions are the
  synchronous queries the platform makes (accessibility node lookups,
  `UITextInput` geometry, `InputConnection.getTextBeforeCursor`), which read
  from snapshots the engine published at the end of the last frame.

### 4.3 Lifecycle

Native events map to `PlatformLifecycle` (and from there to the runtime's
`ApplicationLifecycle`). Surface lifetime is tracked separately because on
Android the surface can be destroyed while the activity lives.

| Android | iOS (`UIScene`) | `PlatformLifecycle` | Engine action |
| --- | --- | --- | --- |
| `onCreate` | `scene(_:willConnectTo:)` | (engine `Starting`) | Create or attach engine, restore state |
| `onStart` | `sceneWillEnterForeground` | `Resumed` → `Inactive` | — |
| `onResume` / window focus gained | `sceneDidBecomeActive` | `Active` | Resume animations, tickers |
| `onPause` / focus lost | `sceneWillResignActive` | `Inactive` | Keep rendering (split screen, system dialogs) |
| `onStop` | `sceneDidEnterBackground` | `Suspended` | Stop the frame pump, flush restoration, release transient GPU caches |
| `onDestroy` (finishing) | `sceneDidDisconnect` | `Stopping` | Detach view; dispose engine unless cached |
| `surfaceCreated` / `surfaceChanged` | layer attached / `layoutSubviews` size or scale change | — | Create or reconfigure the WGPU surface |
| `surfaceDestroyed` | view removed from window | — | Drop the WGPU surface *before returning* (Android requires it); keep the device |
| `onTrimMemory`, `onLowMemory` | `didReceiveMemoryWarning` | — | New `MemoryPressure { level }` event routed to image, glyph and GPU cache owners (hard rule 7) |
| `onSaveInstanceState` | `stateRestorationActivity(for:)` | — | Serialize the runtime restoration snapshot into the bundle / `NSUserActivity` |

Requirements that come from the platforms:

- iOS apps use the `UIScene` lifecycle from day one. Apple has announced it
  becomes mandatory for apps built with SDKs after iOS 26; the generated
  `Info.plist` always contains a `UIApplicationSceneManifest`.
- Android process death is normal. Restoration (already in the runtime) must
  round-trip through `onSaveInstanceState`; the bundle limit is ~500 KB in
  practice (`TransactionTooLargeException`), so the CLI-generated activity
  stores large snapshots in a file and only a key in the bundle.
- Configuration changes do not recreate the activity: the generated manifest
  sets `android:configChanges` for orientation, screen size, smallest screen
  size, screen layout, density, UI mode, keyboard, keyboard hidden,
  navigation, font scale, layout direction and locale, and the engine applies
  each change as an environment update (as Flutter does).

### 4.4 Frame pump

- Android: `Choreographer.FrameCallback` in Kotlin forwards
  `frameTimeNanos` to Rust (works on every supported API level; avoids the
  `AChoreographer` API-level split). High refresh rate via
  `Surface.setFrameRate` / `Display.Mode` preference when the app opts in.
- iOS: `CADisplayLink` with `preferredFrameRateRange` up to the display max.
  ProMotion above 60 Hz on iPhone requires
  `CADisableMinimumFrameDurationOnPhone = true` in `Info.plist`; the CLI sets
  it.
- The pump only runs while a frame is requested (`frame_requested`), so an
  idle app costs nothing, as on desktop.
- Rotation and keyboard animations need the frame and the native layout
  change to land together: on iOS present with
  `CAMetalLayer.presentsWithTransaction` during those transitions; on Android
  apply `WindowInsetsAnimation` progress inside the same Choreographer frame.

### 4.5 Errors, panics and logging

- Every JNI and C ABI entry point runs inside `catch_unwind`. A panic marks
  the engine poisoned, logs the payload and backtrace, and returns an error
  code; the shell then crashes natively with the Rust message (an
  `IncularEngineException` on Android, `fatalError` on iOS) so crash
  reporters see a real crash instead of a frozen screen.
  Unwinding never crosses the FFI boundary.
- `tracing` output goes to logcat (`__android_log_write`) and `os_log`
  (subsystem = bundle identifier) through a subscriber installed by the
  embedder. `println!`/`eprintln!` are not visible on Android; the docs must
  say so.
- Typed backend errors (`RunError` equivalents) surface to the shell's
  `onEngineError` callback, never as silent black screens.

## 5. Rendering

- **Android:** WGPU Vulkan on an `ANativeWindow` obtained from the
  `SurfaceView`'s `Surface` (`ANativeWindow_fromSurface`). Fall back to GLES 3
  (EGL) when Vulkan is missing, below 1.1, or on a driver denylist kept in
  `incular-wgpu/driver_workarounds.rs`. Pre-rotation: query the surface
  transform and render rotated rather than letting the compositor rotate,
  which costs a full-screen pass on many GPUs.
- **iOS:** WGPU Metal on the view's `CAMetalLayer`. Track `contentsScale`
  from the trait collection; resize drawable size in `layoutSubviews`.
- **Transparent and translucent views:** `TextureView` (Android) and
  non-opaque layers (iOS) are opt-in because they cost composition.
- **Device loss** (backgrounded GL contexts, GPU resets) is a typed event:
  the host recreates the device and reuploads from CPU-side owners, which
  the existing cache ownership model already supports.
- **Shader warm-up:** WGPU pipelines are created on first use; first-frame
  jank on mobile is more visible. Phase 2 adds pipeline caching
  (`wgpu::PipelineCache` on Vulkan) persisted in the app cache directory.
- **Memory:** mobile GPU budgets are smaller. `incular-wgpu` cache budgets
  scale from the device's reported memory class
  (`ActivityManager.getMemoryClass`, `os_proc_available_memory`).
- **Screenshots and parity** reuse the existing capture path.

## 6. Input

### 6.1 Touch and stylus

- Android `MotionEvent`: every pointer, including historical samples
  (`getHistorical*`) so fast strokes are not decimated; `ACTION_CANCEL`
  cancels every active pointer; tool type maps to `PointerDeviceKind`
  (finger, stylus, eraser, mouse); pressure, size, tilt, orientation; hover
  (`ACTION_HOVER_*`) for stylus and mouse; buttons for mice.
- iOS `UITouch`: coalesced touches for full sample rate, predicted touches
  exposed for drawing apps, `force` / `maximumPossibleForce`, Apple Pencil
  `altitudeAngle` / `azimuthAngle`, Pencil hover (`UIHoverGestureRecognizer`),
  `UIPointerInteraction` for iPad trackpads, and
  `UIApplicationSupportsIndirectInputEvents = true` in `Info.plist` so trackpad
  clicks arrive as indirect pointer events rather than touches.
- All of this lands in the existing `InputEvent::PointerWithId` path with
  `NativePointerSample` metadata. Gesture arbitration stays in
  `incular-gestures`; the platform's own gesture recognizers are not used for
  in-app gestures.
- **System gestures:** Android back swipe and iOS edge swipes must not be
  stolen. On Android the generated activity sets
  `android:enableOnBackInvokedCallback="true"` and routes
  `OnBackAnimationCallback` progress into the existing `PredictiveBackEvent`;
  `systemGestureExclusionRects` is exposed for widgets that need edge drags
  (sliders near screen edges). On iOS the view controller's
  `preferredScreenEdgesDeferringSystemGestures` is driven from the runtime.

### 6.2 Hardware keyboards

Android `KeyEvent` and iOS `UIPress` (`UIKey`) map to the existing keyboard
events with physical and logical keys. Shortcut discovery: iPad's
`UIKeyCommand` menu (hold ⌘) and Android's keyboard shortcuts helper are a
later phase.

## 7. Text input, IME and autofill

This is the largest part of the embedder. The existing `TextInputCommand`
(engine → platform) and `ImeEvent` (preedit/commit) contracts are not enough:
mobile IMEs edit the text directly. `incular-platform` gains the reverse
direction, modelled on Flutter's `TextInputClient` protocol:

```text
TextInputEvent (platform -> engine)
  UpdateEditingState { client, state: TextInputState }        // full value
  UpdateEditingDeltas { client, deltas: Vec<TextEditingDelta> } // when enabled
  PerformAction { client, action: TextInputAction }
  PerformSelector { client, selector }                          // iOS deleteBackward:, moveLeft:, ...
  FloatingCursor { client, phase, offset }                      // iOS space-bar trackpad
  InsertContent { client, uri, mime }                           // keyboard images/GIFs
  ConnectionClosed { client }
TextInputCommand (engine -> platform), additions
  SetEditableSizeAndTransform { client, size, transform }
  SetMarkedTextRect / SetSelectionRects { client, rects }      // iOS geometry queries
  SetStyle { client, font, size, align, direction }            // IME candidate positioning
  RequestAutofill / FinishAutofillContext
```

Rules:

- The engine is the source of truth; the native side keeps a mirror
  (`Editable` on Android, an internal buffer on iOS). Both sides carry a
  version counter so stale updates from the IME are rejected (the classic
  Flutter "text jumps back" bug class).
- Offsets cross the boundary as UTF-16 code units (what both platforms use)
  and are converted to UTF-8 byte offsets inside `incular-mobile`, with a
  test suite of surrogate pairs, combining marks and emoji ZWJ sequences.
- **Android:** `IncularView.onCreateInputConnection` builds `EditorInfo` from
  `TextInputConfiguration` (input type flags, `imeOptions` action,
  `IME_FLAG_NO_PERSONALIZED_LEARNING`, `initialSelStart/End`, surrounding
  text) and returns an `InputConnection` over the mirror. Batch edits are
  coalesced into one `UpdateEditingState`. `windowSoftInputMode` is
  `adjustResize` (the keyboard inset is handled by the engine, §8).
- **iOS:** `IncularView` hosts a hidden first-responder view conforming to
  `UITextInput` and `UITextInputTraits` (`keyboardType`, `returnKeyType`,
  `autocapitalizationType`, `autocorrectionType`, `spellCheckingType`,
  `smartQuotesType`, `textContentType`, `secureTextEntry`). Marked text,
  `firstRect(for:)`, `caretRect(for:)` and `selectionRects(for:)` answer from
  the published geometry snapshot. Scribble (`UIScribbleInteraction`),
  dictation and the system edit menu (`UIEditMenuInteraction`) are wired in
  phase 2.
- **Autofill:** Android `AutofillManager` with a virtual `ViewStructure` for
  each autofillable field (`autofill_hints` → `View.AUTOFILL_HINT_*`); iOS
  `textContentType` per field and password autofill through the hidden input
  views. `TextInput.finishAutofillContext` maps to
  `AutofillManager.commit()` / saving credentials on iOS. Password autofill
  on iOS also needs the `webcredentials:` associated domain (§13.4).
- **Selection UI** (handles, magnifier, toolbar) is drawn by Incular widgets,
  as in Flutter, so it matches across platforms; the native edit menu is an
  optional `ContextMenu` backend on iOS 16+.

## 8. Insets, system UI and orientation

- **Edge-to-edge is the only mode.** Android 15 enforces edge-to-edge for
  apps targeting API 35 and API 36 removed the opt-out, so the engine always
  draws behind the status and navigation bars and reports insets. iOS has
  always worked this way.
- The existing `SystemEnvironmentPreferences` fields are the contract:
  - `view_padding`: system bars, display cutout, rounded corners, home
    indicator (Android `WindowInsets` system bars ∪ display cutout; iOS
    `safeAreaInsets`);
  - `view_insets`: the soft keyboard (Android `WindowInsets.Type.ime()`, iOS
    `keyboardLayoutGuide` or keyboard frame notifications);
  - `safe_insets`: what is currently usable.
- Keyboard animation is reported frame-by-frame (Android
  `WindowInsetsAnimationCompat.Callback`, iOS keyboard notification curve and
  duration applied per frame) so scroll views and bottom bars move with the
  keyboard instead of jumping.
- **System UI style** (`SystemUiOverlayStyle` equivalent): status bar
  icon brightness, navigation bar contrast enforcement, immersive / sticky
  immersive modes (`WindowInsetsController.hide` with
  `BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE`), iOS `prefersStatusBarHidden`,
  `preferredStatusBarStyle`, `prefersHomeIndicatorAutoHidden`. New portable
  commands in `incular-platform`, reported through capabilities.
- **Orientation:** `preferredOrientations` maps to
  `Activity.setRequestedOrientation` and
  `UIViewController.supportedInterfaceOrientations` +
  `UIWindowScene.requestGeometryUpdate`. The static default comes from
  `Incular.toml`. Android 16 ignores orientation and resizability locks on
  large screens (smallest width ≥ 600 dp) for apps targeting API 36, so
  layouts must be responsive; the docs and `incular doctor` must say so.
- **Display features:** foldable hinges and folds (Jetpack WindowManager
  `FoldingFeature`) and the iPhone Dynamic Island cutout become a
  `display_features` list in the environment (Flutter
  `MediaQuery.displayFeatures`). Phase 3.
- **Other services** reported through `PlatformCapabilities`: haptics
  (`View.performHapticFeedback`, `UIFeedbackGenerator`), system sounds,
  keep-screen-on (`FLAG_KEEP_SCREEN_ON`, `isIdleTimerDisabled`, no
  permission needed), share sheet, URL launching, clipboard (Android 13+
  shows a paste confirmation; iOS 16+ shows a paste prompt unless the paste
  comes from `UIPasteControl`), screen brightness, app review prompt.
- `content_sensitivity` (already in the platform crate) maps to
  `FLAG_SECURE` on Android and a secure text-field layer trick on iOS
  (documented as best-effort there).

## 9. Accessibility

The existing `MobileAccessibilityProjection` and the two adapters are the
data side. The embedder adds the native side:

- **Android:** `IncularView.getAccessibilityNodeProvider()` returns a provider
  whose `createAccessibilityNodeInfo(virtualId)` reads the last projected
  snapshot; `performAction` goes through `AndroidAccessibilityAdapter::action`
  and is queued for the next frame, with `take_optimistic_update` applied
  immediately as Flutter does. Events
  (`TYPE_WINDOW_CONTENT_CHANGED`, `TYPE_VIEW_ACCESSIBILITY_FOCUSED`,
  announcements, live regions) are sent from the incremental update.
- **iOS:** `IncularView` is an accessibility container
  (`accessibilityElements`) of `UIAccessibilityElement` subclasses mirroring
  the snapshot; scroll containers implement `accessibilityScroll`; custom
  actions, `accessibilityIncrement/Decrement`, `UIAccessibility.post`
  notifications for screen and layout changes.
- Preferences (bold text, reduce motion, invert colors, high contrast,
  on/off switch labels, text scale) already exist in
  `SystemEnvironmentPreferences`; the embedder fills them from
  `Settings.Global`/`Configuration.fontScale` and `UIAccessibility` /
  `UIContentSizeCategory`, and listens for changes.
- Verified with TalkBack and VoiceOver on real devices per phase, plus
  Android's Accessibility Scanner and Xcode's Accessibility Inspector.

## 10. Assets, fonts and files

- **Asset bundling.** `Incular.toml` lists asset globs (§14.2). The CLI copies
  them into `assets/` of the APK/AAB and into the app bundle's resources on
  iOS, preserving relative paths.
- **Asset access.** Android assets are not files: a new `AndroidAssetBundle`
  (implementing the existing `AssetBundle` trait) reads through
  `AAssetManager`, mapping uncompressed assets with `AAsset_getBuffer` to
  avoid copies (the CLI tells AAPT2 not to compress font and image
  extensions). On iOS the existing `DirectoryAssetBundle` over
  `Bundle.main.resourcePath` works unchanged.
- `std::fs` paths used today (`incular-assets/bundle.rs`,
  `incular-image`) must go through the bundle abstraction on mobile; reading
  relative paths from the current directory is a bug there.
- **System fonts.** Android: enumerate `/system/fonts` and read the platform
  fallback chain from `ASystemFontIterator` (API 29+) or `fonts.xml` (26–28).
  iOS: system fonts (SF Pro, the CJK and emoji fonts) are not readable as
  plain files from every path; load them through CoreText
  (`CTFontCopyDefaultCascadeListForLanguages` for fallback, font URLs or
  table data for bytes). Both feed `incular-text` font selection so Incular
  uses the platform's default face, as Flutter does (Roboto / SF).
- **App directories.** Android passes `Context.getFilesDir()` to
  `nativeCreate`, which hands it to `incular_platform::set_application_data_root`
  before the app's `main` runs, so `application_data_local_directory` returns
  `<filesDir>/<application id>`. On iOS `directories::ProjectDirs` already
  resolves `Library/Application Support` inside the app container. A cache
  directory (`getCacheDir()`, `Library/Caches`) follows when a caller needs it.
- **File pickers:** Android Storage Access Framework and Photo Picker
  (no permission needed); iOS `UIDocumentPickerViewController` and
  `PHPickerViewController` (no permission needed). These map to the existing
  `FileDialogRequest` contract.

## 11. Native interop and plugins

### 11.1 Calling platform APIs from Rust

Incular does not need Flutter's message channels: Rust can call the platform
directly. The engine exposes, per platform, the handles a plugin needs:

```rust
// incular-android
pub struct AndroidContext { /* JavaVM, Activity/Context global refs, main-thread looper */ }
impl AndroidContext {
    pub fn with_env<R>(&self, f: impl FnOnce(&mut jni::JNIEnv, &JObject /* activity */) -> R) -> R;
    pub fn activity_result_launcher(&self) -> ActivityResults; // startActivityForResult / permission results
}

// incular-ios
pub struct IosContext { /* MainThreadMarker, UIWindowScene, root UIViewController */ }
```

These are backend-class APIs (ARCHITECTURE.md API classes) reached through
`incular::android` / `incular::ios`, cfg-gated, and only valid on the main
thread (`MainThreadMarker` on iOS makes that a type error).

### 11.2 Plugin crates

A plugin is an ordinary Rust crate that may also ship native sources and
declare native requirements:

```text
incular-camera/
  Cargo.toml          # [package.metadata.incular.plugin] (below)
  src/lib.rs          # portable Rust API + cfg(target_os) implementations
  android/            # Kotlin sources, consumer ProGuard rules, AndroidManifest fragment
    src/main/kotlin/dev/incular/camera/CameraBridge.kt
    consumer-rules.pro
  ios/
    Sources/IncularCamera/CameraBridge.swift
```

```toml
[package.metadata.incular.plugin]
id = "dev.incular.camera"

[package.metadata.incular.plugin.android]
sources = "android"
dependencies = ["androidx.camera:camera-camera2:1.4.0", "androidx.camera:camera-lifecycle:1.4.0"]
min-sdk = 26

[package.metadata.incular.plugin.ios]
sources = "ios"
frameworks = ["AVFoundation"]
swift-packages = []      # { url, from, product }

[package.metadata.incular.plugin.requires]
permissions = ["camera"]          # the app must supply the reason (§12)
```

The CLI discovers plugins through `cargo metadata` over the app's resolved
dependency graph, so enabling a plugin is just adding the crate. Native
sources are found next to the crate's `Cargo.toml` (they are in the published
package via `include`), which keeps the Rust and native halves version-locked
without a second registry. Each plugin becomes a Gradle library module and an
SPM target in the generated projects.

Kotlin/Swift code calls back into Rust through functions the plugin itself
registers (`RegisterNatives` from the plugin's `init`; `@_cdecl` / C ABI on
iOS). Kotlin classes called from native code need ProGuard keep rules; the
plugin ships them in `consumer-rules.pro`, and the CLI fails the build if a
plugin with Android sources has none.

Plugin crates that need asynchronous platform results (permission dialogs,
pickers, activity results) use the existing `NativeRequestId` /
`NativeOperationCompletion` pattern, so results are typed and stale results
for a closed view are dropped.

### 11.3 First-party plugin set

Shipped as separate crates in phase 3, mirroring the Flutter plugins people
use first: `url_launcher` behavior (in core services), `share`, `path`
directories (core), `shared_preferences`-style key-value storage, secure
storage (Keystore / Keychain), `image_picker`, `camera`, `geolocation`,
`local_notifications`, `connectivity`, `device_info`, `package_info`,
`webview` (needs platform views), `in_app_review`. Push notifications and
in-app purchases come later and need extra store-side setup.

## 12. Permissions

### 12.1 How each platform works

**Android** has two layers:

1. **Manifest declaration.** Every permission the app might use is a
   `<uses-permission android:name="…"/>` in `AndroidManifest.xml`. Gradle's
   *manifest merger* combines the app manifest with every library's
   manifest, so libraries can add permissions the app author never wrote.
   Some permissions are capped by SDK level
   (`android:maxSdkVersion="32"`), and hardware permissions imply
   `<uses-feature>` requirements that filter devices on Google Play unless
   marked `android:required="false"`.
2. **Runtime grant.** *Normal* permissions (`INTERNET`, `VIBRATE`,
   `ACCESS_NETWORK_STATE`, `USE_BIOMETRIC`) are granted at install. *Dangerous*
   permissions (camera, microphone, location, contacts, media, notifications
   on Android 13+, nearby devices) must be requested at runtime with
   `ActivityCompat.requestPermissions` / the Activity Result API; the user can
   deny, deny permanently ("don't ask again"), grant once, or pick
   approximate location or a subset of photos. *Special* permissions (exact
   alarms, overlay, all-files access) send the user to a Settings screen.
   Google Play additionally restricts some permissions by policy (SMS, call
   log, all-files access, broad photo/video access, background location) and
   asks for a declaration in Play Console.

**iOS** has no permission list. Instead:

1. **Usage descriptions.** Every protected resource needs a purpose string in
   `Info.plist` (`NSCameraUsageDescription`, …). Accessing the resource
   without its key **terminates the app** at the first access, and App Review
   rejects vague strings. The strings are localizable (`InfoPlist.xcstrings`).
2. **Entitlements.** Capabilities tied to Apple services (push, HealthKit,
   associated domains, iCloud, NFC, App Groups, Sign in with Apple) are
   entitlements in a `.entitlements` file and must also be enabled on the
   App ID in the developer portal; the provisioning profile must include
   them.
3. **Runtime authorization.** The framework that owns the resource prompts
   (`AVCaptureDevice.requestAccess`, `CLLocationManager`,
   `PHPhotoLibrary.requestAuthorization`, `UNUserNotificationCenter`…). Each
   has its own status values (`notDetermined`, `restricted`, `denied`,
   `authorized`, `limited`, `provisional`, `ephemeral`).
4. **Privacy manifest.** `PrivacyInfo.xcprivacy` declares collected data
   types, tracking domains and *required-reason APIs* — and this applies to
   the Incular framework itself, not only the app (§13.6).
5. **App Tracking Transparency** (`NSUserTrackingUsageDescription` + the ATT
   prompt) for anything that tracks across apps.

### 12.2 How Incular declares them

One portable declaration in `Incular.toml`, keyed by capability rather than by
platform constant:

```toml
[permissions]
camera        = { reason = "@reason.camera" }
microphone    = { reason = "Record voice notes." }
location      = { access = "when-in-use", precision = "approximate", reason = "@reason.location" }
photos        = { access = "add-only", reason = "Save edited images to your library." }
notifications = {}                 # no iOS string needed; Android 13+ runtime permission
bluetooth     = { scan = "no-location", connect = true, reason = "Connect to your sensor." }

[strings.en]
"reason.camera"   = "Scan receipts with your camera."
"reason.location" = "Show stores near you."

[strings.de]
"reason.camera"   = "Belege mit der Kamera scannen."
"reason.location" = "Geschäfte in Ihrer Nähe anzeigen."
```

The CLI turns this into:

- `<uses-permission>` entries with the correct `maxSdkVersion` caps,
  `<uses-feature android:required="false">` for optional hardware, and flags
  such as `android:usesPermissionFlags="neverForLocation"` for Bluetooth scans;
- `Info.plist` usage descriptions, with `InfoPlist.xcstrings` for every
  `[strings.*]` locale;
- entitlements and privacy-manifest entries where the capability implies
  them;
- a generated Rust table (via an env var the CLI passes to `cargo`,
  read by `incular-mobile` with `option_env!`) of the declared capabilities,
  used by the runtime check below.

Validation the CLI performs before building:

- A plugin that `requires` a permission the app did not declare is an error
  that names the plugin and the line to add.
- An iOS-relevant permission without a `reason` is an error (the app would
  crash or be rejected).
- Restricted Play permissions (background location, broad media access,
  exact alarms, SMS/call log) require an explicit `play-policy = "acknowledged"`
  field so nobody adds them by accident; `incular doctor` explains the
  Play Console declaration needed.
- Raw escape hatch for anything not modelled:

  ```toml
  [android.manifest]
  uses-permission = [{ name = "com.example.CUSTOM", max-sdk = 33 }]
  [ios.info-plist]
  NSSomeNewUsageDescription = "@reason.new"
  ```

### 12.3 Runtime API

A new `permissions` module in `incular-platform`, implemented by
`incular-mobile` (desktop and web report `Unsupported` or `Granted` per
capability, never fabricated success — hard rule "Unsupported platform
behavior must be explicit"):

```rust
pub enum Permission { Camera, Microphone, Location(LocationAccess), Photos(PhotoAccess),
                      Contacts, Calendar(CalendarAccess), Notifications, Bluetooth, Motion,
                      SpeechRecognition, LocalNetwork, Tracking, /* … */ }

pub enum PermissionStatus {
    NotDetermined,          // never asked (iOS) / never asked and no rationale (Android)
    Granted,
    Limited,                // iOS limited photos/contacts, Android partial media, approximate location
    Denied { can_ask_again: bool },   // Android "don't ask again" → false; iOS denied → false
    Restricted,             // parental controls / MDM (iOS)
    Provisional,            // iOS provisional notifications
}

// Context-level API, async through the existing native-request machinery:
ctx.permissions().status(Permission::Camera) -> PermissionStatus
ctx.permissions().request(&[Permission::Camera]) -> NativeRequestId  // completes with a map of statuses
ctx.permissions().should_show_rationale(Permission::Camera) -> bool  // Android; false on iOS
ctx.permissions().open_settings()
```

- Requesting a permission that was not declared returns
  `PermissionError::NotDeclared { permission, add: "camera = { reason = … }" }`
  in every build instead of crashing (iOS) or silently failing (Android).
  The check also reads the live manifest (`PackageManager` requested
  permissions, `Bundle.main.object(forInfoDictionaryKey:)`) so overlays are
  respected.
- One request in flight per permission group; a second request joins the
  first.
- Results arrive after the activity resumes (Android) or the prompt closes
  (iOS); the runtime receives them as a lifecycle-aware completion.

### 12.4 Mapping table

`R` = runtime request needed, `—` = none. SDK caps are the generated
`maxSdkVersion` / API levels. Re-verify against Appendix B before each phase.

| Incular | Android manifest | Android runtime | iOS `Info.plist` / entitlement | Notes |
| --- | --- | --- | --- | --- |
| `camera` | `CAMERA`; `uses-feature android.hardware.camera required=false` | R | `NSCameraUsageDescription` | |
| `microphone` | `RECORD_AUDIO` | R | `NSMicrophoneUsageDescription` | |
| `location` when-in-use, precise | `ACCESS_FINE_LOCATION` + `ACCESS_COARSE_LOCATION` | R (request both; Android 12+ lets the user pick approximate) | `NSLocationWhenInUseUsageDescription` | `precision = "approximate"` declares coarse only on Android and sets `NSLocationDefaultAccuracyReduced` on iOS |
| `location` always | + `ACCESS_BACKGROUND_LOCATION` | R, separately and after foreground is granted; Android 11+ sends the user to Settings | + `NSLocationAlwaysAndWhenInUseUsageDescription`; `UIBackgroundModes: location` if continuous | Play policy declaration |
| `photos` pick only | — (Photo Picker) | — | — (`PHPicker`) | Default; use this |
| `photos` read | `READ_MEDIA_IMAGES`, `READ_MEDIA_VIDEO` (API 33+), `READ_MEDIA_VISUAL_USER_SELECTED` (API 34+), `READ_EXTERNAL_STORAGE maxSdk=32` | R | `NSPhotoLibraryUsageDescription` | Play restricts broad media access to apps whose core feature needs it |
| `photos` add-only | `WRITE_EXTERNAL_STORAGE maxSdk=28` | R on ≤ 28 only | `NSPhotoLibraryAddUsageDescription` | |
| `contacts` | `READ_CONTACTS` (+ `WRITE_CONTACTS`) | R | `NSContactsUsageDescription` | iOS 18 limited access → `Limited` |
| `calendar` full / write-only | `READ_CALENDAR`, `WRITE_CALENDAR` | R | `NSCalendarsFullAccessUsageDescription` / `NSCalendarsWriteOnlyAccessUsageDescription` | |
| `reminders` | — | — | `NSRemindersFullAccessUsageDescription` | iOS only |
| `notifications` (local) | `POST_NOTIFICATIONS` | R on API 33+ | — (runtime authorization only) | |
| `push` | + FCM dependency, `google-services.json` | — | `aps-environment` entitlement, `UIBackgroundModes: remote-notification` | Phase 4 |
| `bluetooth` | `BLUETOOTH_SCAN` (`neverForLocation` if `scan = "no-location"`), `BLUETOOTH_CONNECT`, `BLUETOOTH_ADVERTISE` (API 31+); `BLUETOOTH`, `BLUETOOTH_ADMIN maxSdk=30` (+ location ≤ 30) | R on 31+ | `NSBluetoothAlwaysUsageDescription` | |
| `nearby-wifi` | `NEARBY_WIFI_DEVICES` (API 33+) | R | — | |
| `local-network` | — | — | `NSLocalNetworkUsageDescription`, `NSBonjourServices` | iOS prompts on first LAN access; matters for devtools too (§16) |
| `motion` | `ACTIVITY_RECOGNITION` (API 29+) | R | `NSMotionUsageDescription` | |
| `speech-recognition` | `RECORD_AUDIO` | R | `NSSpeechRecognitionUsageDescription` + microphone | |
| `biometrics` | `USE_BIOMETRIC` | — | `NSFaceIDUsageDescription` | |
| `health` | Health Connect permissions per data type | R (Health Connect UI) | HealthKit entitlement, `NSHealthShareUsageDescription`, `NSHealthUpdateUsageDescription` | Phase 4 |
| `nfc` | `NFC`, `uses-feature android.hardware.nfc required=false` | — | `NFCReaderUsageDescription`, `com.apple.developer.nfc.readersession.formats` entitlement | |
| `tracking` | `com.google.android.gms.permission.AD_ID` | — | `NSUserTrackingUsageDescription` | ATT prompt on iOS |
| `internet` | `INTERNET`, `ACCESS_NETWORK_STATE` | — | — (ATS applies, §13.5) | **Always on.** Flutter adds `INTERNET` only to debug manifests and release builds silently lose network; we add it unless `internet = false`. |
| `vibrate` | `VIBRATE` | — | — | Added automatically when haptics are used |
| `foreground-service` (kind) | `FOREGROUND_SERVICE` + `FOREGROUND_SERVICE_<TYPE>` (API 34+) and `<service android:foregroundServiceType>` | — | `UIBackgroundModes` (`audio`, `location`, …) | Phase 4 |
| `background-tasks` | WorkManager dependency | — | `UIBackgroundModes: fetch, processing`, `BGTaskSchedulerPermittedIdentifiers` | Phase 4 |
| `exact-alarms` | `SCHEDULE_EXACT_ALARM` or `USE_EXACT_ALARM` | Special (Settings) | — | Play policy |

## 13. Everything else in the manifest and `Info.plist`

The CLI generates these from `Incular.toml` (§14.2). Listed so nothing is
forgotten:

### 13.1 Identity and versioning

| Incular.toml | Android | iOS |
| --- | --- | --- |
| `id = "com.example.app"` | `applicationId`, `namespace` | `CFBundleIdentifier` |
| `name` | `android:label` (`@string/app_name`, localized) | `CFBundleDisplayName`, `CFBundleName` |
| `version = "1.4.0"` (default: Cargo package version) | `versionName` | `CFBundleShortVersionString` |
| `build = 42` (or `--build-number`) | `versionCode` (must increase on every upload) | `CFBundleVersion` |
| `icon = "assets/icon.png"` (1024², plus optional `adaptive-foreground`/`background`/`monochrome`) | mipmap set + adaptive icon + themed (monochrome) icon | `AppIcon` asset catalog (single 1024² is accepted by current Xcode; dark and tinted variants optional) |
| `splash` (background color, centered image) | Android 12 `SplashScreen` theme (`Theme.SplashScreen`), `postSplashScreenTheme` | `UILaunchScreen` dictionary (required; no storyboard needed) |
| `orientations` | `android:screenOrientation` | `UISupportedInterfaceOrientations` (+ `~ipad`) |
| `locales = ["en", "de"]` | `android:localeConfig` (per-app language, API 33+) + `resConfigs` | `CFBundleLocalizations`, `CFBundleDevelopmentRegion` |

### 13.2 Always generated (Android)

- `<activity android:name="dev.incular.IncularActivity" android:exported="true">`
  with the `MAIN`/`LAUNCHER` intent filter (`exported` is mandatory on API 31+
  for components with intent filters);
- `android:configChanges` (§4.3), `android:windowSoftInputMode="adjustResize"`,
  `android:hardwareAccelerated="true"`, `android:launchMode="singleTop"`;
- `android:enableOnBackInvokedCallback="true"` (predictive back);
- launch theme and normal theme, edge-to-edge;
- `<meta-data>` for engine settings (renderer preference, initial route);
- 16 KB page-size compatible native libraries (§15.2), stored uncompressed
  (`useLegacyPackaging = false`).

### 13.3 Always generated (iOS)

- `UIApplicationSceneManifest` with `IncularSceneDelegate`,
  `UIApplicationSupportsMultipleScenes` (true on iPad when enabled);
- `UILaunchScreen`, `LSRequiresIPhoneOS`, `UIRequiredDeviceCapabilities =
  [arm64, metal]`;
- `CADisableMinimumFrameDurationOnPhone = true`;
- `UIApplicationSupportsIndirectInputEvents = true`;
- `UIViewControllerBasedStatusBarAppearance = true`;
- `ITSAppUsesNonExemptEncryption` from `Incular.toml` (`false` default:
  HTTPS-only apps are exempt; set `true` when the app ships its own crypto),
  which avoids the export-compliance question on every upload.

### 13.4 Deep links and app links

```toml
[links]
schemes = ["myapp"]                                  # custom URL schemes
domains = ["example.com", "*.example.com"]           # verified https links
```

- Android: an `<intent-filter android:autoVerify="true">` with `VIEW`,
  `BROWSABLE`, `DEFAULT` per domain; custom schemes as separate filters. The
  site must host `/.well-known/assetlinks.json` with the signing certificate
  SHA-256 — `incular doctor --links` prints the exact file for the debug and
  release keys.
- iOS: `com.apple.developer.associated-domains` entitlement
  (`applinks:example.com`, and `webcredentials:` for password autofill) plus
  `CFBundleURLTypes` for schemes; the site hosts
  `/.well-known/apple-app-site-association`, also printed by `doctor`.
- Incoming links arrive as the existing `UrlActivation` and feed the
  navigation crate's router.

### 13.5 Querying and opening other apps, network security

- Android 11+ package visibility: opening `https:` / `mailto:` / `tel:`
  through `canLaunch`-style checks needs `<queries>` intents. The CLI adds
  them for the schemes the app's code declares in `[links.open]`.
- iOS: `canOpenURL` for custom schemes needs `LSApplicationQueriesSchemes`
  (same `[links.open]` list; max 50).
- Cleartext HTTP is blocked by default on both (Android
  `usesCleartextTraffic=false` since API 28, iOS ATS). `[network]
  cleartext-domains = ["10.0.2.2"]` generates a
  `network_security_config.xml` and `NSAppTransportSecurity` exceptions.
  Debug builds allow cleartext to `localhost`, `10.0.2.2` and the dev
  machine for devtools and hot reload.

### 13.6 Privacy manifest and store privacy forms

- Incular itself calls required-reason APIs: `std::time::Instant` on Apple
  platforms reads `mach_absolute_time` (system boot time category, reason
  `35F9.1`), and `std::fs::metadata` reads file timestamps (`C617.1` for
  files inside the app container). `incular-ios` therefore ships its own
  `PrivacyInfo.xcprivacy` in the Swift package, and every first-party plugin
  ships one for what it uses. The CLI merges them with the app's
  `[privacy]` section (collected data types, tracking, tracking domains) into
  the app's manifest. Third-party Rust crates that call such APIs (for
  example `UserDefaults` or disk-space queries) are the app's
  responsibility; `incular doctor` scans the linked static library's
  imported symbols against Apple's required-reason list and warns.
- Google Play's Data safety form is filled in Play Console, not in the
  manifest. `incular build --privacy-report` prints what Incular and the
  plugins collect, to copy from.

### 13.7 Background execution, alarms, services

Background modes, foreground services, WorkManager and `BGTaskScheduler`
are phase 4. They need an engine that runs Rust without a view (headless
engine) and a policy for how much of the runtime starts. The `Incular.toml`
keys are reserved (`[background]`) so the schema does not change later.

## 14. Project layout and configuration

### 14.1 App layout

```text
my_app/
  Cargo.toml              # one [[bin]]; depends on incular with "mobile" for mobile targets
  Incular.toml            # app-level native configuration (this section)
  src/
    main.rs               # fn main() { incular::run(app) } on every platform (§3)
  assets/
  platform/               # optional overlays; empty in most apps
    android/
      AndroidManifest.xml # merged by Gradle's manifest merger (tools:node etc. work)
      res/                # extra Android resources (e.g. notification icons)
      build.gradle.kts    # appended into the app module (signing, flavors, extra deps)
      src/main/kotlin/    # extra Kotlin sources (custom IncularActivity subclass)
    ios/
      Info.plist          # merged key-by-key over the generated plist (overlay wins)
      App.entitlements    # merged over generated entitlements
      Sources/            # extra Swift sources
  target/incular/android  # GENERATED Gradle project (git-ignored)
  target/incular/ios      # GENERATED Xcode project (git-ignored)
```

### 14.2 `Incular.toml` schema (initial)

```toml
[app]
id = "com.example.notes"
name = "Notes"
version = "1.0.0"              # optional; defaults to Cargo package version
icon = "assets/icon.png"
orientations = ["portrait", "landscape"]
locales = ["en", "de"]

[app.splash]
background = "#101418"
image = "assets/splash.png"

[android]
min-sdk = 26
target-sdk = 36                # CLI default tracks Appendix B
abis = ["arm64-v8a", "x86_64"] # armeabi-v7a opt-in
renderer = "auto"              # auto | vulkan | gles
surface = "surface-view"       # surface-view | texture-view
signing = { keystore = "$INCULAR_ANDROID_KEYSTORE", alias = "upload" }  # values from env, never in git

[ios]
deployment-target = "16.0"
team = "$INCULAR_APPLE_TEAM_ID"
devices = ["iphone", "ipad"]
multiple-scenes = false
encryption = false             # ITSAppUsesNonExemptEncryption

[permissions]                   # §12.2
[links]                         # §13.4
[network]                       # §13.5
[privacy]                       # §13.6
[assets]
include = ["assets/**"]
exclude = ["assets/**/*.psd"]

[strings.en]                    # localizable strings used by the sections above
```

The schema is versioned (`incular-toml = 1` implied) and validated with
precise, span-carrying errors — application-author mistakes are structured
diagnostics (CONTRIBUTING.md error policy). JSON Schema is published for
editor completion.

### 14.3 Why generated projects plus overlays

Flutter commits `android/` and `ios/` and lets users edit them; every Flutter
upgrade then becomes a manual migration of those projects. Generating them on
every build from `Incular.toml` makes upgrades free and keeps one source of
truth, while overlays cover what the schema does not (Gradle's manifest
merger and a plist merge are both well-defined). `incular eject` copies the
generated projects into `platform/` for teams that want full control and
accept owning upgrades.

## 15. Tooling: the `incular` CLI

### 15.1 Commands

| Command | Does |
| --- | --- |
| `incular create <name>` | New app with `lib.rs`, `main.rs`, `Incular.toml`, desktop and mobile features |
| `incular doctor [--links]` | Checks Rust targets, Android SDK/NDK/JDK, Xcode and command-line tools, simulators, signing, `Incular.toml`, plugin requirements, required-reason API scan; prints fixes |
| `incular devices` | Lists `adb` devices/emulators and `xcrun devicectl`/`simctl` devices |
| `incular run android\|ios [--device] [--release]` | Builds, installs, launches, streams logs, connects devtools |
| `incular build apk\|aab\|ios\|ipa\|xcframework` | Release artifacts; `aab` and `ipa` are what the stores take |
| `incular logs` | logcat filtered to the app / `os_log` for the bundle id |
| `incular eject` | Copies generated native projects into `platform/` (§14.3) |

`cargo` stays the build system for Rust; the CLI calls it with the right
target, linker and crate type and never replaces it. `cargo check
--target aarch64-linux-android` keeps working without the CLI (useful in CI
and for the cross-platform check recipe).

### 15.2 Android build pipeline

1. `cargo rustc --lib --crate-type cdylib --target <abi>` per ABI with the
   NDK clang linker (`aarch64-linux-android<min-sdk>-clang`), linked with
   `-Wl,-z,max-page-size=16384` (Google Play requires 16 KB page-size
   compatibility; NDK r28+ does this by default, the flag makes it explicit).
2. Strip debug info into a separate symbols archive for Play's native crash
   symbolication (`ndk.debugSymbolLevel`).
3. Copy `libincular_app.so` per ABI into `jniLibs/`, assets into `assets/`.
4. Generate the Gradle project: settings, app module, one module per plugin,
   the `incular-embedding` library module from
   `crates/incular-android/android/`.
5. `./gradlew assembleDebug|bundleRelease` with the pinned Gradle wrapper,
   AGP and Kotlin versions from the CLI template. R8 is on for release.

Targets: `aarch64-linux-android` (devices), `x86_64-linux-android`
(emulators), `armv7-linux-androideabi` opt-in for old 32-bit devices.

### 15.3 iOS build pipeline

1. `cargo rustc --lib --crate-type staticlib` for `aarch64-apple-ios`
   (devices) and `aarch64-apple-ios-sim` (Apple-silicon simulators); Intel
   simulators are not supported.
2. Wrap the libraries in `IncularApp.xcframework`.
3. Generate an Xcode project (from a template via a small project-file
   writer; no XcodeGen/Tuist dependency) that links the xcframework, the
   `IncularKit` Swift package from `crates/incular-ios/apple/`, plugin Swift
   packages, the merged `Info.plist`, entitlements, asset catalog,
   `PrivacyInfo.xcprivacy` and `InfoPlist.xcstrings`.
4. `xcodebuild` for build/archive/export; `xcrun devicectl` and `simctl` to
   install and launch.
5. Signing: automatic signing with the configured team by default; manual
   profiles via overlay. Credentials never enter `Incular.toml`.

**iOS builds require macOS with Xcode.** From Windows or Linux, developers
can `cargo check --target aarch64-apple-ios` (with the stub-toolchain recipe)
and use CI or a remote Mac for the rest. `incular doctor` says this plainly
instead of failing late.

### 15.4 Minimum versions and why

- **Android API 26 (8.0).** Covers well over 95 % of active devices; gives
  `ANativeWindow`/`AHardwareBuffer` basics, adaptive icons, AAudio and the
  font APIs we need; avoids pre-Oreo background and notification-channel
  special cases. Vulkan is used only where the device passes WGPU's checks.
- **iOS 16.** Covers the overwhelming majority of active devices in late
  2026; gives `UIEditMenuInteraction`, `keyboardLayoutGuide`, `UIScene`
  maturity and current Metal feature sets, and lets us skip the deprecated
  `UIMenuController` path.
- Raised on the same schedule as the Rust MSRV: when a newer API makes the
  code meaningfully better, with a CHANGELOG entry.

## 16. Platform views, external textures, devtools and hot reload

### 16.1 Platform views

The platform crate already defines the commands and the honest compositing
statement. On mobile:

- **Phase 3: `AboveSurface`.** Android adds the native `View` to
  `IncularView`'s `FrameLayout` above the `SurfaceView`; iOS adds the
  `UIView` as a sibling above the Metal layer. Touches inside the view's
  bounds go to the native view unless an Incular gesture claims them (the
  existing `PlatformViewPointerEvent` forwarding).
- **Later: composited platform views.** Two known techniques:
  Android *texture layer* (render the view into a `SurfaceTexture` /
  `AHardwareBuffer` and import it into WGPU as an external texture) and the
  iOS/Android *overlay split* (extra Metal/Surface layers above each native
  view for Incular content painted over it). Both need the external texture
  support below and are scoped in their own spec when a real use case
  (web view, maps) needs them.

### 16.2 External textures

Video, camera preview and some plugins produce frames on the GPU or in shared
memory. A new `ExternalTextureId` contract in `incular-rendering`, executed by
`incular-wgpu` (imported `AHardwareBuffer` on Android through Vulkan's
external memory extension; `CVPixelBuffer`/`IOSurface` → `MTLTexture` on iOS),
with a widget that paints it. This touches the vendored WGPU and is phase 3.

### 16.3 Devtools and hot reload

- Devtools transport is the existing WebSocket; `incular run` sets up
  `adb reverse tcp:<port> tcp:<port>` for Android and uses localhost for the
  iOS simulator. Physical iOS devices connect over the LAN, which triggers
  the local-network prompt: debug builds add `NSLocalNetworkUsageDescription`
  and the devtools Bonjour type automatically, release builds never do.
- Hot reload: `dx serve --hot-patch` supports Android and the iOS simulator;
  evaluate in phase 2 with the recipe in the hot-reload memory notes. No dx
  fork (existing decision).

## 17. Testing and CI

- **Host-side tests** (`crates/incular-mobile/tests/`): lifecycle state
  machine, frame pump, text-input protocol (UTF-16 conversion, version
  rejection, batch edits), inset animation, permission broker, plugin
  metadata parsing, `Incular.toml` validation, manifest / plist / privacy
  manifest generation (golden files). These run on every developer machine.
- **Build checks** in `.github/workflows/build.yml`: `cargo clippy` for
  `aarch64-linux-android` and `aarch64-apple-ios` like the existing wasm32
  clippy job.
- **Android device tests:** emulator job (Linux runner with KVM) running an
  instrumented test APK that drives an example through `adb`: launch, tap,
  type with the emulator IME, rotate, background/foreground, process death
  and restoration, TalkBack node tree dump via `uiautomator`.
- **iOS device tests:** macOS runner, `xcodebuild test` with an XCUITest
  target that does the same through the accessibility tree (which also
  proves VoiceOver exposure).
- **Parity:** the Flutter harness gains mobile runs: the Flutter reference
  app and the Incular example on the same emulator/simulator, screenshots via
  `adb exec-out screencap` / `simctl io screenshot`, compared with the
  existing pixel tooling.
- **Performance:** frame-time and allocation benchmarks on a documented
  low-end Android device and one iPhone, per the performance rules.
- Real-device checks per phase (TalkBack, VoiceOver, IMEs: Gboard, Samsung,
  SwiftKey, a CJK IME, iOS Japanese/Chinese keyboards) are listed in the
  phase exit criteria; anything not run is reported, not assumed.

## 18. Phases

Each phase ends with the validation in AGENTS.md, examples running on real
devices, CHANGELOG entries, and the contract docs updated.

| Phase | Scope | Exit criteria |
| --- | --- | --- |
| **0. Host extraction** | Extract `incular-host` from `incular-desktop`; desktop and web unchanged in behavior. Add `Android`/`UiKit` to `NativeWindowSystem`, architecture.json edges, hard-rule rewrite. | Full suite green; desktop examples and web smoke unchanged; frame benchmarks unchanged. |
| **1. Pixels and touches** | `incular-mobile` skeleton; Kotlin and Swift shells; JNI/C ABI; surfaces; Vulkan/GLES/Metal; Choreographer/`CADisplayLink`; touch input; lifecycle and surface loss; logging and panic containment; asset bundles; system fonts; insets (no keyboard yet); `incular` CLI `create`/`doctor`/`run`/`build apk`/`build ios` with generated projects and `Incular.toml` (`[app]`, `[assets]`). | `counter`, `scroll` and `animation` examples run on an Android phone, an Android emulator, an iPhone and the iOS simulator at the display refresh rate; rotate/background/foreground survive; no allocations in steady-state frames. |
| **2. Text and accessibility** | Text-input protocol both ways, IME, autofill, keyboard insets with animation, selection UI, hardware keyboards, TalkBack and VoiceOver bridges, predictive back, system UI style, orientation, haptics, clipboard, share, URL launch, restoration through process death, pipeline cache, hot-reload evaluation. | `text_field`, `semantics_gallery`, `navigation_showcase` pass device checks with the IMEs and screen readers listed in §17; CI emulator and simulator jobs green. |
| **3. Native integration** | Permissions (declaration, generation, runtime API), `[links]`, `[network]`, `[privacy]`, privacy manifest, plugin crates and discovery, first-party plugins, `AboveSurface` platform views, external textures, `build aab`/`ipa`, release signing, display features. | A sample app with camera, location and notifications builds release artifacts that pass Play's pre-launch report and App Store Connect validation (TestFlight upload); `incular doctor` catches every misconfiguration in its test matrix. |
| **4. Background and services** | Headless engine, push (FCM/APNs), background tasks, foreground services, in-app purchase plugin, add-to-app guides and engine caching. | Add-to-app sample in an existing Kotlin and SwiftUI app; background task and push samples on devices. |

## 19. Open questions

1. **Kotlin/Swift shell distribution.** Vendoring the shell sources from the
   crate directory (§11.2) keeps versions locked and avoids Maven Central and
   Swift registry publishing. Is there a need for prebuilt AARs (for teams
   that add Incular to an existing app without the CLI)? Proposal: publish
   AAR/XCFramework artifacts only for add-to-app, in phase 4.
2. **Material vs Cupertino.** Mobile users expect iOS-style controls on iOS.
   Incular has `incular-material` and neutral controls; an iOS look (and the
   platform-adaptive constructors Flutter has) is a separate design question,
   not a host question.
3. **Native text rendering.** Incular shapes its own text. Matching iOS's
   text rendering (SF optical sizes, tracking tables) closely is a parity
   question for `incular-text` that becomes visible on iPhones.
4. **Vulkan vs GLES default.** Start with Vulkan where WGPU accepts the
   device, measure jank and memory on low-end devices, and keep the denylist
   honest.
5. **Should `Incular.toml` merge into `[package.metadata.incular]`?** One
   file is simpler for small apps; a separate file keeps `Cargo.toml` small
   and lets the schema have its own version. Proposal: separate file for
   apps, package metadata for plugins (it has to travel with the crate).

## Appendix A: what Flutter does, for reference

| Area | Flutter | Incular choice |
| --- | --- | --- |
| Embedding | Java/Kotlin `FlutterActivity`/`FlutterView`, ObjC `FlutterViewController`, C++ engine | Same shape: Kotlin/Swift shells, Rust engine |
| Threads | Platform + UI merged (3.29+), raster thread, IO thread | Main thread for everything; Tokio for async |
| Rendering | Impeller (Vulkan/GLES on Android, Metal on iOS) | WGPU (Vulkan/GLES, Metal) |
| Native calls | Method channels, Pigeon, FFI | Direct Rust calls (JNI/objc2) and plugin-owned Kotlin/Swift |
| Native projects | Committed `android/` and `ios/`, user-edited | Generated from `Incular.toml`, overlays, `eject` |
| Permissions | App edits `AndroidManifest.xml` and `Info.plist` by hand; plugins merge Android manifests; `permission_handler` plugin at runtime | One portable declaration generating both, runtime API in core, undeclared requests are typed errors |
| iOS dependencies | CocoaPods, moving to SPM | SPM only |

## Appendix B: platform facts to re-verify

Checked against knowledge as of 2026; confirm before each phase and update
CLI defaults in one place.

| Fact | Value used here | Source to check |
| --- | --- | --- |
| Google Play target SDK requirement | New apps and updates must target API 36 since Aug 2026; next bump expected Aug 2027 | Play Console "Target API level requirements" |
| Edge-to-edge | Enforced for target 35, opt-out removed for target 36 | Android 15/16 behavior changes |
| Large-screen orientation/resizability locks ignored | Target 36, smallest width ≥ 600 dp | Android 16 behavior changes |
| Predictive back | System animations on by default for target 36; `onBackPressed` not called | Android 16 behavior changes |
| 16 KB page size | Required on Play for apps targeting Android 15+ since Nov 2025; NDK r28 aligns by default | Android developers "Support 16 KB page sizes" |
| App Store SDK requirement | Xcode 26 / iOS 26 SDK for uploads since April 2026 | Apple "Upcoming requirements" |
| `UIScene` lifecycle | Required for apps built with the SDK after iOS 26 | WWDC25 / UIKit release notes |
| CocoaPods | Trunk becomes read-only Dec 2026 | CocoaPods blog |
| Required-reason API list | `mach_absolute_time` (35F9.1), file timestamps (C617.1), disk space, active keyboards, `UserDefaults` | Apple "Describing use of required reason API" |
| Android 13 / 14 media permissions | `READ_MEDIA_*`, `READ_MEDIA_VISUAL_USER_SELECTED` | Android "Granular media permissions" |
| Play photo/video permission policy | Broad access only for core-use apps; others use the Photo Picker | Play policy "Photo and Video Permissions" |
