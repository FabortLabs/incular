# SynchronousFuture continuation timing

The pinned `foundation/synchronous_future_test.dart`:11 registration
passed. The native core type calls an already-available continuation
inline; its timeout wrapper yields one scheduler turn before
returning the already available value, and its stream emits the same
value once and closes.

Completion actions start immediately, but an asynchronous completion
does not replace the original value. A construction-time or later
error is delivered as a Rust `Result`. Rust handles are single-use
and can be cloned for multiple listeners; the ordinary callback and
async-returning callback have separate methods. Two focused native
tests passed.
