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
