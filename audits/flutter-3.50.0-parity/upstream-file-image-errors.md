# FileImage distinguishes empty files and retries the path

Reference: Flutter 3.50.0 at `ab2912035e160d9cad6306df307576da6128ebdb`.

A readable zero-byte file now produces `ImageError::EmptyFile`, with its
original path, before decode. It reaches both stream listeners and
`ResizeImage` without publishing a frame. A missing file still produces an
I/O error, and nonempty invalid bytes still produce a decode error.

An authored pinned Flutter probe observes a pending key, then its eviction
on an empty-file error. It writes a valid 8x8 PNG to the same path and the
same provider decodes it successfully at scale 2. The native test uses the
same PNG and validates the failure, later new stream, decoded pixels'
dimensions, scale, and path. The original stream remains failed.

The three pinned originals (65, 87, 155), the authored pinned recovery
probe, and all 10 focused native tests passed. The originals remain
partial: native `FileImage` still resolves synchronously and does not
publish Flutter's pending cache transition or retain the pending entry on
a nonempty decoder failure. The recovery result is verified without
claiming either cache policy.

## Native runtime pending and eviction

Mounted native `FileImage` now uses the existing host blocking executor.
The bounded decoded-image cache tracks the exact path, scale, and optional
resize key. Lookups share the first pending job. The job is submitted
after the pending entry is observable; a canceled worker job does not
leave a stuck key or a hung stream.

A readable empty file leaves the pending and live cache maps before its
observer receives the error, and a later mount paints the real 8x8 image
after it is written. A nonempty invalid file reports “Invalid image data”
and keeps the failed pending key until eviction, even if the file is
subsequently replaced. The resized route retains the original empty-file
failure and can later decode at 4x4 without poisoning the original key.
These tests distinguish failure classification, key lifetime, and what
reaches the mounted widget, rather than inferring it from API names.

The existing pinned originals at 65 and 155 now have full native matches.
Line 87 stays partial because Flutter's pluggable low-level decoder at
`loadBuffer` is not the native direct `load` or stream API. Explicit
headless file loads remain synchronous. The existing authored Flutter
recovery probe, 52 focused image-crate tests, and five focused runtime
tests provide the evidence. No renderer changes.
