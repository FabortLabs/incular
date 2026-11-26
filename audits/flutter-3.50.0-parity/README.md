# Flutter 3.50.0 parity audit

**Stage:** Focused behavioral verification and framework fixes after the
initial source audit. Every in-scope library is inventoried. Unreviewed
contracts remain open; this is not a parity certification.

- Flutter reference: local `.tmp/flutter`, tag `3.50.0`, commit
  `ab2912035e160d9cad6306df307576da6128ebdb`. No fetch or checkout was done.
- Incular source reviewed: `c647cb856bda434be4657660e492c43dcee882b9`.
- Earlier reference: Flutter `3.41.7` (`cc0734ac716fbb8b90f3f9db8020958b1553afa7`).
  The generated signature data retains [Flutter’s license](LICENSE.flutter).
  Older audit artifacts remain untouched. Their coverage labels and fixtures
  are not automatically valid against 3.50.0.
- Public API and observable contracts first; performance second. Keep wgpu
  and intentional Incular features such as built-in animation composition.
  Material, Cupertino, and other design-system parity are excluded. Engine
  internals, VM mechanisms, and language-specific surface are not automatic
  port targets; document the Rust or platform equivalent where relevant.

## Artifacts

| Artifact | Read it as |
| --- | --- |
| [Behavior audit](behavior-audit.md) and [focused probes](behavior-probes.md) | Neutral focus, paint, editing and boundary cases; six wrong results reproduced on Incular. |
| [Full-audit ledger](coverage-ledger.md) | Source and test queue, open scenarios, and evidence rules. |
| [Manual contract review](contract-review.md), [paired evidence](focused-comparisons.md) and [checkpoint](audit-checkpoint.md) | Broader source review, six pinned comparisons, and explicitly open tracks. |
| [Findings](findings.md) | Current source evidence and behavior/API differences. |
| [Flutter public API](flutter-public-api.jsonl) | Framework-owned, locally exported declarations and explicitly declared public members with line, signature, owner and export entrypoints. One row per source symbol. |
| [Top-level review](top-level-review.tsv) and [counts](review-counts.json) | Rust lexical leads and historical evidence to review, not parity verdicts. |
| [Flutter API delta](flutter-api-delta.jsonl) | AST signature/export/file changes since the 3.41.7 tag, not body-only changes. |
| [Flutter source delta](flutter-source-delta.tsv) | Changed files in the 11 library trees, including behavior-only changes. |
| [Inventory notes](inventory-notes.json) | Exports traversed outside the local framework, parse diagnostics, and conditional export/extension caveats. |
| [External exports](external-reexports.md) | Grouped dependency and engine boundary dispositions. |
| [Live checkpoint](audit-checkpoint.md), [coverage validator](coverage-progress.py) | Verified progress, work still open, and distinct frame/upstream denominators. |
| [Frame ledger](harness-dispositions.tsv), [upstream ledger](upstream-dispositions.tsv) | Per-scenario outcomes with exact registration sites and linked evidence. |

## Regression tests

Use the [backfill ledger](test-backfill.tsv) to find permanent test targets
for the previously reproduced cases. A focused test may intentionally fail
at its reference assertion until the implementation is fixed. Run only the
named targets; do not run the entire workspace for this audit. The
[Flutter reference probes](reference-probes/test/) pin the expected side
of selected regressions in code. Run one from the repository root, e.g.
`./audits/flutter-3.50.0-parity/run-reference-probe.ps1 -Test raster_slice_test.dart`.
The runner verifies the pinned checkout and uses an offline scratch package.

The [frame fixture milestone](frame-list-wheel-and-coverage-milestone.md)
records 253/253 comparable harness registrations, not full upstream or
device coverage. Run a named original with the [pinned upstream
runner](run-upstream-reference.ps1), for example:
`./audits/flutter-3.50.0-parity/run-upstream-reference.ps1 -Test painting/alignment_test.dart -FullName 'AlignmentGeometry.resolve()'`.

On Linux, use the [pinned reference runner](run-reference-linux.sh):

```sh
bash audits/flutter-3.50.0-parity/run-reference-linux.sh probe tap_region_actionable_semantics_test.dart
bash audits/flutter-3.50.0-parity/run-reference-linux.sh upstream painting/alignment_test.dart 'AlignmentGeometry.resolve()'
```

Keep the official `3.50.0` tag in the pinned checkout, even when fetching it
shallowly. Flutter uses the tag for SDK version resolution; a commit-only
checkout may report an unknown SDK to Pub. On the first run with an empty
Pub cache, set `INCULAR_BOOTSTRAP_PUB=1` to populate dependencies for the
selected scratch package. Subsequent runs resolve dependencies offline.
The Linux runner checks the revision, runs one file with test concurrency
one, and verifies that upstream runs actually executed the named tests.
Successful Linux reference and [focused frame](run-focused-frames-linux.sh)
runs keep test logs and any comparison reports/JSON, but discard compiled
Dart build products. Set `INCULAR_AUDIT_KEEP_DART_BUILD=1` for a run that
needs to keep those products. Failed runs retain their isolated package.

