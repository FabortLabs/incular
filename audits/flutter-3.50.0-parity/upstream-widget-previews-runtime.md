# Flutter 3.50 widget preview composition

Pinned `lib/src/widget_previews/widget_previews.dart` and the authored
`preview_slice_test.dart` establish default group, newest wrapper outside
earlier wrappers, first theme outermost, immutable builder snapshots,
wrapper clearing, and label/size/text scale/brightness metadata. The
Rust target `flutter350_preview_composition` checks actual retained paint
order and underlying leaf preservation as well as the metadata.
The facade is `incular::widget_previews` and depends on no design system.

This closes the authored wrapper/theme API gap recorded in
[listener-tween-preview-comparisons.md](listener-tween-preview-comparisons.md).
This is only runtime composition. Annotation discovery, generator, scaffold,
host/rendering, file watching/reload, and DTD/IDE integration remain outside
this slice. These authored cases do not count as any of the 113 upstream
tooling registrations.

## Localized preview subtrees

`PreviewLocalizationsData` and `Preview::apply_localizations` install
catalog resources and resolved directionality in the retained preview.
Pinned `preview_localizations_reference_test.dart` exercises the actual
Flutter preview descriptor and the resolver used by its scaffold. Both
sides propagate English, Spanish, and Arabic resources; change inherited
direction without remounting the child; and invoke the list override
before the single-locale override when an explicit locale is supplied.
The Rust environment and preview targets passed 11 tests; the authored
Flutter probe passed both comparisons.

Incular catalogs are not Flutter delegate lists. This probe does not
execute the actual `widget_preview_scaffold/test/localizations_test.dart:50`
registration: the offline SDK cache lacks that tool's design-system and
launcher dependencies. It is not credited as a paired registration.

## Preview scaffold manifest

Three exact hermetic tool registrations passed in the pinned Flutter SDK:
valid manifest fields, initial/stale SDK scaffold regeneration, and root
pubspec changes. Flutter's manifest records manifest and Dart SDK versions
and project pubspec hashes. The native `incular_devtools::preview_manifest`
now pairs those three behaviors using Cargo manifests. It records the
schema, SDK identity, and dependency fingerprints, requests full
regeneration for a missing or SDK-incompatible scaffold, and refreshes
only metadata after a project edit. It uses parsed TOML and a stable
cache fingerprint rather than the Dart pubspec schema or MD5 bytes.

The five native tests also cover comment-only edits, removed members,
corrupt metadata, invalid project TOML, and preserving host fields on
atomic refresh. This component does not generate or launch the preview
host, discover annotations, or implement DTD; those cases remain open.

To repeat against the SDK's initialized `flutter_tools` package:

```bash
bash audits/flutter-3.50.0-parity/run-reference-linux.sh \
  tooling preview_manifest/preview_manifest_test.dart \
  'PreviewManifest generates a valid manifest' \
  'PreviewManifest identifies widget preview scaffold project needs to be generated' \
  'PreviewManifest identifies root project pubspec has changed'
```

### Default locale selection

Pinned `packages/flutter/test/widgets/app_test.dart:708` passed its six
original preference cases. The paired Rust helper compares the same
inputs and results; `preview_localizations_reference_test.dart` and Rust
also check seven adversarial preferences, including first-supported
ties, promotion of a later same-language exact match, and a region-only
match that actually outranks an unrelated first-supported locale.

Previews and `WidgetsApp` now use the Flutter selection order, rather
than ICU's resource fallback. The app test checks the locale reaches
the generated title. ICU remains the source of text direction,
formatting, and catalog message fallback. BCP-47 extensions remain
available to Rust consumers but do not alter the three-component match.

### Simulated media and actual layout

`Preview::compose` applies the wrapper, theme, optional size, scoped
MediaQuery and localizations in the scaffold's inner-to-outer order.
The scope starts with the nearest MediaQuery, or the retained native
snapshot when the preview is mounted directly. It only replaces the
requested fields. A positive-infinite axis leaves layout sizing to
the child without replacing the simulated MediaQuery extent.

The pinned neutral `preview_media_reference_test.dart` and retained Rust
`flutter350_preview_composition` verify that an unconstrained-width,
60-high preview renders a 40-by-60 leaf, both theme and wrapper observe
the simulated brightness/scale/size, padding changes are inherited
without remounting, and an outer MediaQuery's independent fields are
still visible. The authored probe does not exercise scaffold controls,
including the user-selected brightness override, zoom, or restart.

## Watch event scope and invalidation

