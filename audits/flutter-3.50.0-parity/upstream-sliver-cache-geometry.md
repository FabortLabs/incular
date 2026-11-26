# Sliver cache geometry and eviction

Pinned Flutter and Incular's `flutter350_sliver_cache_geometry` verify
`rendering/sliver_cache_test.dart:13,251,325,399,473` using an 800×600
viewport and a 250 px cache. The native tests wrap production slivers,
record the constraints they actually receive and the geometry they
return, and check real retained element membership. The wrapper requests
layout on each tested offset so the test can inspect geometry even when
the normal cache-window fast path could reuse layout.

- Sequential 100 px box slivers make the start/cache boundaries at
  offsets 0, 50, 150 and 1000 observable, including a 50 px cache-only
  tail, a partially visible first child, and a fully off-cache child.
- Fixed and variable indexed lists have precisely 0–8, 0–8, 12–23 and
  21–29 mounted at offsets 0, 50, 1500 and 2400. The two-lane grid mounts
  exactly twice those counts, with the identical 850/900/1100/850 px
  cache extents. Only the 600 px paint window is reported visible.
- A 50 px pad on each side consumes separate paint and cache budgets.
  The outer and inner geometry match exactly for visible, cache-only and
  fully outside slivers at offsets 0, 50 and 5400. At the trailing end,
  the partly cached leading pad contributes 50 px and its inner box
  contributes zero.

These comparisons verify caching/eviction without asserting Flutter's
private debug strings or requiring either framework to perform extra
work while the mounted interval stays unchanged.

## Filling the remaining viewport: paint and cache are separate budgets

All six pinned cases at `rendering/sliver_cache_test.dart:756,858,1045,
1151,1346,1480` passed. The three additional native tests exercise both
preceding-sliver sizes (400 and 850 px) for each mode. Each checkpoint
compares the incoming cache origin/remaining paint/remaining cache/scroll
offset, the returned paint/cache extent and visibility, and the actual
constraints sent to the mounted child.

- A non-scrolling fill remains tightly sized to 200 px after the
  400 px box and 100 px after the 850 px box. It does not grow with
  trailing overscroll.
- A stretching fill grows its child maximum and reported paint extent
  by 50 px on trailing overscroll. Its minimum and logical cache extent
  remain 200 px or 100 px.
- A scrolling fill has a zero minimum and a 250 px maximum when
  cache-only, then fills the exact remaining paint budget once visible.
  When it occupies the whole viewport its cache contribution is 600 px,
  not the viewport's full available 850 px budget.
