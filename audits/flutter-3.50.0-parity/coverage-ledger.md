# Full-audit coverage ledger

Baseline is the checked-out Flutter 3.50.0 commit in [README](README.md).
These files make the review enumerable; they do not certify behavior.
The old 3.41.7 audit, fixture names, and lexical matches cannot close a row.

| Input | Ledger | Interpretation |
| --- | --- | --- |
| Flutter framework API | [flutter-public-api.jsonl](flutter-public-api.jsonl) | 15,791 declarations/members from the 11 public entrypoints; explicit members only, with owner, line and signature. |
| Flutter source and test files | [coverage-queue.tsv](coverage-queue.tsv) | 441 in-scope framework source files, 776 Flutter test/helper files, 47 existing harness source files. Includes files without parsed declarations or test registrations. |
| Flutter upstream scenario registrations | [upstream-scenarios.jsonl](upstream-scenarios.jsonl) | 8,749 registration sites, not assertions reviewed or total dynamic cases. A row records the upstream file/line, scenario description, group and options. |
| Existing harness | [existing-harness-scenarios.jsonl](existing-harness-scenarios.jsonl) | 261 Dart registration sites (frame and scalar tests); Rust harness sources accounted for separately. Not assumed to use the current baseline. |
| Exceptions and counts | [scenario notes](scenario-notes.json), [summary](coverage-queue-summary.json), [external re-exports](external-reexports.md) | Explicit scan boundaries and external contracts; no row passes automatically. |

The upstream catalog uses animation, foundation, gestures, painting, physics,
rendering, scheduler, semantics, services and widgets tests, plus preview
tooling and preview scaffold suites for widget_previews. Material and Cupertino
suites are not parity targets. A neutral upstream test can use a design-system
widget as a fixture; preserve its neutral assertion, then substitute a neutral
fixture if needed. Pure Dart and engine-owned helpers are tracked when they
affect observable in-scope contracts; the catalog does not pretend to enumerate
all tests in those separate trees. Re-exported dependencies are accounted for in
the external appendix.

## How to close a case

1. Record what the Flutter source actually promises: default values and nullable
   cases, lifecycle/exceptions, observable state/order, and where the contract
   changes by platform. Trace the default from public descriptor to widget/render
   object/host. Reconcile migrated or renamed APIs by semantics.
2. Trace the Incular path. Record the call site and lowering, not only a
   structurally similar type. Classify **source match**, **source mismatch**,
   **deliberate difference**, or **unreviewed**. Scope and platform-specific
   limitations stay explicit. A match covers only the stated case.
3. For reproducible behavioral cases, record exact setup, sequence and output
   on each side. Mark **Incular reproduced against source expectation** if only
   Incular ran; mark **paired verified** only when both run on the pinned
   builds. Geometry, pixels and platform adapters have separate evidence.
4. Expand any upstream registration with variants, loops, parameters or custom
   wrappers into the actual cases before claiming runtime coverage. Preserve
   skip reasons and unavailable environments; do not silently treat a skip
   as passing. Parsed scenario rows are an accounting tool, not a matching or
   testing script.

Case-design dimensions: exact threshold minus/at/plus, zero/null/default,
fractional and large values, normal and reversed axes or RTL, nested/ancestor
effects, lifecycle races and cancel/reentrancy, offstage/hit/semantics, and
platform/DPR variants where applicable. Test unconfigured, explicitly
configured, and disabled forms. Include positive controls so a fix does not
remove behavior that already matches.

## Validation and regeneration

The file ledger explicitly reconciles all catalog rows to paths in the
pinned source/test trees. Unmapped external/dynamic scenarios are described
above and in the notes; no Flutter tests were executed by the catalog.

With the scratch analyzer environment from [README](README.md), run:

```powershell
Push-Location .tmp/flutter350-audit-tools
dart --disable-dart-dev --packages=.dart_tool/package_config.json ../../audits/flutter-3.50.0-parity/scenarios.dart ../flutter ../.. ../../audits/flutter-3.50.0-parity
Pop-Location
python audits/flutter-3.50.0-parity/coverage-queue.py audits/flutter-3.50.0-parity .tmp/flutter
```
