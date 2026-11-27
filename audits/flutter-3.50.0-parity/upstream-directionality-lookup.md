# Directionality lookup and unchanged providers

All four exact pinned `directionality_test.dart` cases passed
(:9, :29, :43 and :59). Public `maybe_of` returns only the closest
ancestor's direction, with no implicit LTR. `of` requires a provider
and reports the missing-provider diagnostic on failure.

The mounted sequence caught a native rebuild when a newly created
direction provider carried the same LTR value. The retained tree
now reuses the existing inherited value in that case. The same
consumer runs once for the initial build and once per real
direction change, including RTL back to LTR. Four focused and two
adjacent native layout tests passed.
