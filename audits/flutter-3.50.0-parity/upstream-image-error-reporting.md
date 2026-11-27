# Unhandled image failure reporting

Both pinned `painting/image_stream_test.dart` cases at :1184 and
:1211 passed. A default stream listener that is later dropped does
not hide the subsequent error from the observed reporter. A
listener that opts out makes the suppression permanent, even when
it unregisters before the failure. The terminal typed error remains
available; the completion is not replayed. The grouped listener's
identity includes the reporting option.

The native tests use a scoped reporter on the failing completer,
rather than a process-wide logger. A live grouped handler receives
the actual error and prevents a duplicate unhandled report. A
pending completer handoff preserves remembered suppression on the
source that will actually complete the stream. All 17 focused image
stream, handoff, listener, and progress tests passed. This verifies
the one-shot provider stream, not reporting from animated codecs or
the global Flutter exception hook.

## ProviderImage fallback across unmount

Both pinned `widgets/image_test.dart` cases at :3026 and :3082
passed. A mounted provider image with an error builder remembers
that policy on its actual pending stream. Dropping the image and,
in the second case, first removing only its builder while retaining
the same provider, does not reenable reporting when the source
later fails. The mounted native controls previously reported that
failure in both cases. A provider image that never installed a
builder still reports once. All 26 focused provider, stream,
and scroll-aware image tests passed.
