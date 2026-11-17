# Motion API implementation checklist

Design: `.tmp/animation-api-design.md` (section numbers below refer to it).
Branch: `feat/motion`.

Legend: `[ ]` not started · `[~]` in progress / partial (note says what is missing) · `[x]` done and tested.

## A. Foundation — `incular-animation` (§2, §9)

- [x] A1 `ms()` / `secs()` duration helpers
- [x] A2 `Spring` presets: `snappy`, `smooth`, `bouncy`, `new(duration, bounce)`, `physical(m, k, c)`
- [x] A3 `Transition`: `ease`, `spring`, `instant`, `.curve`, `.delay`, per-property overrides; `From<Spring>`, `From<Duration>`
- [x] A4 `MotionValue<f32>`: retained value + velocity, `set`, `animate_to(target, transition)`, `velocity`, `is_animating`, settle detection
- [x] A5 Interruption continuity: springs keep velocity; eases restart from the current value
- [x] A6 Opacity / color channels never spring (short ease under a spring transition)
- [x] A7 `MotionValue::map` derived values (no rebuild)
- [x] A8 Keyframe tracks (`keys`) with even or explicit times

## B. Engine — `incular-widgets` tree (§11)

- [x] B1 `AffineController`: compositor-only retained transform (translate, scale x/y, rotate, skew, origin)
- [x] B2 Frame-delegate ticker node so retained motion state ticks on the tree's animation clock (time scale, `TickerMode` muting)
- [~] B3 `MotionNode`: channels (timeline · pose · drag · layout) composed: translate Σ, scale/opacity Π — pose + drag channels done; timeline and layout channels pending
- [~] B4 Lazy layers: opacity / filter layers only while non-identity — layers are chosen from the configuration (only stages some pose uses exist); not yet dropped at runtime while at identity
- [x] B5 Composite-only proof test: no build/layout/paint counters during a pose animation

## C. `.transition()` (§3)

- [x] C1 `Transitionable` trait (typed; only interpolable widgets get the method)
- [x] C2 impls: `Container`, `Padding`, `Align`, `Positioned`, `Opacity`, `FractionallySizedBox`, `PhysicalModel`, `DefaultTextStyle`
- [x] C3 `.transition(impl Into<Transition>)` — duration or spring, retargets continuously — `MotionValue` progress 0→1, retarget from the on-screen value; opacity uses the fade rule; reduced motion snaps
- [x] C4 `AnimatedContainer` & friends documented as the same implementation — module docs in `dx/transition.rs`; every `Animated*` also gets `.transition()`

## D. `Pose` (§4)

- [x] D1 `Pose` value: opacity, x, y, offset, slide_x/y, scale, scale_x/y, rotate, skew, origin
- [x] D2 filters: blur, brightness, saturate, grayscale, tint
- [x] D3 shadow, clip reveal (inset / circle) — reveal clips rebuild the thin wrapper (one wrapper layout per frame); shadow is compositor-only
- [x] D4 per-pose `transition`
- [x] D5 pose fields can bind a `MotionValue` (live link, no rebuild)

## E. `.motion()` states (§5.1–5.4)

- [x] E1 `.motion(|m| …)` modifier on every widget
- [x] E2 `hovered` — mouse/pen only, hit-tested on the untransformed layout box
- [x] E3 `pressed` — passive (never joins the arena), cancels on pointer cancel
- [x] E4 `pressed` from Enter/Space while focused
- [x] E5 `focused`, `focus_visible`, `focus_within`
- [~] E6 `dragging` — set by free drag; reorder/DnD pending
- [x] E7 `in_view` (+ `amount`, `once`)
- [x] E8 `when(cond, …)` + `Cond` algebra (`&`, `|`, `!`; `bool`, `&Signal<bool>`)
- [x] E9 `&Signal<bool>` conditions retarget without rebuilding the subtree
- [x] E10 Precedence: per property, later declaration wins
- [x] E11 Transition choice: entering pose's transition / destination's on leave
- [x] E12 `rest(…)`
- [x] E13 Named `Group`s: `group(G)`, `G.hovered()/pressed()/focused()/focus_within()/any_hovered()`, nearest same-name ancestor shadows
- [x] E14 DevTools/debug warning: `pressed` declared before `hovered` — `Motion::lints()` + one-time debug `eprintln` (no tracing in widgets); covers pressed→hovered, focus_visible→focused, dragging→pressed/hovered

