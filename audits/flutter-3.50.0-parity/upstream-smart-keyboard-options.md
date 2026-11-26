# EditableText smart punctuation and inline prediction

All five pinned widget registrations and the pinned service serializer
registration ran. A mounted, ungrouped Incular editor emits the ordinary
`TextInput.setClient` message with no scoped field list. The wire carries
smart dashes and smart quotes as string indices. Both default on for an
ordinary editor and off when obscured; explicit policies still override
the default and can be set independently.

Inline prediction carries true, false, or null, including null for the
default editor. The typed native client options and serialized Flutter
method arguments match those values. This comparison covers the engine
configuration boundary, not device-side UIKit prediction or substitution.
