# Disposable listenable builders

Six pinned originals in `widgets/transitions_test.dart:1102,1132,1168,
1194,1224,1260` passed. Runtime frames verify that both animation and
listenable builders rebuild once per notification, not on idle pumps,
and leave the already-built pass-through child alone. The controller and
FocusNode zero-area cases both complete at exactly zero size.

The existing Rust constructor for animation sources is unchanged;
`AnimatedBuilder::from_listenable` accepts a non-animation listenable
such as ChangeNotifier. Both forms use the same retained builder path.
The notifier's framework tracking is lazy and drops on disposal; a
builder removing or replacing its source drops the old registration.

The Rust baseline lacked these listener types. After adding them, the
new tests also reproduced missing frame demand until the runtime
included its retained dependency tracker. The authored pairs verify
replacement/unmount and a post-frame notification requesting another
frame, without prematurely rebuilding in the current frame. Thirty-one
focused Rust checks passed.
