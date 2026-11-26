# Decoded image size and one-pixel decode

The pinned `painting/image_info_test.dart:15` original passed.
Incular's public ImageHandle owns the decoded resource and logical
scale, so it now exposes size_bytes on that handle. It measures
decoded RGBA8 storage, not logical dimensions, encoded bytes, or
GPU allocations. The focused native checks match 800, 8,000, and
80,000 bytes, and cloning a handle preserves its reported size
without copying pixels. The cache accounting uses the same accessor.

Pinned `painting/image_decoder_test.dart:16` passed. The native
public decode path returns a transparent 1×1 RGBA8 image with four
bytes. That original remains partial: Flutter also verifies the
overridable PaintingBinding codec hook is called exactly once;
Incular's direct decoder does not provide that binding hook.

Seventeen focused native public API and bundled cache tests passed.