Pinned hermetic `preview_detector_regression_178317_test.dart:76`
passed. The real Flutter detector checks generated paths before
manifest handling and drops source events without an analysis
context. Incular's preview manifest now creates a matching native
event router for its workspace. It calls neither invalidation path
for the generated scaffold, Cargo target directories, files outside
registered source contexts, or unrelated files. Source and manifest
changes have distinct actions, so an actual dependency edit can
refresh metadata without replacing the whole scaffold.

Eight focused native checks pass. The new cases create and remove
temporary project files, assert no callbacks for the excluded paths,
and exercise an actual dependency metadata refresh on the manifest
callback. Paths are matched by components so unrelated names cannot
be mistaken for a configured exclusion. The opt-in `preview-watch` feature now also tests real Linux watcher
notifications. Writes inside the scaffold and Cargo target, and source
files without a registered context, do not invalidate previews. A
valid nested source does reload and publish its exact on-disk contents.
The original Flutter case injects its watcher and now has a full
behavioral pair for the tested callback scope. Separate native tests
verify deletion, atomic manifest replacement, and rescanning after a
missing source directory appears. Watch plans exclude generated
subtrees, the queue is bounded, and overruns request a rescan instead
of presenting missing events as a complete stream. The IDE analysis
transport and other OS backends are not verified by these tests.

```sh
cargo test --locked -j 1 -p incular-devtools --features preview-watch --test flutter350_preview_changes --test flutter350_preview_watcher --test flutter350_preview_reload -- --test-threads=1
bash audits/flutter-3.50.0-parity/run-reference-linux.sh tooling preview_detector/preview_detector_regression_178317_test.dart 'PreviewDetector regression test https://github.com/flutter/flutter/issues/178317'
```

## Reload completion, retries, and timing

Six pinned hermetic `lsp_preview_detector_test.dart` registrations
cover source reloads, manifest-only updates, ignored files, ignored
directories, lookup failure, and analysis failure. Incular now routes
the same cases through `PreviewReloadCoordinator`. Source changes
wait for analysis, load, and publish before returning elapsed time.
In the native fixture, two actual file writes publish two different
snapshots; dependency edits update the on-disk manifest without
running analysis; ignored paths touch neither backend.

The lookup budget matches Flutter: five attempts, a 200 ms delay
before retrying, and a five-second deadline per attempt. Analysis
failure does not start lookup. Failed, timed-out, rejected, or stopped
requests produce no completed timing sample. A subsequent request
restarts cleanly. The host calls are serialized and cancellation drops
both active and queued requests.

All six reference and 17 focused native tests passed. These are
pairs at the watcher/analysis boundary, where Flutter's tests also
inject the watcher and analysis services. Incular returns a local
timing result rather than transmitting Flutter analytics. Native filesystem integration and readiness on a connected transport
are checked separately below.


## Shared readiness on an existing IDE connection

All seven pinned `dtd_services_test.dart` registrations passed. The
Incular pair uses `PreviewIde` with controlled connection events, as
the Flutter originals inject fake DTD. Concurrent workspace and
document requests share exactly one listener, one stream request,
and one registry query. The client attaches locally before
subscribing, accepts an already-subscribed stream, waits if any of
the three analysis methods are missing, and does not wait for an
event when all methods are present. The workspace barrier uses that
same connection. A failed subscription drops the listener before
the next attempt; the native controls also cover cancellation, a
bounded initialization timeout, and unrelated events.

Nine focused Rust tests passed. The readiness adapter assumes an
already-connected transport. It never starts an analysis server,
but does not itself open an IDE socket or define a cross-language
wire protocol. Service launch, scaffold registration and end-to-end
IDE discovery remain unverified.

```sh
cargo test --locked --offline -j1 -p incular-devtools --test flutter350_preview_ide -- --test-threads=1
```


### Reloading through the existing connection

The pinned detector's existing-connection case at
`lsp_preview_detector_test.dart:204` also passed. The retained
`PreviewIdeReloadHost` connects the readiness adapter to the reload
coordinator and the same already-connected analysis service handles
both an explicit barrier and a source reload. Construction issues
no analysis request. The explicit call issues one, a file change
issues a second, and only its document is looked up and published.
A real Cargo.toml edit then updates the durable manifest hashes
without another IDE request. All 24 focused Rust IDE, reload, and
watcher tests passed. This pairs the injected service contract,
not a real LSP daemon or browser preview scaffold.

## MultiPreview member transformations

The actual Flutter `MultiPreview.transform` invokes its members in
declaration order before a custom group adds shared wrappers or changes
labels. The pinned `multi_preview_transform_test.dart` and native
`flutter350_preview_composition` exercise light and dark members, group
labels, and the outer-group/inner-member/leaf wrapper order. Repeating just
the base transform yields a fresh independent set without the group
changes. Both lanes passed, including eight native composition tests.

