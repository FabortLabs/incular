# Resized image cache identity

Pinned ResizeImage controls at lines 46, 54, 151 and 169 compare
provider inequality/reflexivity and canonical resized key equality,
hash equality and separation by effective dimensions. The
Rust `key_for` uses the already-resolved source key plus physical
dimensions, policy and upscaling permission. At 2x, a 25-logical-pixel
request and a 50-physical-pixel request have equal keys. At 1x that same
logical request has a different key. The Flutter wrapper hash-code test
at line 25 is not counted: the Rust erased provider implements equality
but not a wrapper-level hash.

Runtime tests prove the same identities in the actual bundle cache.
Two equivalent requests share the same retained image ID and do not
reread the file. An 8x10 request and a 16x20 request stay distinct, and
neither caches the 50x50 source. The source only enters the cache after
a separate unresized request. Decoded scale still maps pixels to logical
image size; there is no extra scaling just because cache dimensions are
logical. A callback-only transport remains asynchronous.

The bundle cache retains only the requested decoded size. This is not
evidence that the native decoder avoids a temporary full-size source
buffer, or that memory/custom providers share the same global cache.
