# support: rendering and services behavior observable through widgets

Revalidated at HEAD `a19bf487`.

## Summary

130 inventory rows from Flutter `rendering/` and `services/` were classified against HEAD. Final status counts:

- `out-of-scope`: 65. Debug helpers, platform channel and codec plumbing, native platform view controllers, embedder chrome, haptics and sounds, build metadata, and abstract base classes whose behavior is covered by concrete rows. Each has its reason in the coverage file.
- `partial-reading`: 48. Counterpart located with `rg -n` and cited by path and line; only the named aspects were compared by reading. Most are reassigned to layout, paint, gestures, slivers, misc or navigation.
- `missing`: 17. No Incular counterpart after a search. Covered by five findings: support-001 (platform views), support-002 (predictive back), support-003 (platform undo manager), support-004 (spell check), support-005 (network and platform asset bundles).

No frame scenarios were added. The package's observable behaviors are either already covered by other packages' scenarios (paint-004 for PhysicalShape, misc-002 for RotatedBox) or absent APIs with nothing to compare at frame level. `harness/incular/tests/frames/support.rs` and `harness/flutter/test/frames/support_test.dart` remain empty placeholders.

## Coverage

| Area | Result |
| --- | --- |
| `rendering/binding.dart`, `object.dart`, `view.dart`, `box.dart` | Frame entry, pipeline phase enum and root layout located (`incular-runtime/src/frame.rs:2203`, `incular-widgets/src/recursion.rs:25`, `tree/layout.rs:96`). Phase order not verified against Flutter. Layout protocol reassigned to layout. |
| `rendering/layer.dart`, `proxy_box.dart` layers | Leader and follower paths (`tree.rs:869-872`, `tests/linked_layers.rs`) and `compositing.rs:291` for annotations. Platform view layer missing (support-001). |
| `rendering/proxy_box.dart`, `rotated_box.dart`, `list_body.dart` | LimitedBox defaults read (`sizing_constraints.rs:219`). RotatedBox relies on misc-002. ListBody located, not compared. |
| `rendering/platform_view.dart` | Missing (support-001). |
| `rendering/debug.dart`, `error.dart`, `mouse_tracker.dart`, `service_extensions.dart` | Debug-only or partial: RenderErrorBox colours at `utilities.rs:88`, hover refresh at `tree/raw_input.rs:121`. |
| `services/mouse_cursor.dart`, `mouse_tracking.dart` | Cursor mapping read (`incular-desktop/src/pointer.rs:130-151`); MouseRegion analogue at `incular-widgets/src/lib.rs:302`. Reassigned to gestures. |
| `services/asset_bundle.dart`, `asset_manifest.dart` | Trait and caching bundle located (`incular-assets/src/bundle.rs:40, 177`), `DefaultAssetBundle::of` (`incular-widgets/src/asset_bundle.rs:33`). Network bundle missing (support-005). Manifest out of scope (compiled bundles). |
| `services/restoration.dart`, `sensitive_content.dart` | `RestorationScope` (`incular-core/src/restoration.rs:128`) and `SensitiveContentHost` (`incular-widgets/src/environment.rs:517`). Reassigned to navigation for restoration. |
| `services/predictive_back_event.dart` | Missing (support-002). |
| `services/undo_manager.dart` | Missing (support-003). Reassigned to text. |
| `services/spell_check.dart` | Missing (support-004). Reassigned to text. |
| `services/system_chrome.dart`, `system_sound.dart`, `haptic_feedback.dart`, `scribe.dart` | Out of scope: embedder chrome, sounds, haptics and handwriting with no widget-visible state in Incular's hosts. `raw_tooltip.rs:25` keeps only a feedback policy flag. |
| `services/message_codec*.dart`, `platform_channel.dart`, `binary_messenger.dart`, `system_channels.dart` | Out of scope: platform channel plumbing. |
| `services/platform_views.dart` | Out of scope: native controllers. Absence recorded in support-001. |
| `services/binding.dart`, `flutter_version.dart`, `flavor.dart`, `deferred_component.dart`, `debug.dart` | Partial (ServicesBinding lifecycle at `frame.rs:607-611`) or out of scope (build and debug metadata). |

## Findings

### support-001 — Platform views cannot be embedded in the widget tree

