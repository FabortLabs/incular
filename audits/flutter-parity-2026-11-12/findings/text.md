# text: Text and editing

## Summary

The retained RichText path discards the styles of all flattened runs after the first. This finding is based on source inspection; the rest of the text package has not yet been audited.

## Coverage

| Flutter item | Incular counterpart | Method | Scenarios | Result |
| --- | --- | --- | --- | --- |
| RichText, TextSpan styling | incular_text::RichText and Widget conversion | reading | none | divergent |
| Other text and editing APIs | various | not checked | none | not checked: audit incomplete |

## Findings

### text-001 — RichText drops styles after the first flattened run

- Severity: P1
- Category: paint
- Flutter: `painting/text_span.dart:286` and `painting/text_span.dart:315` — each span pushes its own style and recursively builds its children into the paragraph.
- Incular: `crates/incular-widgets/src/tree/descriptors.rs:656` — conversion selects only the first flattened run's style and passes all plain text to a single text widget; `crates/incular-text/src/spans.rs:322` does the same in direct layout.
- Evidence: code reading of the references above.
- Impact: a paragraph containing a red first span and a blue second span paints both in the first span's style; different font sizes also change layout and wrapping.
- Likely cause: span flattening preserves runs, but the retained text descriptor and layout call collapse them to one style.
- Fix direction: shape and paint each styled run while retaining paragraph-wide layout and selection.
- Confidence: high

## Pixel scene proposals

Not prepared; the text scene would require matching red and blue spans and identical font settings.

## Harness requests

None.

## Open questions

The remaining text, editing and selection behavior needs independent review.
