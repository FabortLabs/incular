# TextStyle debug provenance

The pinned original `painting/text_style_test.dart:433` and the native
tests agree on the empty and labelled descriptions and on provenance
through copy, merge, apply, and lerp, including nested and unlabeled
cases. A label-only change keeps style equality and does not request
layout or paint. Provenance is synthesized in debug builds only.
These checks cover the original label formats, not arbitrary diagnostic
renderings of every paint or decoration.

## Hash semantics

The pinned original `painting/text_style_test.dart:458` passes. Native
hashes are stable for an unchanged style and distinguish the two font
fallbacks and two leading distributions in the original. Signed zero
and a different debug label do not change the hash of an equal style.
The integer is produced by Rust's caller-provided hasher.
