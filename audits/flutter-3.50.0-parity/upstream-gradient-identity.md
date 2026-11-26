# Gradient interpolation identity

Pinned `painting/gradient_test.dart` registrations 177, 536 and 794
pass for linear, radial and sweep gradients. The Rust borrowing API
returns the original descriptor for two references to that object,
and returns no descriptor for two absent inputs. This matches the
observable identity guarantee rather than merely comparing two copies.

Five native tests passed. Distinct descriptors still combine stop
positions (even at t=0), a single descriptor fades from/to transparent,
and retargeting an aligned shape using the same shader keeps the
existing shared handle. Four existing directional and shape-gradient
cases also passed. The existing owned API remains available.
