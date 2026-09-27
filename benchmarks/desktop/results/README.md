# Latest evidence

- [Latest DX12 descriptor-limit measurement](memory-architecture/gpu-descriptor-limit.json) and [interpretation](../DESCRIPTOR-LIMIT.md)
- [Single-run glyph-sharing measurement](memory-architecture/05-single-run-glyph-sharing.json) and [interpretation](../memory-plans/05-single-run-glyph-sharing-results.md)
- [Latest optional auxiliary element-state measurement](memory-architecture/01-optional-auxiliary-state.json) and [interpretation](../memory-plans/01-auxiliary-state-results.md)
- [Boxed text-field widget measurement](memory-architecture/03-boxed-textfield-widget.json) and [interpretation](../memory-plans/03-widget-textfield-results.md)
- [Boxed render-feature measurement](memory-architecture/03-boxed-render-features.json) and [interpretation](../memory-plans/03-render-feature-results.md)
- [Scrolling-state measurement](memory-architecture/01-scrolling-sidecar.json) and [interpretation](../memory-plans/01-scrolling-sidecar-results.md)
- [Optional semantic metadata measurement](memory-architecture/02-optional-semantic-metadata.json) and [interpretation](../memory-plans/02-semantic-metadata-results.md)
- [Latest measured text-cache change](memory-architecture/04-cache-key-sharing.json) and [interpretation](../memory-plans/04-text-cache-key-results.md)

Superseded result directories were moved to the Windows Recycle Bin on September 25, 2026. On September 27, superseded tracked build/test logs and per-launch traces were removed while compact measurement JSON, validation summaries, visual comparisons and the three logs linked from reports were kept. Raw local experiments under ignored `target/desktop-benchmark` are not part of this retained evidence.

- [Latest paired distribution memory/disk results](../MEMORY-DISK.md)
- [Previous release strict benchmark](windows/electron-ui-batched/incular-windows.json)
- [Electron strict benchmark](windows/electron-ui-electron-final/electron-windows.json)
- [Incular after interactions](windows/electron-ui-batched-after-smoke/incular-windows.json)
- [Latest complete workspace/GPU validation](validation-uploads/status.json)
- [Visual comparison](bundle-dist/visuals/compare-issue-tracker.png)

The release-profile memory measurements remain identified by their original bundle hash. A distribution-profile build is a separate artifact; its size must not be presented as a fresh memory measurement.

- [Distribution build validation](bundle-dist/status.json)
- [Distribution bundle manifest](bundle-dist/bundle-manifest.json)
