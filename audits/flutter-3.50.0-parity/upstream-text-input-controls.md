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

## Operation-specific host error reporting

Twelve more pinned services originals pair through the outbound
adapter. A failed attach, detach, config/state update, show, hide,
editable-size transform, composing/caret/selection geometry, or
autofill request/finish reports one typed platform exception with
the original message and Flutter operation context. Neighboring
successful requests report nothing. Finalizing autofill does not
require an editor to remain attached. The style-specific failure
was already verified separately. Twelve native error tests and
seven existing control tests passed.
