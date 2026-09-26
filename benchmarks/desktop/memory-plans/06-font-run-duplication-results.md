# Multiline font-run metadata duplication

Implemented 2026-09-26. [Compact paired evidence](../results/memory-architecture/06-font-run-duplication.json).

`TextEngine::layout_document` rebased every paragraph font-run record inside the
loop over visual lines. An 11-line paragraph therefore retained 121 records
where its source layout had 11. Each duplicate carried owned family/script
strings. The rebasing now happens once per paragraph before line assembly.

A focused regression failed before the fix (121 versus 11) and passes after it.
It also verifies the second paragraph's byte ranges. All 16 targeted text-engine
tests pass. The underlying shaping, glyph positions and visual output are
unchanged; five UI state captures have identical hashes.

In the fixed issue-tracker workload, a strict three-launch run measured
**109.04 MB** private resident median (range 108.90–109.20 MB), **149.69 MB**
private commit median and **0%** sampled idle CPU. The preceding cache-key-only
build measured **108.53 MB** resident median. This run does **not** demonstrate
an app-level memory win from removing duplicate font-run metadata; the issue
tracker's retained multiline content and process variation limit that comparison.
One launch restarted idle qualification after a memory fluctuation. The exact
binary hashes, run medians, policy and visual hashes are in the evidence file.

Decision: **retain as a correctness and multiline-memory fix**. Do not count an
unobserved issue-tracker saving. The proposed shared-string representation in
[plan 06](06-font-debug-strings.md) remains separate and unimplemented because
the public `String` fields require an API compatibility decision.
