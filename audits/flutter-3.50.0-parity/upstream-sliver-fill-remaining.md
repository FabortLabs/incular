# SliverFillRemaining viewport geometry and semantics

Pinned Flutter 3.50.0 passed the fourteen selected originals for default fill, non-scrolling fill, and the two non-overscroll accessibility groups. The Rust target has eight passes and two intentional failures.

| Registrations | Result |
| --- | --- |
| :73, :100 | Default no-sibling height stays 600 at offsets 0, 50, -100, 0. With a preceding 100px box, both produce [500,550,400,500]. These are the originals' pumped-frame measurements, including the out-of-range jump. |
| :130, :261 | Viewport intersection and settled controller offsets match. Default mode reaches 150, leaving the preceding 150px box offscreen; hasScrollBody(false) settles back to 0 with both children visible. Rust checks bounds after virtual frames, not Flutter's test finder. |
| :284, :332 | Non-scrolling fill takes 450 vertical / 650 horizontal with no natural extent, and allows a larger footprint of 600 vertical / 1000 horizontal. |
| :303 | Partial match for the smaller bottom-aligned child: 450 vertical / 650 horizontal, centered at x=400 and bottom y=600 for vertical. The Rust target substitutes a same-sized neutral box for Flutter's helper button. |
| :349 | Partial match: a five-item preceding list still gives the nested child its 148px intrinsic extent, centered at x=400 with button analogue bottom at y=550. Rust moves to the range end programmatically, rather than repeating Flutter's drag. |
| :152, :444 | A fill starting at y=500 has a visible labeled semantics node in both modes on both frameworks. |
| :186, :479 | At y=600 (within 250px cache but outside viewport) Flutter's label remains mounted and hidden. Incular incorrectly retains it as visible, with physical geometry already verified offscreen. |
| :221, :515 | At y=850 (exact far cache boundary) Flutter removes the label. Incular still reports it as visible in both scroll-body modes. |

The node checks use real per-frame framework semantics, not native screen-reader output. Run cargo test --locked -p incular-runtime --test flutter350_sliver_fill_remaining_parity -- --nocapture.

## Four programmatic overscroll regressions (both axes and reverse)

All eight original Flutter iOS/macOS variant expansions for :600, :644, :685 and :744 passed. Rust selected Bouncing physics directly in the framework harness rather than presenting on either OS. The focused Rust binary has four intentional failures. Each case passed the exact initial box and returned to that same box on jump-to-zero.

| Registration | Flutter while overscrolled | Incular |
| --- | --- | --- |
| :600 150px preceding sibling, jump 200 | y=-50, height=650, bottom=600 | y=-50, height=450, bottom=400 |
| :644 lone child, jump 50 | y=-50, height=650, bottom=600 | y=-50, height=600, bottom=550 |
| :685 reversed vertical, jump 50 | y=0, height=650; (400,25) reaches fill child | y=50, height=600; point does not reach child |
| :744 reversed horizontal, jump 50 | x=0, width=850 | x=50, width=800 |

Run with cargo test --locked -p incular-runtime --test flutter350_sliver_fill_overscroll_parity -- --nocapture.

## Touch drag, flexible child, and clamping follow-up

All remaining thirteen original registrations and their named iOS/macOS variants passed on the pinned reference. Flutter's exact test names are in the ledger. Incular explicitly selects the corresponding physics and delivers a 50px synthetic touch swipe through Runtime's actual input path, then pumps monotonic frames. The focused Rust binary has three matches and six intentional failures. Its 116px-wide neutral button analogue and 100px logo analogue compare geometry, not the helper widgets or OS integrations.

- :385: flexible logo and bottom button geometry matches at rest; the 48px/40px button heights cover both reference density dimensions. The hardware platform theme is not involved.
- :555: a real bouncing drag reaches about +22.49px outside the range, but the expanding child remains 450px tall. Flutter stretches beyond 450px. Initial and settled height match.
- :793 and :958: the smaller bottom-aligned box and the flexible-column case fail to stretch. The bottom follows the offset to ~577.51 instead of staying at y=600. The logo and button keep their intended dimensions, and rest positions match after settling.
- :838: beyond five 150px rows, the padded child attains the same resting 148px/140px natural heights. During the drag it stays at that same height despite +22.49px overscroll; Flutter grows beyond 148px in both variants.
- :902 and :1283: partial mismatch for accessibility of the final list row alongside a zero-natural-size fill. Before scrolling, Flutter's finder does not expose label 4; Rust reports a visible node. Once the end is reached the label is visible on both. The zero-height paint-geometry observation is not equivalent to the Flutter element finder during overscroll; no exact presence claim is made there.
- :1034, :1081, :1125: with fillOverscroll enabled, inside / within-cache / outside-cache states should be visible / hidden / absent. Incular gives visible / visible / visible despite the exact physical y coordinates.
- :1175, :1198 and :1233: with clamping physics, the attempted swipe does not stretch the 450px fill or move its bottom-aligned small child; the padded 148px natural extent at the end of the long list also remains unchanged. These pass. The small button and padded center cases use neutral box analogues.

Run cargo test --locked -p incular-runtime --test flutter350_sliver_fill_drag_semantics -- --nocapture. All 31 original registrations in this file now have an explicit disposition, including partials. See earlier sections for programmatic/reverse overscroll and no-swipe visibility.
