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
