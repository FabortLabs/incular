# EditableText outside pointer actions

## An outside release keeps its press owner

The pinned authored `editable_outside_release_owner_test.dart` drives an
actual Linux mouse press on a hit-testable sibling. The default action
blurs the editor immediately; its release callback must still run once.
A press that began while the editor was unfocused cannot acquire the
release callback by gaining focus afterwards. Both observations
originally failed in Incular, in opposite ways.

The runtime now records the original focused outside editor in the
pointer's captured route and consumes it once on release. Group
membership is determined at press time, and a canceled route or a
removed editor cannot receive a late callback. This replaces the
previous release-time lookup of the currently focused field. Existing
same-group, explicit callback, captured-release, and native focus tests
pass. The authored comparison adds no original registration.
