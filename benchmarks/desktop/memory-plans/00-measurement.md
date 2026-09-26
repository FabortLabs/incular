# Measurement and acceptance protocol

Status: prerequisite plan. All ten optimization plans use this protocol.

## 1. Establish a reproducible baseline

Record commit plus dirty-diff identity, executable hash, compiler/profile/features,
DLL versions, GPU/driver, OS, DPI, window dimensions and harness settings. Preserve
the last validated binary until a successor is verified. Rebuild the actual current
baseline rather than attributing old results to new source.

Keep 1,000 data records and 100 retained rows, exact content/interactions, visible
window, DPI and hardware backend. Preserve accessibility, editing, localization,
codecs, effects and optional capabilities. No working-set trimming, hidden/smaller
windows, software fallback, benchmark-only virtualization, or removal of the AMD
idle-CPU workaround.

## 2. Build an allocation census

Count live objects and allocated capacity for Element, RenderNode and unique
WidgetNode descriptors. Record enum frequency, optional-state occupancy, text
cache entries and unique backing arrays. Collect size/alignment, owned payload
length/capacity, allocation count and high-water usage. Empty collections have
metadata but normally no backing allocation; Arc clones are not data copies.

Capture startup, scrolling/filtering, details/editing, resize and return to idle.
Use a separate large-tree/document stress workload to assess scalability. Keep
diagnostics opt-in, bounded and outside hot paths. Place test-only helpers under
tests; do not expose private representations as application API for measurement.

The prior Rust live-heap sample was about 17.70 MB. WGPU reported about 13.04 MB
live resources inside 18.55 MB reserved blocks. These differ from process resident
and commit accounting. Do not add/subtract those values as disjoint domains.
Likewise the 93.96 MB isolated triangle probe is a different workload, not a floor.

## 3. Evaluate one patch at a time

1. Estimate net savings: removed inline/payload/capacity bytes minus new metadata,
   allocation headers, allocator rounding, indexes and retained capacity.
2. Run focused correctness tests. Use short memory probes to reject ineffective
   candidates cheaply; do not publish them as strict benchmark results.
3. For credible candidates, run equivalent release/dist builds through the existing
   strict Windows harness: same stabilization policy, three launches and existing
   sample schedule. Save median and spread; avoid compilation during sampling.
4. Measure live/peak heap, private resident/commit, idle CPU, startup and interaction
   timing, and installed bytes. Compare all five existing visual states for changes
   that can affect output. Check semantic output separately from pixels.
5. If a change is smaller than noise, mark it inconclusive; repeat only the relevant
   measurement when justified. Never select the best launch as the result.

## 4. Acceptance and evidence

Correctness, ownership and architecture are mandatory. Require a demonstrable net
retained-byte reduction or meaningful peak/recovery benefit. Structural savings
may not immediately lower resident pages; report both honestly. Reject extra idle
wakeups, unexplained stalls, or interaction CPU regressions beyond repeat-run
variability. Set any explicit performance tolerance before interpreting results.

Use columns: candidate, baseline/new hashes, unique retained bytes, peak heap,
resident/commit median and spread, idle CPU, interaction timings, bundle bytes,
pixels/semantics, decision and limitations. Do not double-count shared allocations
or claim that cache eviction freed externally owned resources.

Keep one compact latest evidence set per accepted change/batch. Preserve the
validated baseline until replacement. Summarize failed experiments rather than
accumulating binaries and log dumps. Never delete unrelated user files. Revert
rejected implementations independently and record why they failed.
