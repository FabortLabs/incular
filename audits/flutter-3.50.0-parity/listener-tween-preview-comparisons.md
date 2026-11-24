# Foundation listeners, animation tweens, and preview composition

Flutter 3.50.0 `ab2912035e` vs Incular framework `c647cb85`. Focused
headless tests, not an exhaustive library verdict.

| ID | Case | Flutter | Incular |
| --- | --- | --- | --- |
| C21 | SizeTween with absent start and end (10,20), at t=0 | null | Some(Size(0,0)) |
| C22 | IntTween 16,777,217→16,777,219 at t=0.5 | 16,777,218 | 16,777,216 |
| Positive control | Listener A removes B and adds C in the first notification | [A]; second cumulative [A,A,C] | [A]; second cumulative [A,A,C] |
| Preview reference only | Empty builder group and two added wrappers | Default; newest wrapper outside old | No equivalent preview builder |
| Preview reference only | MultiPreviewThemeData with first and second theme | First is outermost | No equivalent preview theme API |

Tween controls agree for t=0.5 and 1 with the present size and for the
small integer midpoint 2→4. Flutter's `Tween.transform` returns exact
endpoints before interpolation, while Incular `OptionalTween.evaluate`
always calls lerp. Float conversion in Incular loses the odd integer before
rounding. Sources: Flutter `animation/tween.dart:359–369,426–435,473–485`;
Incular `crates/incular-animation/src/tween.rs:38–41,138–170`.

The notifier comparator is Flutter's re-exported ChangeNotifier vs
Incular's DependencySource. Removing B is explicit vs dropping the token.
Incular snapshots observers but checks active at delivery
(`crates/incular-core/src/reactivity.rs:66–90`), so this case passes.
It does not establish notifier API or async stream parity; the latter has
a separate delivery implementation and separately verified failures.

Preview tests execute the reference: `PreviewBuilder.addWrapper` composes
new(old(child)), group defaults to Default, and MultiPreviewThemeData
iterates in reverse so the first item is outermost. Sources:
Flutter `widget_previews/widget_previews.dart:294–324,405–421`.
Incular's public neutral crates/tools lack those application preview
contracts; `tools/incular-cli/src/main.rs:66–88` exposes build/run
and screenshots, not preview scan, annotations, host/reload or wrappers.
These are reference-verified API gaps, **not** additional paired Rust
runtime failures. Full discovery/scaffold cases remain open.

## Fixtures

Use the pinned scratch Flutter package from [paired comparisons](focused-comparisons.md).
Run the two narrow targets:
`flutter test --no-pub test/foundation_animation_slice_test.dart` and
`flutter test --no-pub test/preview_slice_test.dart`.
All five Flutter checks passed.

`test/foundation_animation_slice_test.dart`:

```dart
import 'dart:ui' show Size;
import 'package:flutter/animation.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
void main(){
  test('notifier remove before turn, add waits for next notification',(){
    final n=ChangeNotifier(), calls=<String>[];
    void b()=>calls.add('B');
    void c()=>calls.add('C');
    var first=true;
    n.addListener((){
      calls.add('A');
      if(first){first=false; n.removeListener(b); n.addListener(c);}
    });
    n.addListener(b);
    n.notifyListeners();
    expect(calls,['A']);
    n.notifyListeners();
    expect(calls,['A','A','C']);
    n.dispose();
  });
  test('nullable size tween preserves the missing endpoint',(){
    final tween=SizeTween(end:const Size(10,20));
    expect(tween.transform(.5),const Size(5,10));
    expect(tween.transform(1),const Size(10,20));
    expect(tween.transform(0),isNull);
  });
  test('integer tween keeps midpoints around the f32 precision boundary',(){
    final ordinary=IntTween(begin:2,end:4);
    expect(ordinary.transform(.5),3);
    final tween=IntTween(begin:16777217,end:16777219);
    expect(tween.transform(.5),16777218);
  });
}
```

`test/preview_slice_test.dart`:

```dart
import 'package:flutter/widget_previews.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
final class MarkTheme extends PreviewThemeData {
  const MarkTheme(this.name);
  final String name;
  @override Widget apply(BuildContext context,Widget child)=>KeyedSubtree(key:ValueKey(name),child:child);
}
void main(){
  test('preview wrapper newest outermost and initial group',(){
    final builder=PreviewBuilder();
    builder.addWrapper((child)=>KeyedSubtree(key:const ValueKey('old'),child:child));
    builder.addWrapper((child)=>KeyedSubtree(key:const ValueKey('new'),child:child));
    final preview=builder.build();
    expect(preview.group,'Default');
    const leaf=SizedBox();
    final outer=preview.wrapper!(leaf) as KeyedSubtree;
    expect(outer.key,const ValueKey('new'));
    expect(outer.child.key,const ValueKey('old'));
    expect((outer.child as KeyedSubtree).child,same(leaf));
  });
  testWidgets('first preview theme is outermost',(t)async{
    const leaf=SizedBox();
    Widget? result;
    await t.pumpWidget(Builder(builder:(context){
      result=const MultiPreviewThemeData([MarkTheme('first'),MarkTheme('second')]).apply(context,leaf);
      return result!;
    }));
    final outer=result! as KeyedSubtree;
    expect(outer.key,const ValueKey('first'));
    expect(outer.child.key,const ValueKey('second'));
    expect((outer.child as KeyedSubtree).child,same(leaf));
  });
}
```

Temporary Rust animation target `crates/incular-animation/tests/audit_tween_slice_tmp.rs`;
`cargo test --locked -p incular-animation --test audit_tween_slice_tmp`.
Both parity assertions failed after their controls.

```rust
use incular_animation::{OptionalSizeTween,TweenValue};
use incular_core::Size;
#[test]
fn nullable_size_tween_preserves_the_missing_endpoint(){
    let tween=OptionalSizeTween::new(None,Some(Size::new(10.,20.)));
    assert_eq!(tween.evaluate(0.5),Some(Size::new(5.,10.)));
    assert_eq!(tween.evaluate(1.),Some(Size::new(10.,20.)));
    assert_eq!(tween.evaluate(0.),None);
}
#[test]
fn integer_tween_keeps_midpoints_near_f32_boundary(){
    assert_eq!(TweenValue::new(2i32,4).evaluate(0.5),3);
    assert_eq!(TweenValue::new(16_777_217i32,16_777_219).evaluate(0.5),16_777_218);
}
```

Temporary Rust foundation target `crates/incular-core/tests/audit_listener_slice_tmp.rs`;
`cargo test --locked -p incular-core --test audit_listener_slice_tmp`.
The exact listener case passed.

```rust
use incular_core::reactivity::DependencySource;
use std::{cell::{Cell,RefCell},rc::Rc};
#[test]
fn notification_removes_before_turn_and_defers_new_listener(){
    let source=DependencySource::default();
    let calls=Rc::new(RefCell::new(Vec::new()));
    let b=Rc::new(RefCell::new(None));
    let c=Rc::new(RefCell::new(None));
    let first=Cell::new(true);
    let removal=b.clone();
    let dest=c.clone();
    let a_calls=calls.clone();
    let inside=source.clone();
    let _a=source.subscribe((),move||{
        a_calls.borrow_mut().push("A");
        if first.replace(false){
            removal.borrow_mut().take();
            let c_calls=a_calls.clone();
            *dest.borrow_mut()=Some(inside.subscribe((),move||c_calls.borrow_mut().push("C")));
        }
    });
    let b_calls=calls.clone();
    *b.borrow_mut()=Some(source.subscribe((),move||b_calls.borrow_mut().push("B")));
    source.notify();
    assert_eq!(*calls.borrow(),["A"]);
    source.notify();
    assert_eq!(*calls.borrow(),["A","A","C"]);
}
```

Temporary Rust test targets were removed; only audit evidence is tracked.
