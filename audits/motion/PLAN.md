# Motion API implementation plan (lossless working notes)

Companion to [CHECKLIST.md](CHECKLIST.md) (status) and `.tmp/animation-api-design.md`
(the user-approved design; § numbers below refer to it). Branch `feat/motion`, off
`b2c2dbdc`. **Nothing committed yet.** The user asked to implement *everything* in the
design, without phasing (no P0/P1 split), and to keep the checklist updated as items land.

---

## 1. Decisions already made (do not re-litigate)

- Three entry points over one engine: `.transition()` (widget's own props), `.motion(|m| …)`
  (poses for states / presence / layout / shared / drag), `.animate()` (scripted chain, kept
  source-compatible). Plus `with_transition`, `view_transition`, reorder/DnD.
- The widget as written is the rest state. No `.initial`. `rest(…)` exists only for visual
  props the widget can't express. `enter` = "from", `exit` = "to".
- Built-in states are only what the wrapper observes: hovered, pressed, focused,
  focus_visible, dragging, in_view (+ drop_over / drop_active for drop targets). Everything
  else is `.when(cond, …)`. No `while_loading` / `while_checked`.
- `Pose` = compositor-only props. Precedence: per property, **later declaration wins**.
  Entering a pose uses that pose's `transition`; leaving uses the destination's (CSS rule).
- Groups are named (`Group::new("card")`); a condition resolves to the **nearest ancestor of
  that name** (lexical shadowing). No unnamed groups.
- Presence uses Rust control flow (removed element exits), not `.presence(bool)`.
- Opacity/color never spring: under a spring they get a 150 ms ease (`Transition::for_channel`).
- Shadow uses the canonical `incular_rendering::Shadow` (no parallel type).
- `focused` means focus on or inside the element (wrapper never becomes focusable);
  `focus_within` is offered as a *group* condition (`G.focus_within()`).
- `ms(u64)` free function (not `200.ms()` — E0689 on integer literals).
- Motion types live in `incular_widgets::motion` (facade: `incular::motion`); only the
  extension traits + `ms` go in the main prelude (avoids `Group`/`Shadow` name clashes).
- Scratch notes go in `.tmp/`; this plan + checklist are in `audits/motion/` on purpose (user
  wants to track progress).

## 2. Engine facts discovered (needed to continue)

- Each render node owns exactly **one** compositor layer attachment (`render_object/layers.rs`
  `AttachmentSpec`). So a motion site is a *chain* of controlled nodes, outer→inner:
  `frame_ticker → [Gesture focus/key observer] → [MouseRegion hover] → [Listener press] →
  [GestureDetector drag] → [size_observer] → Affine → Opacity(controlled) →
  DropShadow(controlled) → ColorFiltered(controlled) → Blur(controlled) → ClipRRect(reveal) →
  [env scopes] → child`. Only stages some pose uses are inserted, decided from the config, so
  structure never changes at runtime (inserting a wrapper later would remount the child).
- Retained controllers already compositor-driven (no rebuild when set): `OpacityController`,
  `ScaleController`, `RotationController`, `TranslationController`, `BlurController`,
  `ColorFilterController`, `DropShadowController` (all in `tree/values.rs`), read in
  `WidgetTree::update_compositor` (`tree/interaction.rs`).
- **Added:** `AffineParts` + `AffineController` (`tree/values.rs`), `WidgetKind::Affine` /
  `RenderKind::Affine` (mirrors `Scale` at every site: specs.rs, tree.rs, lowering/effects.rs,
  layout.rs `content_transform`, painting.rs hit-test matches, layers.rs, update.rs,
  structure.rs Debug/PartialEq/WidgetType/name "MotionTransform"/children, devtools.rs,
  devtools_props.rs, scrolling/layout.rs, reorderable.rs, layout/painting behavior mods,
  interaction.rs compositor arm). Constructor `Widget::controlled_affine`.
- **Added:** `FrameTick` trait + `FrameDelegate` (`tree/specs.rs`, exported from `tree`), new
  optional `frame` field on `AnimationTicker` (Widget+Render kinds, lowering, structure
  Debug/PartialEq), `Widget::frame_ticker(delegate, child)` (`constructors.rs`; delegate owns
  an idle controller + revision so rebuilds compare equal). Ticked in `tick_animations`
  (pre-build, honors `animation_muted`/TickerMode) and in `update_compositor`; contributes to
  `active` so frames keep coming.
- Waking frames: `incular_animation::schedule_frame()` → runtime `frame_requested()` checks
  `has_scheduled_frame()`. Motion calls it on every retarget.
- **Added to `incular_gestures::GestureCallbacks`:** `on_focus_within: Option<Rc<dyn Fn(bool)>>`
  and `on_key_observe: Option<Rc<dyn Fn(KeyboardEvent)>>`. Tree side in `tree/focus.rs`:
  `set_keyboard_focus` calls `notify_focus_within` (ancestors, nearest first);
  `dispatch_keyboard_where` first calls `notify_key_observers` (all ancestors, never
  consumes, skipped when a text field is focused).
- Reactivity: `incular_core::reactivity::{DependencySource, Subscription}`. `Signal` lives in
  `incular-runtime` (runtime → widgets dependency), so `impl From<&Signal<bool>> for Cond`
  must be written **in incular-runtime** using the signal's `inner.value.source()` and
  `get_untracked()`. Reading a Signal inside a build subscribes the *element* (rebuild) —
  that's why live conds use the source directly.
- `BuildContext::find::<T>()` reads an inherited env value without a dependency;
  `depend_on` registers one. `Widget::environment_scope(value, child)` publishes.
  `context.disable_animations()` = reduced-motion preference.
- Keyboard: `incular_core::KeyboardKey` (= keyboard_types `Key`; plain `Key` in
  incular_core is the *widget key* trait), `NamedKey`, `Code`. `event.state.is_down()`.
- Geometry API: `Rect { origin, size }`, `Rect::from_origin_size` (no `from_ltrb`, no
  `left()`); `Color::rgba(u8…)`, fields `red/green/blue/alpha`; `CornerRadii::uniform`;
  `RRect::new(rect, radii)`; `CoreTransform::{translation, scale_non_uniform, rotation,
  skew, then, coefficients, translation_offset, transform_point}` (`a.then(b)` applies b
  first). `incular_core::PRIMARY_POINTER_BUTTON`.
- `Opacity::new(alpha, child).controller(c)`; `Blur::controlled`, `ColorFiltered::controlled`,
  `DropShadow::controlled`; `ClipRRect::new(r, child).custom_clipper(impl CustomClipper)`
  (`get_clip(size) -> ClipShape::{Rect,RRect,Path}`, `should_reclip`).
- `Widget::retained_builder(init, build, constraint_dependent)` is crate-private; state must
  impl `RetainedState::rebind_revision`. Bumping the stored revision rebuilds that builder
  (used only for reveal-clip changes).
- Reconciliation short-circuits ptr-equal child `Widget`s, so rebuilding thin wrappers around
  the same child only reconciles the wrappers.
- `Column` cross-axis default is **center** (a test bug came from assuming left).
- Bash heredocs containing `'…'` in long multi-file commands broke once ("unexpected EOF");
  write files with the Write tool instead. No Python on this machine; use perl.
- `cargo fmt --all` hits os error 206 on Windows: run `cargo fmt -p <crate>` per package.

## 3. File map (what exists now)

| File | Contents |
|---|---|
| `crates/incular-animation/src/motion.rs` | `ms`, `secs`, `Spring` (snappy/smooth/bouncy/new/physical), `Transition` (ease/linear/spring/instant/curve/delay/opacity/color/for_channel/scaled), `Channel`, `TransitionKind`, `Keyframes`, `MotionValue` (new/map/get/velocity/target/is_animating/revision/set/stop/set_velocity/animate_to/animate_keys/tick; reversal shortening; spring keeps velocity) |
| `crates/incular-animation/tests/motion.rs` | 11 passing tests |
| `crates/incular-widgets/src/dx/motion/mod.rs` | `Motion` builder, `MotionExt::motion`, `InView`, `ExitLayout`, `Presence`, `LayoutMotion`, `SharedId`, `SharedMode`, `Stagger`, `StaggerFrom`; re-exports animation vocabulary, `Drag/Bounds/DragAxis/Release`, `Shadow` |
| `dx/motion/pose.rs` | `Prop` (25 props, identity/channel/stage/combine), `Stage`, `Combine`, `PoseValue` (Const/Bound), `Pose` |
| `dx/motion/cond.rs` | `Interaction`, `Interactions`, `Group`, `GroupFlag`, `GroupState`, `GroupScopes`, `LiveCond`, `Cond` (+ `&`/`\|`/`!`), free fns `hovered()` etc. |
| `dx/motion/config.rs` | `MotionConfig` widget, `MotionConfigScope`, `ReducedMotion`, `Reduced`, `reduce()` |
| `dx/motion/node.rs` | `MotionNode` retained state: resolve/settle/enter/composed/push/tick/is_active/stages/assemble/build; `ChannelSource` trait; `DragConfig`; `RevealClipper`; `motion_widget` |
| `dx/motion/presence.rs` | `StaggerScope`, `before_build`, `stagger_delay`, `scope_child`, `wrap` (identity — exit support goes here) |
| `dx/motion/dragging.rs` | `DragChannel` (ChannelSource adding X/Y), `attach` (GestureDetector pan), `bounded`, `release_target` |
| `dx/motion/geometry.rs` | `observe_size` (only when dragging) |
| `crates/incular-widgets/src/dx/drag.rs` | `Drag`, `DragAxis`, `Bounds`, `Release` |
| `crates/incular-widgets/tests/motion.rs` | 12 passing tests (Harness: mount, frame = tick_animations→layout→update_compositor→paint, hover/press via `dispatch_raw_pointer_in_window(7, …)`, reads the n-th `content_transform` in tree order) |
| `lib.rs` | `pub use dx::motion;` |

## 4. Remaining work, in execution order, with approach

Update CHECKLIST.md after each item lands (`[x]` only when tested).

1. **E9 live conds** — in `incular-runtime/src/environment.rs`: `impl From<&Signal<bool>> for
   incular_widgets::motion::Cond` via `Cond::live(self.inner.value.source().clone(), move ||
   s.get_untracked())`. Test in `crates/incular/tests/` (runtime) or widgets test with a raw
   `DependencySource`: flip → pose retargets, `rebuilds` unchanged.
2. **Tests for implemented-but-untested**: D2 filters (color matrix, blur sigma reach
   controllers), D3 shadow + reveal clip (reveal rebuilds wrapper only: `layouts`/`paints` of
   child unchanged), D5 bound MotionValue (drive value, no rebuild), E4 Enter/Space via
   `tree.dispatch_keyboard` with a focused descendant, E5 focus via `set_keyboard_focus`, H2
   `Stagger::delay` math, J1/J2 drag via pan sequence (down/move/up), N1/N2/N3 reduced
   motion (`MotionConfig::reduced_motion(Always)` → geometry instant, opacity ≤150 ms; `Keep`).
3. **E7 in_view + geometry hook** — extend `FrameTick` with `fn wants_geometry(&self) ->
   bool` and `fn geometry(&self, rect: Rect, visible: Option<Rect>)`; in `update_compositor`
   for ticker nodes with `wants_geometry`, compute the render's global rect (layout-space:
   sum of offsets excluding the node's own transform; find an existing helper like
   `render_world_transform`) and the nearest viewport/clip rect; call it. Node sets
   `InView` flag using `InView::amount`/`once`. Same hook feeds G (layout), K (shared), J
   (reorder rects). Must be cheap when unused.
