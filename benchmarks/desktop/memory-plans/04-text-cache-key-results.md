# Text cache key sharing: first measured result

Implemented 2026-09-26. Source: `crates/incular-text/src/engine.rs`.
[Compact paired evidence](../results/memory-architecture/04-cache-key-sharing.json).

The layout cache's map and FIFO queue now hold `Arc` references to one
`LayoutKey`. Previously the queue cloned the entire key, including its owned
text and any owned font family. The 2,048-entry cap, complete key equality,
eviction order, generation invalidation and externally held layout lifetime
remain unchanged. The existing comment calling this LRU was corrected to FIFO.

## Allocation census

One diagnostic test measured requested live bytes through the System allocator
after adding 512 unique 99-character labels to a warmed TextEngine. The
input strings were constructed before the baseline sample; the measured delta
includes whole layout/cache storage, not just keys.

| 512-entry cache | Retained allocation delta |
| --- | ---: |
| Before | 6,382,172 bytes |
| After | 6,111,516 bytes |
| Difference | **270,656 bytes (4.24%)** |

This is a test-process allocation count, not application resident memory. Its
input distribution was selected to expose key ownership and is not the issue
tracker's actual label distribution. All 512 second-pass lookups remained hits.

## Whole-application measurement

Three strict idle-qualified launches per executable on the same Windows AMD
Radeon 610M host, visible 1100 × 720 window, fixed 1,000-record/100-row issue
tracker workload. Decimal MB; CPU is percent of one logical core.

| Executable | Private resident median | Run range | Private commit median | Idle CPU |
| --- | ---: | ---: | ---: | ---: |
| Retained pre-change distribution executable | 109.35 MB | 109.14–110.03 MB | 147.93 MB | 0% |
| New distribution executable | **108.53 MB** | **108.45–108.55 MB** | **146.60 MB** | **0%** |
| Observed difference | **0.82 MB lower** | | 1.32 MB lower | |

The baseline SHA-256 is
`ede26893956a6faddc4d70271cc32ffecca8cfccd5444d2e6f6d048d784ea180`;
the new SHA-256 is
`531910c0d6a1de95e2b6743927f829050757c45785e19de77eb2cdb6505e2d9b`.
The pre-change executable comes from the prior validated distribution package.
It predates a commit adding on-demand profiling and GPU diagnostics; therefore
the 0.82 MB process difference is an observed comparison, not a fully isolated
causal estimate for this one code change. The direct allocation census above
does isolate the cache-key representation. The current application result is
still **28.53 MB above the 80 MB goal**.

The new executable is 10,815,488 bytes, 2,048 bytes larger than the old one.
Together with the existing 178,616-byte VC runtime DLL, its two-file installed
size would be 10,994,104 bytes (10.99 MB). This is a size calculation; the
prior bundle manifest has not been replaced by an unvalidated package.

## Correctness and checks

- Existing text-engine tests plus a new FIFO/external-owner regression: 15 pass.
- Allocation census test: 1 pass.
- All five captured UI state PNG hashes match the previous validated captures.
- Targeted Clippy with warnings denied, workspace Cargo check and formatting
  check passed. No full test suite was run.

The full launch traces and test/build logs remain in ignored local
`target/desktop-benchmark/under-80/` paths; the compact result file retains
hashes, policy, per-launch medians, spread and visual hashes.

Decision: **retain** this change. Next in the saved sequence: measure default
widget/element occupancy and inspect font diagnostic string sharing before
another implementation. Do not add this observed process difference to older
non-paired savings as if all runs had identical baselines.
