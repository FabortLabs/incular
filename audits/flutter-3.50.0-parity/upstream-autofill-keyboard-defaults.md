# EditableText keyboard defaults from autofill hints

The three pinned non-Apple controls paired through an actual focused
Incular editor and its emitted native SetClient/Update commands.
A street-address hint chooses the address keyboard, an explicit
text keyboard wins, and an email OTP hint chooses text.
Only the first hint is used. An empty list preserves the normal
single/multiline distinction, and an explicit text keyboard on a
multiline editor stays text.

All six Flutter registrations ran, including both iOS and macOS
variants of the three Apple cases. The Apple keyboard table and
its precedence are tested directly in Rust, but only Linux host
attachment was available here, so the three Apple registrations
are partial. Other cases cover ordinary OTP, telephone, date,
decimal amount, and unknown hints in the typed policy.
