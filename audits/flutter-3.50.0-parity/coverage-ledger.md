# Full-audit coverage ledger

Baseline is the checked-out Flutter 3.50.0 commit in [README](README.md).
These files make the review enumerable; they do not certify behavior.
The old 3.41.7 audit, fixture names, and lexical matches cannot close a row.

| Input | Ledger | Interpretation |
| --- | --- | --- |
| Flutter framework API | [flutter-public-api.jsonl](flutter-public-api.jsonl) | 15,791 declarations/members from the 11 public entrypoints; explicit members only, with owner, line and signature. |
| Flutter source and test files | [coverage-queue.tsv](coverage-queue.tsv) | 441 in-scope framework source files, 776 Flutter test/helper files, 47 existing harness source files. Includes files without parsed declarations or test registrations. |
| Flutter upstream scenario registrations | [upstream-scenarios.jsonl](upstream-scenarios.jsonl) | 8,856 registration sites, not assertions reviewed or total dynamic cases. A row records the upstream file/line, scenario description, group and options. |
| Existing harness | [existing-harness-scenarios.jsonl](existing-harness-scenarios.jsonl) | 261 Dart registration sites (frame and scalar tests); Rust harness sources accounted for separately. Not assumed to use the current baseline. |
| Exceptions and counts | [scenario notes](scenario-notes.json), [summary](coverage-queue-summary.json), [external re-exports](external-reexports.md) | Explicit scan boundaries and external contracts; no row passes automatically. |

The catalog includes 216 direct calls to the verified testGesture wrapper (Flutter packages/flutter/test/gestures/gesture_tester.dart:28-35). Helpers nested in an already registered test are not additional registrations.

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


### Registration inventory correction

The earlier count of 8,965 included 69 non-registration calls: 50 in
preview tooling, 14 in widgets, and five in gestures. For example,
`Cache.test(processManager: ...)` constructs a fixture and
`test(Offset.zero)` calls a local assertion helper inside a registered
test. The parser now accepts unqualified runner calls with at least two
positional arguments in this checkout. All 69 removed sites were checked
against the pinned source; all other rows and comparison evidence stayed
the same. This changes the denominator, not the number of behavior
comparisons. Runtime variants, unrecognized registration helpers, and
conditionally skipped paths still need separate accounting.

A second pass removed 40 awaited fixture calls from the widgets count,
bringing 8,896 sites to 8,856. Each removed call invokes a declared local
`Future<void> test(...)` helper, not a test registration. This covers one
Container golden fixture call and 39 sliver fixture calls across four
files. Their parent registrations remain, along with the requirement to
check every exercised fixture before marking those registrations paired.
All removed sites were checked against the pinned source, and none had
comparison evidence. No other scenario rows changed.

## Validation and regeneration

The file ledger explicitly reconciles all catalog rows to paths in the
pinned source/test trees. Unmapped external/dynamic scenarios are described
above and in the notes; no Flutter tests were executed by the catalog.

Use the analyzer configured by the initialized pinned Flutter Tools
package (14.3 at this revision). Keep the separately captured original
harness inventory because its ledger refers to the source locations at
that harness snapshot, not subsequent edits to the fixtures.

```bash
AUDIT="$PWD/audits/flutter-3.50.0-parity"
FLUTTER="$PWD/.tmp/flutter"
SCRATCH="$PWD/.tmp/inventory-recheck"
mkdir -p "$SCRATCH"
"$FLUTTER/bin/cache/dart-sdk/bin/dart" \
  --packages="$FLUTTER/packages/flutter_tools/.dart_tool/package_config.json" \
  "$AUDIT/scenarios.dart" "$FLUTTER" "$PWD" "$SCRATCH"
cp "$SCRATCH/upstream-scenarios.jsonl" "$SCRATCH/scenario-notes.json" "$AUDIT/"
python3 "$AUDIT/coverage-queue.py" "$AUDIT" "$FLUTTER"
```

## Verified progress counters

Run `python audits/flutter-3.50.0-parity/coverage-progress.py` to validate
that all upstream and harness dispositions reference exact catalog rows and
existing evidence files. Its percentages refer to registration sites, not
the total dynamically generated cases or all application behaviors.