Use `python3 audits/flutter-3.50.0-parity/coverage-progress.py --by-library`
for paired, partial, and unreviewed registration counts per library.
These are ledger counts; the linked test evidence establishes behavior.

## Inventory by public entrypoint

Counts include cross-library re-exports. Do not add the rows to get unique
symbols or interpret any count as progress toward parity.

| Flutter library | Top-level | Members |
| --- | ---: | ---: |
| animation | 59 | 332 |
| foundation | 143 | 457 |
| gestures | 196 | 660 |
| painting | 138 | 1,178 |
| physics | 12 | 59 |
| rendering | 535 | 4,530 |
| scheduler | 22 | 106 |
| semantics | 44 | 612 |
| services | 224 | 2,062 |
| widget_previews | 9 | 36 |
| widgets | 1,286 | 8,255 |

There are **15,791 unique framework rows** across **412 parsed Dart files**,
with **2,110 top-level review rows**. The old-to-new comparison found **606**
changed declaration keys: 236 added, 47 removed and 323 moved or changed in
signature/export visibility. The source diff has **259** path records. These
counts are discovery, not verified gap counts. Scope is from the 11 public barrels, not
all Dart files that happen to live under `lib/src`.

## How evidence was assigned

1. Traverse local exports and parts from the 11 public entrypoints, respecting
   show/hide for local declarations; capture owned public members and enum
   values. Compare the snapshots with the same extractor.
2. Cross-reference current neutral Rust source and prior evidence.
   `lexical-candidate` includes exact and snake_case names, not necessarily
   reachable facade exports or correct implementations. `historical-lead-only`
   is old evidence, not a 3.50 verdict. `no-lexical-lead` is not proof of an
   absence. No top-level verdict, and no method/field verdict, is implied
   unless supported by the source review.
3. Review contracts and numeric behavior in each library. Record partial
   implementations, confirmed differences and intentional architecture choices.
   Review checked out sources, not newer online documentation.

AST inventory limits: conditional exports are a union; inherited members are
listed on declaring owners, not copied onto every subtype; unnamed extensions
are logged rather than listed; external dependency/engine edges are listed but
not expanded; and annotations such as `@internal` or `@protected` are not a
promise of app-level support. The pinned analyzer recovers from new doc syntax
warnings and a trailing comma on `OverlayChildLayoutInfo`; the latter's three
getters were checked manually (see W11). Counting declarations or registration
sites is discovery, not verification. Since the initial inventory, focused
Flutter/Rust behavioral checks, strict frame comparisons, and selected wgpu
pixel readbacks have run. No Incular full workspace suite was run. Native
assistive technology, platform-wide pixel results, and performance are not
certified. The old harness baseline was 3.41.7; use the focused runner to
verify the pinned SDK and generate fresh 3.50 results.

## Highest-impact next checks

1. Declarative navigation and scheduler: reproduce state loss and same-frame
   callback timing (W01, SC01). These can affect otherwise ordinary apps.
2. Neutral layout, input, and accessibility: verify the new sliver/cache
   contracts, web conventions, autofill, semantic tap regions and IME styles
   (R02/R03, F03, SV01/SV02, W02/W09).
3. Fill the remaining contract ledger systematically: validate actual facade
   visibility and signatures against the inventory, then confirm behavior
   family by family. Cover unmatched foundation/services and neutral preview
   tooling explicitly, not by assuming the old audit already did.
4. Benchmark focused equivalents after API and behavior are settled. Treat
   static performance risks in the findings as hypotheses until measured.

## Regeneration

Run from the Incular checkout in PowerShell with Python 3.9+ and Dart 3.11.5.
The analyzer dependency is pinned by the historical tool manifest and lockfile.
Use a scratch package; do not bootstrap or update either SDK as part of the
inventory. The following regenerates the current snapshot and Rust leads:

```powershell
$root = (Get-Location).Path
$out = Join-Path $root 'audits/flutter-3.50.0-parity'
$tmp = Join-Path $root '.tmp/flutter350-audit-tools'
New-Item -ItemType Directory -Force $tmp | Out-Null
Copy-Item audits/flutter-parity/inventory_tool/pubspec.yaml,audits/flutter-parity/inventory_tool/pubspec.lock $tmp
Push-Location $tmp
dart pub get --offline
dart --disable-dart-dev --packages=.dart_tool/package_config.json "$out/inventory.dart" "$root/.tmp/flutter/packages/flutter/lib" $out
Pop-Location
python "$out/compare.py" $out
```

To regenerate the API delta, archive `packages/flutter/lib` at `3.41.7` from
the local Flutter clone into a scratch directory, run the same extractor there
and separately save its output. Invoke `python delta.py OLD_JSONL NEW_JSONL
OUTPUT_JSONL`. For the source delta, run `git diff --name-status
--find-renames 3.41.7 HEAD` in the Flutter clone, scoped to the 11
entrypoints and corresponding `packages/flutter/lib/src/` trees.
