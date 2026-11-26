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
