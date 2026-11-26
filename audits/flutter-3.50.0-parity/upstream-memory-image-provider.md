# In-memory image provider cache

Pinned `painting/image_provider_and_image_cache_test.dart:70` and `:85`
passed by exact name. The native tests decode a real PNG and resolve it with
two providers sharing the same bytes. A warm resolve reuses the decoded
identity. Eviction returns false for an absent key; after successful eviction
a cold resolve creates a different identity with identical pixels while
previously returned handles remain valid.

Resolving and evicting through a supplied cache leaves the shared cache
warm. The fixture checks both cache byte accounting and unchanged shared
identity. A host `flutter/system` memory warning clears the default cache
without invalidating an already delivered stream; a different channel does
not clear it. All 33 focused cache, image-stream and host pressure tests passed.

Incular retains its existing byte-content key semantics and separate bounded
memory and bundle caches; Flutter uses memory-buffer identity and a combined
cache. The pinned cases reuse the same buffer and do not compare mixed-source
cache budgets, resize-key eviction or live multi-frame images.

## Listening for a provider failure

Pinned `painting/image_provider_and_image_cache_test.dart:115` passed.
A failing Rust provider returns its typed decoding error from the actual
resolution path. A caller that attaches a frame listener after resolve
still receives the original error exactly once. Native controls follow an
asynchronously rebound stream, ignore chunk progress, cancel listeners on
drop, distinguish cached from delayed success, and release decoded pixels
after a completed listener no longer needs them. All 20 focused stream
and bundle-cache checks passed.

This is a first-outcome listener for Incular's single-frame stream.
It does not claim Flutter's animated-frame or uncaught-error reporting APIs.
