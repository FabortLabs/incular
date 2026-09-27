# Latest desktop benchmark results

Windows AMD Radeon 610M, three idle-qualified launches each. Decimal MB; CPU is percent of one core. Incular uses the latest distribution build; Electron is the retained previous measurement on this host and was not rerun in this experiment.

| Application | Private resident MB | Private committed MB | Idle CPU | Qualified launches |
| --- | ---: | ---: | ---: | ---: |
| Incular | **103.76** | **136.22** | 0.00% | 3/3 |
| Electron | 104.40 | 196.98 | 0.00% | 3/3 |

The new Incular executable plus the retained VC runtime DLL totals **11.01 MB**; the bundle manifest still describes the previous validated package. The memory result above uses the new executable. The retained Electron installed bundle is 386.14 MB.

Incular's new result is 0.64 MB below the retained same-host Electron result; Electron was not rerun for this change. QuickGUI's macOS chart uses a different OS and memory metric, so it is not a same-machine comparison.

- [Hardware findings and methodology](AMD-HARDWARE.md)
- [Paired memory and disk reductions](MEMORY-DISK.md)
- [Latest text cache key result](memory-plans/04-text-cache-key-results.md)
- [Retained scrolling-state result](memory-plans/01-scrolling-sidecar-results.md)
- [Boxed render-feature result](memory-plans/03-render-feature-results.md)
- [Boxed text-field widget result](memory-plans/03-widget-textfield-results.md)
- [Latest optional auxiliary element-state result](memory-plans/01-auxiliary-state-results.md)
- [Latest optional semantic metadata result](memory-plans/02-semantic-metadata-results.md)
- [Latest DX12 descriptor-limit result](DESCRIPTOR-LIMIT.md)
- [Single-run glyph-sharing result](memory-plans/05-single-run-glyph-sharing-results.md)
- [Bundle reduction and dependency patches](BUNDLE-SIZE.md)
- [Retained compact evidence and validation](results/README.md)
- [Electron visual comparison](results/bundle-dist/visuals/compare-issue-tracker.png)