## F. Presence (§5.5)

- [x] F1 `enter` plays on mount only (not on rebuild)
- [x] F2 `exit`: departing elements kept until exit settles — `reconcile_children` asks the removed child's topmost motion ticker (`FrameTick::begin_exit`); `tree/departing.rs` keeps it in place and drains it before the next layout
- [x] F3 Departing elements: no hit testing, no focus, no semantics — IgnorePointer + excluded focus + hidden semantics, toggled by property (no remount)
- [x] F4 Re-entry mid-exit reverses (state preserved) — same element id revived; exit reverses to the resolved pose
- [x] F5 `ExitLayout::InPlace` / `ExitLayout::Pop` — Pop collapses the site to zero size (Align factors), painting from its top-left
- [x] F6 Keyed replacement crossfade + `Presence::Sync` / `Presence::Wait` — presence parent lays its child in a Stack and counts departures through a `PresenceScope`
- [x] F7 Only the topmost removed element exits

## G. Layout motion (§5.6)

- [x] G1 `layout()` FLIP position on the compositor — geometry listener writes the inverse delta into a layout X/Y channel; test asserts zero relayouts while gliding
- [x] G2 size animation (relayout of that element only) — reflowing `ChildSizedLayout` lays the child out tightly at the interpolated size (thin builder relayout only)
- [x] G3 `layout_position()`
- [x] G4 Scroll-aware measurement (no animation from scrolling) — compared in content space; a scroll-scope change never animates
- [x] G5 Debug warning for unkeyed `layout()` siblings — `MotionLint::UnkeyedLayout` (child key checked; wrapper now carries the child key)

## H. Stagger (§5.7)

- [x] H1 `stagger(Stagger::every(d))` on a parent offsets children's enter / `.animate()`
- [x] H2 `StaggerFrom::{Start, End, Center, Index}`
- [x] H3 Stagger reaches `.animate()` chains (shared stagger index with enters)

## I. `.animate()` (§6)

- [x] I1 Port the chain onto the compositor-only engine (no per-tick rebuild for transform/opacity) — timeline `ChannelSource` on a motion node; test asserts no rebuild/layout/paint
- [x] I2 `play(Play::OnMount | InView | When | Handle)` — `Play::{OnMount, InView, When, Handle}` (+ `in_view().once()`, `when(cond)`)
- [x] I3 `AnimationHandle`: play, reverse, seek, restart, finished future (+ pause, progress, is_playing); one handle can drive several chains
- [x] I4 `progress(Progress::value | signal | scroll | view)`; `target()` kept as deprecated alias — value (smoothed), motion, live (`From<&Signal<f32>>`), scroll (range, horizontal), view (Cover/Contain/Entry/Exit)
- [x] I5 New effects: tint, saturate, shadow, clip_reveal, shimmer — shimmer is paint-tier (ShaderMask rebuilt per frame), the rest compositor
- [x] I6 `custom(|t, pose| …)` (composite) and `build_each_frame(|t, child| …)`
- [x] I7 `keys(Prop, [...])`, `then_gap`, `then_overlap`
- [x] I8 `apply(preset)`

## J. Drag, drop, reorder (§7)

