# Mounted EditableText replacement action

The authored reference probe goes through Flutter's widget Actions and
`ReplaceTextIntent`, not directly through the value helper. The focused
Incular tests call the mounted runtime action and inspect the controller,
focus, formatter, and text-change callbacks. Equal UTF-16 lengths keep
marked text and selection affinity/direction. Resizing moves both
selection and composition endpoints and resets affinity/direction, while
preserving the order of base and extent.

An identical replacement still requests internal focus but is not a
new formatter invocation or text-change notification. On desktop, this
internal focus must not select the whole field. The native regression
failed on that select-all behavior before the focus fix. Invalid UTF-8
byte ranges and read-only fields are rejected without editing or taking
focus on the Incular public action.

These probes are additional to the upstream inventory. They cover the
shared editable action, not Material or Cupertino toolbar behavior.