4. **E14** debug warning (`#[cfg(debug_assertions)]` log once) when `pressed` declared before
   `hovered`; plus G5 unkeyed `layout()` warning. Use `tracing::warn!` if widgets has tracing,
   else a devtools diagnostic.
5. **N4** loops / off-screen: muted subtrees already skip ticks (FrameDelegate under
   `animation_muted`); add pause of `Play::InView` timelines when out of view (after I).
6. **C `.transition()`** — `pub trait Transitionable: Into<Widget>` in `dx/transition.rs`;
   implement by reusing `ImplicitlyAnimated` machinery in `implicit_animations.rs`
   (`AnimatedContainer` etc.). `TransitionExt::transition(self, impl Into<Transition>) ->
   Widget`: for `Container` build an `AnimatedContainer` with the same props. Needs
   `ImplicitTiming` to accept a `Transition` (spring support: drive with MotionValue 0→1 or
   per-prop). Minimum: Container, Padding, Align, Positioned, Opacity,
   FractionallySizedBox, PhysicalModel, DefaultTextStyle. Typed: only these impl the trait.
   Interruption continuity: retarget from current interpolated value (implicit widgets already
   do `begin = current`). Docs: AnimatedX are the same implementation.
7. **I `.animate()` port** — rewrite `dx/animate.rs` so `From<Animate> for Widget` builds a
   `.motion()`-style node with a **timeline `ChannelSource`**: each `Effect` becomes a
   per-prop track evaluated at elapsed time (Fade→Opacity multiply, Slide→SlideX/Y add,
   Move→X/Y add, Scale→Scale multiply, Rotate→Rotate add, Blur→Blur, Shake→X add). No
   per-tick rebuild. Keep `frame(elapsed)` (test helper) and all existing methods.
   Then: `Play::{OnMount, InView{once}, When(Cond), Handle(&AnimationHandle)}`,
   `AnimationHandle` (play/reverse/seek/restart/finished future via `TickerFuture`-like
   oneshot), `progress(Progress::{value, signal, scroll, view(ViewRange)})` (scroll/view
   from the geometry hook), `target()` → deprecated alias of `progress(value)`, new effects
   tint/saturate/shadow/clip_reveal/shimmer (shimmer = ShaderMask stage: add `Stage::Mask`
   + a `ShaderMask` controlled wrapper or rebuild path), `custom(|t, Pose| Pose)`,
   `build_each_frame(|t, child| Widget)` (rebuild path), `keys(Prop, [..])`,
   `then_gap(d)`, `then_overlap(d)`, `apply(fn)`. Stagger applies to `.animate()` too
   (read `StaggerScope` delay on mount). Existing tests in `tests/dx_layers.rs` must keep
   passing.
