# Upstream numeric helpers and tolerance diagnostics

Flutter 3.50.0 `ab2912035e`, Incular `c647cb85`. Unmodified pinned
`near_equal_test.dart`, `utils_test.dart`, and `tolerance_test.dart`
passed all four targeted upstream registrations. Rust passed the
three executable translations.

| File:line | Outcome |
| --- | --- |
| `near_equal_test.dart:9` | Match: symmetric near/far bounds. |
| `near_equal_test.dart:16` | Flutter-only: exact null cases passed; Rust near_equal takes plain f32, with no nullable equivalent. Not counted as paired. |
| `utils_test.dart:9` | Match: infinities, close/far values, and equality. |
| `tolerance_test.dart:9` | Match for a compact diagnostic (Rust Debug, not a promise of identical text or Display API). |

No wrapper was written to fake the missing nullable API. Flutter's
one-line diagnostic checks non-empty, no newline, no fallback
instance string, no edge whitespace (`flutter_test/lib/src/matchers.dart:1532–1542`).
Incular numeric helper and Tolerance definitions:
`crates/incular-animation/src/physics.rs:10–56`.

Run the three named reference files separately. For Rust use
`cargo test --locked -p incular-animation --test audit_upstream_near_tmp upstream_ -- --nocapture`.
After tightening the diagnostic matcher, reran the exact tolerance test.
Temporary test source removed.

```rust
use incular_animation::physics::{near_equal,Tolerance};
#[test] fn upstream_near_friction(){
    assert!(near_equal(5.,6.,2.));
    assert!(near_equal(6.,5.,2.));
    assert!(!near_equal(5.,6.,0.5));
    assert!(!near_equal(6.,5.,0.5));
}
#[test] fn upstream_near_equals(){
    assert!(near_equal(f32::INFINITY,f32::INFINITY,0.1));
    assert!(near_equal(f32::NEG_INFINITY,f32::NEG_INFINITY,0.1));
    assert!(!near_equal(f32::INFINITY,f32::NEG_INFINITY,0.1));
    assert!(!near_equal(0.1,0.11,0.001));
    assert!(near_equal(0.1,0.11,0.1));
    assert!(near_equal(0.1,0.1,0.0000001));
}
#[test] fn upstream_tolerance_description(){
    let s=format!("{:?}",Tolerance::default());
    assert!(!s.contains(['\r','\n']));
    assert!(!s.is_empty());
    assert!(!s.contains("Instance of "));
    assert_eq!(s.trim(),s);
}
```
