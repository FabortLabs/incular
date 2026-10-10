# incular-devtools-ui

## Architecture and support

| Contract | Status |
| --- | --- |
| Ownership | Standalone desktop DevTools application over the public facade and diagnostics protocol. |
| API class | application; this package is a tool, not a framework re-export surface. |
| Support | Available as an opt-in desktop tool; transport/model limits are enforced independently of application UI state. |

The tool opts into the exact facade features it needs (desktop, controls,
material, and devtools) instead of inheriting the facade default set.

## Workspace

The native workbench uses compact top navigation and independently scrolling
widget-tree and detail panes. The inspector keeps widget identity, dimensions,
and detail tabs visible while inspecting long property lists. It opens directly
on the widget inspector. Overview shows CPU
frame statistics, display-budget misses, memory inventory, and recent diagnostics.

- **Widget inspector:** virtualized rows with indent guides, typed labels and
  child counts; a draggable tree/details divider; selection history and clickable
  ancestor breadcrumbs; subtree isolation; and search across collapsed branches.
  Combine `type:Text`, `text:hello`, `key:save`, `id:…`, and `has:children` filters.
  Enter selects the first match; Up/Down move through matches and wrap around.
  Reveal selection clears the filter and restores the widget's tree context.
  Properties are filterable and grouped into collapsible sections, with editing
  for properties explicitly exposed by the target. Layout includes a retained
  box model, constraints, clipping, and layout decisions. Signals, invalidation
  causes, semantics, and overlays have their own persistent tabs.
- **Target picker:** click Pick widget or press Ctrl+Shift+C, hover the target to
  preview geometry, and click to inspect. The chosen widget is revealed in the
  tree. Escape cancels picking in either window.
- **Performance:** nearest-rank CPU p95, budget-aware jank rate, a frame chart,
  recording, selected ranges, responsive flamegraphs, and ranked trace work.
- **Memory:** resource inventory cards and an A/B comparison table.
- **Console:** live text and severity filters; pause the display while target
  capture continues. Clearing the console also resets retained byte accounting.
- **Transport:** actual local DevTools request status, duration, and sent/received
  bytes, with failure filtering and a 256-entry bound. This view does not capture
  application HTTP traffic.
- **Application:** target identity, protocol, native windows, and connection retry.

**Export report** saves versioned JSON to `Downloads/Incular DevTools` (falling
back to the OS temporary directory) and shows the saved path in the status bar.
Reports contain retained frames, traces, snapshots, logs, transport metadata,
and the selected widget; discovery credentials and request bodies are omitted.

The tool stays open when no target is running. Start a DevTools-enabled app
with `--devtools`, then use **Find target** or **Reconnect**. Use
`--target-pid <pid>` to select an exact process. `INCULAR_DEVTOOLS_VIEW` can
select the initial view (`overview`, `widgets`, `performance`, `memory`,
`console`, `transport`, or `application`).

## Native visual review

Run with a live target:

```text
cargo run -p incular-devtools-ui --example devtools_visual_review -- --target-pid <pid>
```

This opt-in review uses Incular's normal input pipeline and GPU frame capture
to exercise navigation, layout inspection, overlays, memory snapshots,
console controls, transport filters, and recording. PNG captures are saved to
`target/devtools-review`. Set `INCULAR_DEVTOOLS_REVIEW_DIR` to choose a capture
directory, and `INCULAR_DEVTOOLS_REVIEW_WIDTH` / `INCULAR_DEVTOOLS_REVIEW_HEIGHT`
to inspect another window size. Set `INCULAR_DEVTOOLS_REVIEW_OFFLINE=1` with a
nonexistent `--target-pid` to capture the connection workspace.
Set `INCULAR_DEVTOOLS_REVIEW_INSPECTOR=1` for the shorter inspector review,
including large trees, keyboard navigation, picking, subtree isolation, overlays,
property groups, search, and reveal.
