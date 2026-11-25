# PageView layout options, controller handoff and pre-layout commands

All eleven pinned originals selected here pass. Rust boundary target passes 3/3; controller-edges target passes five and intentionally fails one.

## Boundary geometry

- :1173 partial: the same retained viewport reports default HardEdge and updated AntiAlias, and its recorded rectangular clip disappears when disabled. Rust diagnostics plus display list recording do not reproduce the reference painting-context mode callback or certify rectangular antialiasing pixels.
- :1207 partial: the default half-fraction page starts at x=200 with end insets, then x=0 when padEnds(false) is repumped. This is actual geometry corresponding to Dart's inspected SliverFillViewport property.
- :1223 partial: the nearest f32 width to 392.72727272727275 allows initial page 152 and a jump to final page 365 of 366 without exceptions. This does not certify the exact f64 arithmetic path.
- :1304 reference only. Flutter's controller exposes explicit onAttach/onDetach hooks; no corresponding public Incular hook was found.

## Retained controllers

- :1384 and :1413 match. An added controller can drive pages 0, 1, 2; rebuilding without an external controller is safe and shows page 0 when no prior selection was made.
- :1392 and :1404 partially match. A replaced/removed controller cannot move the live view; the replacement can drive it. Dart also asserts on commands sent to the detached controller, whereas Rust can defer commands. These rows compare actual visible authority, not identical API errors or the selection transfer tested separately at :591.
- :1572 partial mismatch. Flutter's first child build reads a null page while content dimensions are unavailable, then sees a number after rebuild. Incular's child builder already reads Some(0) on the first invocation and still reads Some(0) after rebuilding. Rust schedules an equivalent descriptor rebuild programmatically instead of tapping the fixture button.
- :1624 and :1650 partial match. A request issued after constructing the view but before its first runtime frame completes without throwing, then displays page 1, for both animate/jump. Flutter tests a covered Navigator page; the Rust fixtures test the command timing directly and do not claim equivalent Navigator layout suppression.

Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_boundary_layout and cargo test --locked -p incular-runtime --test flutter350_upstream_page_controller_edges.

## Attachment validation and PageView cache options

All thirteen selected originals passed on the pinned reference. The extended controller-edge binary has seven passes and two intentional failures.

- :1463 and :1490 partially match the safety boundary: both frameworks prevent the command from ambiguously driving two live views. Flutter allows both attachments but asserts when animate/jump is invoked. Incular rejects the second live attachment earlier with DuplicateScrollAttachment at layout. No matching assertion-message contract is claimed.
- :1515 and :1548 partially mismatch: Flutter rejects a request on a controller never attached to any PageView. Incular deliberately queues it; a subsequently attached view is already on page 2 before the next explicit command. The unrelated view stays on page 0. Once attached both support commands normally. These rows flag the deferred behavior for a compatibility decision rather than requiring the Dart exception type.
- :1701 and :1763 partial match: the cold default PageView mounts only page 0 and exposes only its label, where Flutter's default false setting does the same. The true branch, where a full adjacent page is mounted and exposed semantically, is not configurable on the Rust PageView.
- :1678, :1730 and :1794 are reference-only. Explicit, changing, and inconsistent ScrollCacheExtent/allowImplicitScrolling combinations are not exposed in the Rust descriptor.
- :1818, :1847, :1884 and :1914 are reference-only. The Flutter cases check reveal from a mounted implicit-cache neighbor or physics-wrapper updates. Incular's ordinary Scrollable ensureVisible path and default zero-cache PageView do not exercise these contracts.

## Precision-width raster control (separate from the Flutter text golden)

The pinned Flutter source test at :1253 uses a pre-recorded text golden that is not present in the local SDK checkout. The original remains open. An independently authored Flutter probe uses the original logical viewport width of 411.42857142857144 and same 300ms ease animation to page 2, replacing the font-dependent text with a 60×22 solid marker on a black 512×192 frame. Its native Windows wgpu counterpart uses the nearest representable f32 width. Both pass; interior, inclusive near-inside edge, and immediately outside samples match at device ratio 1. This confirms representative geometry/paint at that width, not text or golden equivalence.

Run Flutter with run-reference-probe.ps1 -Test page_view_fractional_width_raster_test.dart. Set INCULAR_DESKTOP_LIVE_TESTS=1 and run cargo test --locked -p incular-desktop --test flutter350_page_fractional_width_raster -- --nocapture.

After the live PageView position handoff repair, a previously unattached
controller's queued command no longer replaces the already visible page
when it takes over a retained viewport. Incular still accepts the
unattached request rather than raising Flutter's assertion, so the
`:1515` and `:1548` dispositions remain partial mismatches. Fresh mounts
still support Incular's pre-layout command API.

## Default implicit reveal is not an explicit scroll

An authored paired probe uses a 400px viewport and fraction 0.625. The
neighbor occupies x=250..500 and still contributes a semantic label.
Flutter leaves the default PageView at offset 0 for show-on-screen;
Incular previously moved to 100px. The repaired runtime leaves the
inner page stationary while still letting an ordinary vertical ancestor
reveal the target (100px in the paired nested fixture). The ordinary
single-scroll reveal regression remains green.

In the pinned Flutter opt-in control, the same reveal moves immediately
to 100px and snapping then settles back to 0. That is a different
behavior than never moving. Incular's opt-in API and cache defaults are
not covered by this default-path repair. Authored checks add no original
registration to the coverage ledger.

## Opt-in cache and implicit reveal

PageView now separates the implicit reveal policy from the actual cache
extent. The effective default is zero viewport lengths when disabled
and one viewport length when enabled. Explicit viewport multiples
are resolved at layout; pixel values stay fixed when the viewport
resizes. A mismatched explicit value/flag pair is rejected in debug.

Exact pinned registrations `:1678`, `:1701`, `:1730`, `:1763`,
`:1794`, `:1818` and `:1847` have matching behaviors. The tests check
which keyed pages actually mount, which semantic labels are present,
the retained page when the setting changes, the rejection paths, and
the actual final offset after an explicit or default cached reveal.
The authored pixel/viewport comparison resizes from 400px to 200px:
a fixed 300px cache grows from two materialized pages to three, while
a one-viewport cache still retains two.

A partly visible fractional page also matches both phases of the pinned
opt-in comparison: immediate movement to 100px, then a spring back to
0; the default continues not to move. The effect of the scroll-physics
switch at `:1884` is covered, but Dart object's pointer identity is not.
These checks compare portable semantics and do not verify native
screen-reader focus behavior.

### Visibility of a retained semantic page

An authored paired check verifies the distinction between being cached
and being visible: offscreen page 1 is still present in the semantics
tree, but hidden. A reveal makes it visible and marks the now-offscreen
page 0 hidden. Incular retains both semantic identities while modifying
their bounds and visibility; an explicit hidden flag is not cleared
by geometry updates. Portable and AccessKit projection fixtures pass.
This does not reproduce a native screen reader session.
