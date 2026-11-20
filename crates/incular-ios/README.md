# incular-ios

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | iOS embedder shell: UIKit scene, Metal view, touch, safe areas, lifecycle and semantics adapter. |
| API class | backend; re-exports and path overrides follow the [architecture contract](https://github.com/FabortLabs/incular/blob/master/system-design/ARCHITECTURE.md). |
| Support | Partial; surfaces, touch, safe areas and lifecycle run on iOS 16+. Text input, the accessibility bridge and native services follow the mobile spec phases. |

iOS platform integration for Incular.

- `incular::run` calls `run_application`, which starts `UIApplicationMain`
  with Rust-defined `IncularAppDelegate` and `IncularSceneDelegate` classes.
  The bundle's `Info.plist` must name `IncularSceneDelegate` in its scene
  manifest; the `incular` CLI generates one.
- `IncularView` is backed by a `CAMetalLayer`, drives an
  `incular_mobile::MobileEngine` from `CADisplayLink`, and forwards touches,
  safe-area insets and scene lifecycle.
- `IosAccessibilityAdapter` exposes the incremental mobile semantics
  update/action contract for a UIKit or SwiftUI host.

See `system-design/MOBILE.md` for the full design.