- Severity P2, category behavior, impact class missing-feature.
- Flutter: `rendering/platform_view.dart` (AndroidView, UiKitView, AppKitView render objects), `rendering/layer.dart:1005`, `services/platform_views.dart:72`.
- Incular: MISSING. `rg -n -i 'platform_view|PlatformView|AndroidView|UiKitView' crates` returned no hits.
- Evidence: source only. Flutter participates native views in layout, clipping and hit testing. Incular has no type for this.
- Impact: native map, web view or camera preview cannot sit inside an Incular layout.
- Cause: the renderer and widget tree model only Incular-drawn content.
- Fix direction: decide whether native embedding belongs in the native host crates with a placeholder layer in incular-rendering. It is platform-specific, so it does not belong in a renderer-neutral crate.
- Confidence high. Limitations: a product decision is needed; no scenario is possible.

### support-002 — Predictive back progress is not delivered to routes

- Severity P2, category behavior, impact class missing-feature.
- Flutter: `services/predictive_back_event.dart:13, 26`.
- Incular: MISSING. `rg -n -i 'predictive|back_event|BackEvent' crates` found only a doc comment at `crates/incular-material/src/page_transitions.rs:28`.
- Evidence: source only. Flutter delivers back progress, edge and offset; Incular has no equivalent channel.
- Impact: on Android with predictive back, routes cannot follow the back gesture progressively.
- Cause: the Android host does not forward the event and navigation has no progress channel.
- Fix direction: add a platform-neutral back-progress event and map Android callbacks in incular-android.
- Confidence medium. Limitations: the Android host was searched only by these names; no device check.

### support-003 — Text fields have no platform undo manager integration

- Severity P2, category behavior, impact class missing-feature. Reassigned to text.
- Flutter: `services/undo_manager.dart:10, 37, 112`.
- Incular: MISSING. The only undo found is application-level (`crates/incular-runtime/src/lib.rs`, `crates/incular-runtime/tests/undo.rs`).
- Impact: system undo and redo may not act on text-field edits.
- Fix direction: text package exposes an undo client from its editing session and the native hosts forward platform undo commands.
- Confidence medium. Limitations: the native host forwarding path was not checked.

### support-004 — Spell check service is not integrated with text fields

- Severity P2, category behavior, impact class missing-feature. Reassigned to text.
- Flutter: `services/spell_check.dart:27, 65, 101, 124`; widget side `widgets/spell_check.dart` is owned by the text package.
- Incular: MISSING. `rg -n 'SpellCheck|spell' crates` found no service type; the matches were incidental.
- Impact: no spelling underlines or suggestions from the framework.
- Fix direction: text package adds a platform-neutral spell result and suggestion span; the host supplies native checking.
- Confidence medium. Limitations: the authoritative widget-side row is in `coverage/text.jsonl`.

### support-005 — NetworkAssetBundle has no counterpart

Orchestrator note: narrowed after verification. Incular provides a `rootBundle` equivalent at `crates/incular-widgets/src/asset_bundle.rs:40-41`, so only the network bundle is missing.

- Severity P3, category behavior, impact class missing-feature.
- Flutter: `services/asset_bundle.dart:146` (NetworkAssetBundle), `:324` (PlatformAssetBundle).
- Incular: MISSING. Bundles are `MemoryAssetBundle` and `DirectoryAssetBundle` (`crates/incular-assets/src/bundle.rs:95, 140`).
- Impact: assets cannot be loaded from a URL through the bundle API. A custom `AssetBundle` implementation is a partial workaround.
- Fix direction: add a network bundle if network assets are wanted. A platform manifest bundle is not needed because bundles are compiled in.
- Confidence medium. Limitations: no runtime divergence was observed.

## Scenarios

None added. Reasons: the missing items are absent APIs with nothing to compare at frame level; the in-scope partial rows are covered by other packages (paint-004 and misc-002 already have scenarios or source findings).

## Open questions

- Whether RenderLimitedBox's clamping of negative limits (`sizing_constraints.rs:219-250`) should match Flutter's assertion. This is a debug-assert difference only.
- Whether the PipelineOwner flush order (layout, compositing bits, paint, semantics) matches Incular's frame sequence. Not verified by a scenario.
- Whether RenderView's view configuration fields (size, devicePixelRatio) reach the root constraints identically to Flutter's. Not compared.

## Pixel questions

None.

## Harness requests

None.

## Validation run

- `pwsh -NoProfile -File audits/flutter-parity/inventory_tool/render-inventory.ps1`: passed. `support: total 130, unchecked 0`.
- Coverage rows were rewritten by a scratch merge script that maps each row to its status, counterpart and findings; rows keep their original file, line and name.
- No Rust or Dart files were changed, so no cargo or flutter checks were required. `harness/run-frames.sh support__` was not run because no scenarios exist.
