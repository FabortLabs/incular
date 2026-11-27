# Native and custom text input controls

Five pinned services TextInputControl registrations pair. The optional
connection manager gives the custom keyboard the original input
type/configuration, sends the host the disabled-system-keyboard
configuration, and dispatches attach, state, show, geometry, style,
detach and deferred hide in their original order. Native focus/input
adapters remain separate; this manager is for hosts using Flutter's
wire protocol.

A platform editing update reaches the focused client and the custom
control once, with no echo to the platform. Replacing or hiding the
custom control reports its old and new identities once. Geometry is
deduplicated per attachment. A deferred hide cannot dismiss a newer
attachment. The browser variant serializes `isMultiline`; this is
verified by a native wire assertion without counting the web-only
original. Seven native control, ten configuration and six style
tests passed.
