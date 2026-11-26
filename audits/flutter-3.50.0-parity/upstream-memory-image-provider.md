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
