# Share glyph positions for one-run lines

Implemented 2026-09-27 in `incular-text`. [Compact evidence](../results/memory-architecture/05-single-run-glyph-sharing.json).

`TextLine::glyphs` and its sole `GlyphRun::glyphs` previously retained two allocations containing identical `GlyphPosition` values. A one-run line now shares the run's immutable `Arc<[GlyphPosition]>`; it also avoids constructing the temporary line-level vector. Lines with multiple runs still concatenate every run in visual order. Document composition still rebases line glyphs separately because its glyph coordinates differ from the paint run's origin; no public field or coordinate convention changed.

The existing focused allocation census, using 512 unique 99-character single-run labels, reports 5,087,516 requested live bytes after the change versus 6,111,516 in the preceding recorded census: **1,024,000 fewer retained bytes**, or 2,000 per label. This is a test-process heap delta and excludes allocator overhead. The earlier number was recorded before the unrelated multiline font-run fix, which does not affect these single-line inputs. The new pointer-identity test confirms the shared allocation and checks the multi-run concatenation.

Three strict idle-qualified launches of each executable on the same Radeon 610M host gave:

| Build | Private resident median | Three launch values | Private commit median | Idle CPU | EXE bytes |
| --- | ---: | --- | ---: | ---: | ---: |
| Baseline | 105.52 MB | 114.20, 105.52, 105.46 MB | 144.88 MB | 0% | 10,827,264 |
| Shared glyphs | 105.40 MB | 105.40, 105.67, 104.89 MB | 140.84 MB | 0% | 10,827,776 |

The resident medians differ by 0.12 MB, but the ranges overlap and one baseline launch had a large, still idle-qualified high-memory excursion. **The issue-tracker process result is inconclusive**, and neither the commit difference nor the change in private commit can be attributed confidently to this patch. The structural retained-byte saving in long single-run labels is direct. All five UI state PNG hashes are unchanged. The 17 focused text-engine tests and allocation census pass; no full test suite was run.

Decision: retain the sharing because it removes an exact duplicate allocation while preserving the public data, paint output and mixed-run behavior. Do not count an app resident-memory win toward the 80 MB goal. The remaining plan-05 work is mixed-run and document storage, whose differing coordinate spaces need a separate design and measurement.