Incular expresses the member and group transforms as Rust traits. This
implements runtime expansion, not compiler annotation discovery or the
generated scaffold's MultiPreview registration. The fixture is authored
and adds no original-registration credit.

### Moving populated source directories

The real `DirectoryWatcher` used by pinned `flutter_tools` reports both
nested-file addition and removal when a populated directory enters or
leaves `lib` by rename. The authored
`reference-probes/tooling/preview_source_directory_move.dart` reproduces
it on Linux without a fake watcher. Invoke the pinned SDK's `dart` with
`--packages=.tmp/flutter/packages/flutter_tools/.dart_tool/package_config.json`
and pass the probe path.

Incular previously emitted nothing when the move did not change the
installed recursive watch roots. It now tracks directory topology
under registered source paths and requests a rescan when a subtree
arrives or leaves, even if the backend has no per-file events. The
watcher checks only the affected subtree for ordinary structural
events, keeps exclusions and symlink policy intact, and emits no rescan
on an unrelated file rename. The focused Linux native tests exercise
both move directions without notifications left over from the previous
move, alongside existing manifest, source and reload checks. This is
an authored comparison, not another original registration.

## Local package dependencies in the generated preview scaffold

For `regress_176018_test.dart:46`, pinned Flutter wrote both expected
local path entries under dependencies and dependency_overrides.
The full test timed out while fetching the rest of the scaffold's
tooling dependencies, so the original remains partial.

The native writer builds the scaffold's Cargo.toml from its host
template. Workspace packages appear as direct paths and crates.io
patches. A real offline Cargo metadata run resolves a workspace
example's version-only reference to its local sibling. Regenerating
without a member removes its dependency and patch, without losing
host metadata. An ambiguous duplicate name leaves the existing
manifest untouched. Two direct and five adjacent native checks
passed. This does not claim to launch a browser preview.

## Disposed analysis contexts

Both pinned permeable tool originals in
`preview_detector_regression_178472_test.dart:69,78` passed. The
coordinator now observes the lifecycle of its selected analysis
context separately from the watcher. A late watcher event for a
valid source finishes quietly without entering analysis. A direct
inspection of that source returns the empty snapshot without
trying to publish a reload. The native backend also disposes during
lookup in a control: an already returned snapshot is dropped rather
than installed in the preview.

The native tests write a real file under an active source context.
They first verify that live inspection returns its exact contents
without a publication, then dispose the backend independently of
the coordinator. Browser session setup is not part of these cases.

Three direct and 12 adjacent native preview checks passed.

## Running preview reloads and restarts

The pinned `widget_preview_test.dart:1138` permeable original passed,
including its generated scaffold startup. This test uses a controllable
resident runner; it does not launch a real browser. In its exact trace,
an edit is held before debug connection, followed by one initial reload.
While the second reload waits for acknowledgement, three further edits
produce just one follow-up reload. In the next burst an explicit restart
upgrades the pending reload and cannot be downgraded by a later edit.
Closing the preview prevents any more host work.

The native gate reproduces the five host operations in order: four
reloads followed by one full restart. An independent source/manifest
fixture writes real files, completes publication through the existing
analysis coordinator and produces the expected host operations. The
native control also verifies closing before connection and that queued
producers cannot outlive a dropped session. The Rust component takes
connection and shutdown signals from its host; it is not a browser
launcher or a DTD implementation.

Four direct and 15 adjacent native preview tests passed.

### Reconfigured preview localization

Pinned `widget_preview_scaffold/test/localizations_test.dart:49` passed.
The real Incular preview initially mounts an explicitly English
configuration, then rebuilds the descriptor with Spanish while retaining
the content element and the same two-language catalog. The previewed
widget itself reads the active locale and localized greeting, switching
from Hello to Hola without falling back to the application's locale.
The catalog is the Rust equivalent of Flutter's delegate configuration.
All nine focused composition tests passed.

### Empty snapshot and transformed previews

The pinned scaffold's `no_previews_detected_test.dart:16` and
`custom_previews_test.dart:90` passed. The neutral native gallery
mounts the help text and accessible documentation link only for an
empty snapshot. The pointer test invokes the injected documentation
opener once. When a reloaded snapshot has one preview, there is one
group and one preview, with neither help text nor link remaining.
Returning to an empty snapshot restores the help without opening it.

A second mounted case applies a fixed-size transform to each member
before the shared brightness transform. Both wrappers actually run,
and the two rendered preview headers match the pinned light and dark
labels exactly. The snapshot composition is separate from package
scanning, HTTP serving, and automatic source discovery. All three
gallery tests and nine composition tests passed.
