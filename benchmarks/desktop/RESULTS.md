# Latest desktop benchmark results

Windows AMD Radeon 610M, three idle-qualified launches each. Decimal MB; CPU is percent of one core. Incular uses the latest distribution build; Electron is the retained previous measurement on this host and was not rerun in this experiment.

| Application | Private resident MB | Private committed MB | Idle CPU | Qualified launches |
| --- | ---: | ---: | ---: | ---: |
| Incular | 106.79 | 144.57 | 0.00% | 3/3 |
| Electron | 104.40 | 196.98 | 0.00% | 3/3 |

The new Incular executable plus the retained VC runtime DLL totals **11.00 MB**; the bundle manifest still describes the previous validated package. The memory result above uses the new executable. The retained Electron installed bundle is 386.14 MB.

Incular still exceeds Electron's measured private resident RAM. QuickGUI's macOS chart uses a different OS and memory metric, so it is not a same-machine comparison.

- [Hardware findings and methodology](AMD-HARDWARE.md)
- [Paired memory and disk reductions](MEMORY-DISK.md)
- [Latest text cache key result](memory-plans/04-text-cache-key-results.md)
- [Retained scrolling-state result](memory-plans/01-scrolling-sidecar-results.md)
- [Latest boxed render-feature result](memory-plans/03-render-feature-results.md)
- [Latest optional semantic metadata result](memory-plans/02-semantic-metadata-results.md)
- [Bundle reduction and dependency patches](BUNDLE-SIZE.md)
- [Latest raw results and validation](results/README.md)
- [Electron visual comparison](results/bundle-dist/visuals/compare-issue-tracker.png)
