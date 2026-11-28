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
