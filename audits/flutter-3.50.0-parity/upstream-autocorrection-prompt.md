# Native autocorrection prompt highlight

The pinned editable-text on-demand highlight registration pairs. An
opted-in editor paints the requested UTF-16 range in the configured
color without changing its selection. There is no highlight initially
or without a configured color. The highlight is removed when text
changes, focus is lost, or the editing controller is replaced. A stale
client or an offset inside a surrogate pair cannot alter the prompt
for the active field.

The services :1056 original also passed against a fake client. Incular
verified the equivalent channel on a mounted editor; its bare
fake-client callback (which does not validate or paint the range) is
still partial. Three focused prompt tests and four neighboring
toolbar tests passed.