- [x] J1 Free drag: `drag(Drag::x/y/both)`, `within(Bounds)`, `elastic`, `Release::{SnapBack, Momentum, SnapTo, Stay}`
- [x] J2 Drag offset is compositor-only
- [x] J3 `DragDrop<T>`, `.draggable(&dnd, payload)`, `.drop_target(&dnd, |t| t.accepts().on_drop())` (tests/dnd.rs, tests/reorder.rs)
- [x] J4 `dnd.active()` / `dnd.over()` conditions (tests/dnd.rs, tests/reorder.rs)
- [x] J5 `.reorder_item(key)` + `.reorderable(on_move)` on any layout (measured rects) (tests/dnd.rs, tests/reorder.rs)
- [x] J6 `Collision::{ClosestCenter, PointerWithin}` (tests/dnd.rs, tests/reorder.rs)
- [x] J7 Siblings shift via layout motion; drop settles with spring; `on_move` once (`live(true)` opt-in) (tests/dnd.rs, tests/reorder.rs)
- [x] J8 `ReorderGroup` cross-container moves (Kanban) (tests/dnd.rs, tests/reorder.rs)
- [x] J9 `reorder_handle`, `axis_lock`, `long_press_on_touch`, `auto_scroll`, `lift`, `placeholder` (tests/dnd.rs, tests/reorder.rs)
- [x] J10 Keyboard reordering (Space / arrows / Esc) + semantics announcements + move actions (tests/dnd.rs, tests/reorder.rs)
- [x] J11 `Move::apply` helper (tests/dnd.rs, tests/reorder.rs)

## K. Shared elements (§8.1)

- [x] K1 `shared(id)` within one tree (start from the departing rect) (tests/shared.rs)
- [x] K2 `shared_bounds(id)` with content crossfade (newcomer fades in; a still-mounted old holder morphs to the new bounds and fades out)
- [x] K3 `shared_scope()` namespacing
- [~] K4 Cross-route matching in `Navigator`; `Hero` alias — cross-route matching works with no controller (crates/incular/tests/shared_routes.rs); `Hero` kept as the Flutter-parity implementation, not rebuilt (see PLAN §5)

## L. Transactions (§8.2)

- [x] L1 `with_transition(t, || …)` overrides transitions caused by that update (tests/transaction.rs)
- [x] L2 `without_transition(|| …)`

## M. View transitions (§8.3)

- [x] M1 `view_transition(|| …)` crossfade using the retained old layer tree (tests/view_transition.rs; the last scene's display list, re-flattened from the unchanged layer tree, paints above the new scene in its own opacity group)
- [x] M2 `.reveal(Reveal::circle_from(point))`, `.duration`
- [x] M3 `view_transition_scope()` — as `.view_transition_scope(&scope)` + `view_transition(..).within(&scope)` (an explicit handle)

## N. Configuration & accessibility (§10)

- [x] N1 `MotionConfig`: default transition, reduced-motion policy
- [x] N2 Reduced motion: transforms/layout snap, opacity keeps a short fade, loops pause — transforms snap, fades capped at 150 ms; `.animate()` one-shots jump to the end and loops hold
- [x] N3 Per-site `reduced(Reduced::Keep)`
- [x] N4 Off-screen / muted subtrees stop ticking — TickerMode mutes ticks; looping chains park while out of view and resume on entry

## O. Tooling (§10)

- [x] O1 DevTools cost-tier badge per motion site (`motion.tier` in node details: composite / paint / build / layout; tests/motion_devtools.rs)
- [x] O2 Motion value inspector (`motion.values`, `motion.states`, `motion.layers`, `motion.animating`)
- [x] O3 Hot reload replays enter animations (runtime `reassemble` calls `motion::replay_enter_animations`; tests/motion_tooling.rs)

## P. Migration, docs, examples (§12)

- [x] P1 Aliases documented (`AnimatedSwitcher`, `AnimatedSize`, `Hero`, `Animate::target`) (motion page: AnimatedSwitcher = presence, AnimatedSize = layout(); animate page: target = progress; shared page: Hero kept as the Flutter-parity implementation)
- [x] P2 Docs pages for transition / motion / drag / shared / view transitions (docs/content/docs/api/motion/*, extensions/animate.mdx; snippets type-checked by hand: no Python here)
- [x] P3 `examples/motion` showcase (photo grid from §14) (compiles; not run in a window)
- [x] P4 Prelude exports (`incular::motion` module; prelude: MotionExt, TransitionExt, DndExt, ReorderExt, ViewTransitionExt, Transition, Spring, ms, with_transition, without_transition, view_transition)
- [ ] P5 Validation: fmt, check, test-constrained, clippy
