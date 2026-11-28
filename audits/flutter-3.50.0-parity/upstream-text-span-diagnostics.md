# Text span diagnostics and equality

The exact pinned `painting/text_span_test.dart` cases at lines 13, 49,
and 75 passed. Separately built text trees compare recursively. Mouse
annotations compare callback identity, not callback code or only which
callbacks exist; cursor changes are distinct. The new diagnostic snapshot
uses the Foundation whitespace tree renderer. Exact output checks inherited
style values, quoted literal runs, empty descendants, callback order, and
the forbidden mouse cursor description. These debug strings do not change
shaping, painting, focus, hit testing, or semantics. The three focused
diagnostic cases and four pre-existing span cases passed.
