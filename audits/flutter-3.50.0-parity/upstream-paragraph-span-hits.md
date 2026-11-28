# Retained paragraph span hits and locale

Four exact pinned `rendering/paragraph_test.dart` registrations
passed.

- :607 switches the locale from zh-HK to ja-JP and reads the
  resolved setting from the same retained paragraph.
- :702 selects across two glyphs and seven real 14 × 14 inline
  children under a 50 px limit. All nine boxes and their directions
  match across the three wrapped lines; adjacent inline children
  remain separate boxes.
- :894 hits the first and last A, the B and C, and the large
  letter-spaced gap between them. The off-box and distant points
  return no span.
- :989 hits the A span even in the middle of its justified
  whitespace, and returns the B span at x=95.

The same retained hit test now exposes unannotated literal spans;
mouse dispatch still checks for a cursor or callback annotation.
Synthetic ellipses cannot inherit the range of truncated source.
The native run executed 33 focused tests, including real mouse
enter/exit and cursor routing. No new text-span gesture recognizer
or assistive action behavior is claimed by these comparisons.
