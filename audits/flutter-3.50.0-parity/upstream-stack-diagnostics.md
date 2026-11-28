# Foundation diagnostic stack output

`incular_core::DiagnosticStack` accepts the producing runtime's trace format,
filters by the original frame properties and sends console messages to the
caller's sink. It keeps surviving frame sources unmodified, canonicalizes
async gaps, reports sorted **actual** internal-frame elisions, and emits the
label before the filtered output. The raw line limit is enforced before
discarding internal frames. The exact mangled Flutter package trace is in the
native test fixture (copyright Flutter Authors, BSD license next to it).

A pinned Flutter probe confirms exact default elision wording and grouping,
label order, and the raw line limit. It also confirms three surprising
custom-filter behaviors at this revision: the final otherwise matching
window is not scanned, a later overlapping pattern overwrites the earlier
reason, and a two-frame custom reason is reported as “3 frames.” Incular
preserves those externally visible results, rather than inferring new
defaults from comments. An empty Rust pattern is a safe no-op.

The original async-gap console case and too-long repetitive filter passed on
both sides. The two tests using a **live Dart VM** stack were run on the
reference side, but are partial: native fixtures validate filtering of
supplied text, not implicit live VM capture. Similarly, generating a real
Dart `ParallelWaitError` remains unpaired; the equivalent text and rendering
path were checked, not the Dart error object's construction. The focused
native suite passed six tests, and the parser regression suite passed eight.

## Named stack diagnostics

The pinned `foundation/diagnostics_test.dart:1630` original
passed. Formatting a trace as a flat block keeps all three
frame lines, including the file positions and whitespace.
The heading gets a colon by default; disabling its separator
removes only the colon. All 23 focused native stack and
diagnostic-tree tests passed.
