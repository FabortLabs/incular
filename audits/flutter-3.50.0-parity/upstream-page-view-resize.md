# PageView and PageController: resizing and deferred state

The first five exact originals from widgets/page_view_test.dart passed on the pinned Flutter 3.50.0 checkout. The exact-name ordinary control also included the selected next/previous and zero-size-container originals; all four reported ordinary cases passed in its raw log, and the omitted overscroll variant names were run separately. The focused Rust resize target has three matches and three intentional failures.

| Registration | Evidence |
| --- | --- |
| :32 | Reference only. Flutter's builder exposes findChildIndexCallback across item-count updates; the Rust PageView builder offers item_count/builder with no corresponding callback. |
| :65 | Initial page 1 becomes visible and a normal jump shows page 14. Flutter restores page 14 after a second collapse/expand; Incular restores page 0. Rust also lacks the initial mounted-but-offstage page 0, and both have no visible content at zero area. |
| :111 | While the viewport is zero the logical page getter returns 14 after animateToPage and 12 after jumpToPage in Flutter. Incular gives None twice and has no mounted initial page. Its queued last request still works when the viewport expands: page 12 becomes visible and is reported by the controller. |
| :158 | Partial match: an expanding page viewport survives a zero to 500×400 host constraint change without throwing and is correctly sized. This does not reproduce Flutter's test-binding MediaQuery update. |
| :196 | Partial match: both APIs decline to return a page number while unattached. Flutter raises a debug assertion and Rust returns None; Rust's Option protocol is tested separately from the attached-zero mismatch above. |
| :310 | Starting page 4, next-page animation to 5, resizing from 600px to 300px, and previous-page animation to 4 produce [4,5,5,4] as in Flutter. Actual frame-driven controller animations are pumped with monotonic timestamps. |
| :456 | Incular mounts no page 0 child at zero size, where Flutter has it mounted offstage. After expanding to 200×200 both show page 0. |

Run cargo test --locked -p incular-runtime --test flutter350_upstream_page_resize -- --nocapture. This verifies framework geometry and attachment, not an OS surface bridge.

## Cache the logical page at zero extent

The repaired Rust target now passes the original `:65`, `:111`, and
`:456` behaviors. A finite zero extent keeps one zero-sized, offstage
child; it is not the unbounded-viewport fallback. A mounted controller
caches the logical page when collapsing, exposes updates from zero-width
jump/animate calls immediately, and converts the last page back to
pixels on expansion. An unattached controller still does not invent a
page through the zero-extent getter.

An authored paired control uses snapping off and fraction 1.25. The
pinned Flutter behavior is subtle: 425px is page 1.6 in a 200px viewport,
but on collapse the centering origin becomes zero before Flutter
decodes the old pixels, so it caches page 1.7. Expanding to 300px
restores that cached page at 675px. The renderer and controller agree,
and no extra page-changed notification fires in this sequence.