8. **F presence/exit** — reconciliation support for *departing elements*: when a child
   element whose widget is a motion site with an exit pose is removed, keep it mounted in a
   per-parent "departing" list instead of unmounting; mark non-interactive (skip hit test),
   excluded from focus traversal and semantics; tell the node to `exit()` (animate to exit
   pose) and unmount when settled. Find the unmount path in `tree/reconciliation.rs`
   (child list diff) — need a hook "element about to be removed → can it defer?". Marker:
   the outer widget from `presence::wrap` (e.g. an `ExitScope` WidgetKind or a field on the
   ticker delegate: `FrameTick::begin_exit(&self) -> bool`). Re-entry: same key re-inserted
   while departing → revive the element, node re-resolves (reverse). `ExitLayout::Pop`:
   departing child excluded from parent layout, painted at last offset. Keyed replacement +
   `Presence::Wait` (parent delays mounting the new child until departing children are gone
   — implement in the parent motion's node by holding the new child until exit completes).
   Topmost only: descendants of a departing element don't start their own exits.
9. **G layout motion** — via geometry hook: node remembers last layout rect (in nearest
   scroll content space); on change, set layout channel offset = old − new and animate to 0
   (ChannelSource adding X/Y, and for `Bounds` scale correction or real size lerp). Real
   size lerp: constrain the child to the interpolated size via a retained
   `SizedBox`-like wrapper whose size comes from a controller → relayout of that subtree
   only. Scroll-aware: subtract nearest scroll offset. `with_transition` (L) overrides.
10. **H** stagger for `.animate()` (after I).
11. **J DnD + reorder** — `DragDrop<T>` wrapping existing `drag_drop::DragDropContext<T>`;
    `.draggable(&dnd, payload)` (lifts the element itself via `Draggable` with feedback = the
    child, Dragging flag), `.drop_target(&dnd, |t| t.accepts(..).on_drop(..))` setting
    DropOver/DropActive flags. `.reorder_item(key)` + `.reorderable(on_move)` on any
    multi-child layout: items publish their rects (geometry hook) into a `ReorderScope`;
    dragged item follows pointer on its drag channel; insertion index by
    `Collision::{ClosestCenter, PointerWithin}`; siblings shift via layout channel; drop →
    spring to slot, call `on_move(Move{key, from, to, from_list, to_list})` once
    (`live(true)` = on every slot change). `ReorderGroup` for cross-container (Kanban).
    `reorder_handle()`, `axis_lock()`, `long_press_on_touch()`, `auto_scroll()` (nearest
    ScrollController at edges), `lift(pose)`, `placeholder(Gap|Ghost(f32))`. Keyboard:
    Space lift / arrows / Space drop / Esc cancel, semantic custom actions move up/down,
    live-region announcement "Moved X to position i of n". `Move::apply(&mut Vec<T>, key_fn)`.
    `Bounds::Parent` for free drag also uses the geometry hook.
12. **K shared** — global per-window registry (thread-local keyed by `SharedId` + scope id)
    of last rects of unmounting/departing shared elements; a mounting element with a
    matching id starts with layout channel offset/scale from that rect (FLIP). `Bounds`
    mode crossfades (old snapshot kept as departing element fading out). `shared_scope()`
    namespaces. Cross-route: works through the same registry since routes share the window;
    then make `Hero` an alias built on it (keep `Hero` API; HeroController becomes
    unnecessary but keep compiling). Check `incular-navigation` route transitions.
13. **L transactions** — `with_transition(t, f)` / `without_transition(f)` in
    `incular_animation` (thread-local stack of override transitions, stamped with a frame
    epoch); MotionNode/Transitionable/layout consult `current_transaction()` when
    retargeting during the frame(s) the update produces (store the override on retarget
    calls that happen before the next `end_frame`). Expose via facade prelude.
14. **M view transitions** — `view_transition(f)` in runtime/widgets: capture the root's
    current layer subtree (retained, `Arc<DisplayList>` pictures) as a frozen overlay layer,
    run `f`, then crossfade/reveal old over new for the duration; `.reveal(Reveal::circle_from
    (point))` uses a clip layer; `view_transition_scope()` limits capture to a subtree.
15. **O tooling** — devtools props for motion sites (tier badge: composite / paint (reveal)
    / layout (transition/size)); motion value inspector (list active values via devtools
    props of the Affine/ticker nodes); hot reload replays enters (hook
    `incular_core::hot_reload` epoch → reset `mounted`).
16. **P** docs pages under `docs/content/docs/api/` (motion, transition, drag & reorder,
    shared, view transitions), update `extensions/animate.mdx`; `examples/motion` (photo
    grid §14) as an `incular` package example target (add `[[example]]` in
    `crates/incular/Cargo.toml` like the others); facade: `pub use incular_widgets::motion`
    as `incular::motion`, prelude gets `MotionExt`, `TransitionExt`, `ms`, `with_transition`,
    `view_transition`; `specs/architecture.json` only if a new crate edge appears (none
    planned).
17. **Validation** (AGENTS.md): `cargo fmt -p <each touched crate> -- --check`,
    `cargo check --workspace`, `cargo test-constrained`,
    `cargo clippy --workspace --all-targets --all-features -- -D warnings`. Fix the 3 unused
    import warnings in `motion/mod.rs` (re-export them publicly or drop). Never add
    `#[cfg(test)]` in `src/`; tests only in `tests/`.

## 5. Known gaps / honesty notes for the final report

- Touch press is immediate and cancels after 18 px of movement; it is not delayed by the
  press timeout inside scrollables yet.
- Layers are chosen from the configuration, not dropped at identity at runtime.
- Reveal clips rebuild the thin wrapper chain (paint-tier, child untouched); everything
  else is compositor-only.
- Momentum release projects with a 0.25 s power then springs (not a true friction curve).
- `.transition()` retargets keep position but restart progress at velocity 0 (multi-property
  tweens share one 0→1 progress, so per-property velocity is not carried). Springs can
  overshoot decoration colors on Container (geometry channel drives the whole tween).
- `Bounds::Parent` uses the nearest ancestor box that differs from the node's own box
  (pass-through wrappers skipped).
- Motion lints print via `eprintln` once per distinct message in debug builds (widgets has
  no logging crate); `Motion::lints()` exposes them for DevTools/tests.
- Shared elements: a per-thread registry keyed by (scope, id); the newest member holds the element and
  starts from the previous holder's on-screen rect (or from a rect left at most 500 ms ago); when the
  holder leaves, the newest remaining member flies back. Element mode hides the other members.
  Different elements are needed: an unkeyed swap at the same index reuses the element (no match).
  Popping a route with a reverse animation hands back only when the page unmounts.
- `Hero` was NOT rebuilt on `shared()`: it is the Flutter-parity implementation the parity harness and
  crates/incular/tests/heroes.rs pin (shuttles, placeholders, rect tweens, `HeroMode`). `shared()` covers
  the controller-free use; replacing `Hero` would need a parity decision from the user.
- View transitions: the old scene is the layer tree re-flattened at the start of the next frame (cached,
  so no extra work when nothing changed) and painted by `WidgetTree::paint` above the new scene, like the
  semantics debugger overlay. Gaps: `shared()` elements inside a transition do not morph (they crossfade
  with the rest); a scoped transition clips the old *scene* to the scope rect, so anything else painted
  over that rect (a popup) is in the snapshot too; reduced motion is not consulted (the tree has no
  `MotionConfig`); the scope API takes an explicit `ViewScope` handle (`.within(&scope)`).
- Transactions (`with_transition`): one thread-local override that ends when the next
  `update_compositor` returns. It reaches motion sites (`transition_for`, so poses, enter/exit,
  `layout()`, `shared()`, reorder) and `.transition()` widgets; duration-based `AnimatedX` (no
  `.transition()`) are unaffected. With several windows on one thread, the first window's frame ends it.
- Motion drags start at the press (`DragStartBehavior::Down`), not after the slop.
- `.animate()` port: `EffectKind`/`Effect` are no longer `Copy` (they hold keyframes and
  custom closures): a breaking change for code that copied them. `Play::When` jumps to the
  matching end on the first build. Clip reveal is paint-tier (same wrapper rebuild as poses).
  `frame(elapsed)` now returns the real site held at that position.
- Presence works in box child lists (`reconcile_children`: Column, Row, Stack, Wrap, Flex).
  Sliver/dynamic child lists remove at once. `ExitLayout::Pop` sites shrink-wrap their child
  (Align loosens constraints). The removed element's topmost motion site on its single-child
  spine decides; a site without `exit` leaves at once.

## 6. Resume point (2026-11-17)

- J done and tested (tests/dnd.rs 1, tests/reorder.rs 14). Fixes made while testing: motion drags
  use `DragStartBehavior::Down` (element stays under the pointer); post-drop relax counts down
  on the list's `ListTicker` instead of frame callbacks (runs in the widget-tree harness too);
  touch long-press drags are finished/followed by the pan callbacks when the pan recognizer takes
  over mid long-press (`dragging::pan`).
- Test harness note: `dispatch_gesture` has device kind `Unknown` (touch slop, 36 px pan slop);
  use `dispatch_pointer_gesture_in_window(0, 0, kind, ..)` for a real device kind.
- K, L, M, O, P1-P4 done (K4 partial, see §5). Remaining: P5 validation (fmt clean; clippy, check, test-constrained), then a final report. Not committed (the user un-did the WIP commit; changes are staged + unstaged on master).
