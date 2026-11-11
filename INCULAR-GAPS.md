# Incular gaps

Framework gaps found while building and profiling Incular DevTools
(`tools/incular-devtools`). Each entry names the symptom, where it lives and
the workaround DevTools uses today.

## Widgets and controls

- **`ActionSurface` ignores `padding` when `content` is set.** The padding
  only applies to the label path, and an `Align` in `content` fills the whole
  available width. Every custom-content button therefore has to pad and size
  its content itself. Location: `incular-widgets/src/internal.rs`
  (`ActionSurface`). Workaround: `views::shared::button_content`.
- **Action surface overlays are square.** Hover, pressed and focus colours
  paint a rectangle with no corner radius, so rounded buttons need a clip.
  Workaround: an `ActionInteractionController` plus a `stateful_builder` that
  paints a rounded fill (`views::shared::state_surface`).
- **`Tooltip` wraps its child in a second, unlabelled action surface.** A
  tooltip around a button hides the button's accessible label from semantics
  lookups (`Simulation::click` fails), and may intercept the press. Location:
  `incular-material/src/feedback/transient.rs`. Workaround: DevTools does not
  wrap icon buttons in tooltips.
- **`TextField` has no `focus_node` or `autofocus`.** A dialog cannot focus
  its input on open. Workaround: the command palette owns its query and reads
  `KeyboardEvent::key` through a focused `KeyboardListener`.
- **Small built-in icon set.** `widgets::internal::icons` has only chevrons,
  check, close, plus and minus. Workaround: `views::icons`, stroked Lucide-style
  paths drawn on a `Canvas`.
- **No clipboard access from widget callbacks.** `Runtime::set_clipboard_text`
  needs `&mut Runtime`, which button callbacks do not get. Workaround: a
  channel drained by a UI-thread task (`views::clipboard`).
- **Prelude name clash.** `incular::prelude::Key` is the widget-key trait, so
  the keyboard key must be spelled `incular::core::KeyboardKey`.

## Focus

- **Focus is lost when the focused element unmounts.** After the focused
  widget (a tree list, a search field) leaves the tree, nothing is focused
  and no `KeyboardListener` receives keys, so application shortcuts stop
  working. Flutter moves focus to the enclosing scope. Workaround: DevTools
  re-requests its root focus node on every view switch and when dialogs
  close.

## Performance

- **`BoxShadow` blurs through an offscreen layer per shadow.** Shadows on
  every card of a dense layout are expensive (`box_painter.rs` builds a
  `Widget::blur` layer for each shadow). There is no cheap path for small,
  static shadows. Workaround: a one-pixel solid contact shadow under cards;
  only floating panels use a real blur.
- **`ClipRRect` costs a stencil push and pop per use.** Rounded clips lower
  to `DrawBatch::StencilRRect` (`incular-wgpu/src/renderer/lowering.rs`),
  which breaks batching. Clipping many small buttons is costly. Workaround:
  rounded fills painted by the builder instead of clips.
- **No subtree memoization primitive.** A root builder that rebuilds
  rebuilds everything under it unless the caller returns the identical
  `Widget` (only `ptr_eq` or leaf equality bails out of reconciliation).
  Workaround: `shell::WidgetCache`, keyed by a hash of the region's inputs.
- **Layout dominates frame time on large retained pages.** Profiling DevTools
  while switching views showed layout at a p95 of about 27 ms in a debug
  build, ahead of build and paint. Pages built from `Column`, `Row`,
  `Expanded` and many `LayoutBuilder`s relayout fully on change; there is no
  relayout boundary a caller can request.

## Tooling

- **`Simulation` has no frame-timing API.** Measuring an interaction needs a
  second process: DevTools now ships `devtools_profile_probe`, which connects
  to any DevTools-enabled target and reports per-phase timings.
