# Paired slice: loose layout sizing and baseline offsets

Flutter 3.50.0 `ab2912035e` and Incular `c647cb85`, headless geometry
assertions. All Flutter cases passed, all Incular assertions failed.
These are deliberately different constraints from the frame cases that
matched in the retained harness.

| ID | Case | Flutter | Incular |
| --- | --- | --- | --- |
| C31 | Default OverflowBox, loose max 100×80, 20×10 child | Parent 100×80, child at (40,35) | Parent 20×10, child at (0,0) |
| C32 | Default contain FittedBox, loose max 100×100, 200×100 child | Parent 100×50 | Parent 100×100 |
| C33 | Baseline 5, child without natural baseline, child 20×20 | Parent 20×5, child at y=-15 | Parent 20×20, child at y=0 |

Flutter default OverflowBox fits the largest allowed size rather than
fitting to the child (`rendering/shifted_box.dart:690–767`). Incular
constrains child size for its own size
(`crates/incular-widgets/src/tree/layout/behavior/core.rs:163–195`).
Flutter's FittedBox preserves aspect under containment
(`rendering/proxy_box.dart:2884–2931`); Incular clamps axes independently
before choosing the fit transform (`tree/layout/behavior/effects.rs:39–51`).
Flutter's Baseline allows a negative top when the requested baseline is
above the child's natural/fallback baseline
(`rendering/shifted_box.dart:1556–1570,1595–1603`); Incular clamps to zero
(`crates/incular-layout/src/algorithms.rs:584–599`).
Remaining: unbounded axes, custom alignment and fit, tight constraints,
actual text baseline variants, possible implications for paint overflow.

## Reproduction

Use the pinned Flutter scratch package from [paired comparisons](focused-comparisons.md).
Run `flutter test --no-pub test/loose_layout_slice_test.dart`:

```dart
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
Widget looseFrame(Widget child,double width,double height)=>
  Directionality(textDirection:TextDirection.ltr,child:Center(child:SizedBox(width:width,height:height,
    child:Align(alignment:Alignment.topLeft,child:child))));
void main(){
  testWidgets('default OverflowBox fills a loose bounded parent',(t) async{
    const p=ValueKey('p'),c=ValueKey('c');
    await t.pumpWidget(looseFrame(const OverflowBox(key:p,child:SizedBox(key:c,width:20,height:10)),100,80));
    expect(t.getSize(find.byKey(p)),const Size(100,80));
    expect(t.getTopLeft(find.byKey(c))-t.getTopLeft(find.byKey(p)),const Offset(40,35));
  });
  testWidgets('loose Contain FittedBox preserves aspect ratio',(t) async{
    const p=ValueKey('p');
    await t.pumpWidget(looseFrame(const FittedBox(key:p,child:SizedBox(width:200,height:100)),100,100));
    expect(t.getSize(find.byKey(p)),const Size(100,50));
  });
  testWidgets('baseline can place child above parent',(t) async{
    const p=ValueKey('p'),c=ValueKey('c');
    await t.pumpWidget(looseFrame(const Baseline(key:p,baseline:5,baselineType:TextBaseline.alphabetic,
      child:SizedBox(key:c,width:20,height:20)),100,80));
    expect(t.getSize(find.byKey(p)),const Size(20,5));
    expect(t.getTopLeft(find.byKey(c))-t.getTopLeft(find.byKey(p)),const Offset(0,-15));
  });
}
```

Run only temporary target `crates/incular-widgets/tests/audit_loose_layout_tmp.rs`
with `cargo test --locked -p incular-widgets --test audit_loose_layout_tmp`.
The temporary target was removed. Full logs are in `.tmp/flutter350-full-audit/`.

```rust
use incular_config::Constraints;
use incular_core::{Offset,Size};
use incular_widgets::{Baseline,FittedBox,OverflowBox,SizedBox,Widget,internal::{Key,WidgetTree}};
fn measure(build:impl FnOnce(Widget)->Widget,child_size:Size,max:Size)->(Size,Offset){
    let key=Key::from(3310_u64);
    let child=Widget::from(SizedBox::new().width(child_size.width).height(child_size.height)).with_key(key.clone());
    let mut tree=WidgetTree::new();
    let root=tree.mount(build(child)).unwrap();
    tree.layout(Constraints::loose(max)).unwrap();
    let _=tree.paint();
    let parent=tree.element_bounds(root).unwrap();
    let child=tree.element_bounds(tree.element_with_key(&key).unwrap()).unwrap();
    (parent.size,child.origin-parent.origin)
}
#[test]
fn default_overflow_box_fills_loose_bounded_parent(){
    let actual=measure(|w|OverflowBox::new(w).into(),Size::new(20.,10.),Size::new(100.,80.));
    assert_eq!(actual,(Size::new(100.,80.),Offset::new(40.,35.)));
}
#[test]
fn loose_contain_fitted_box_preserves_aspect_ratio(){
    let actual=measure(|w|FittedBox::new(w).into(),Size::new(200.,100.),Size::new(100.,100.));
    assert_eq!(actual.0,Size::new(100.,50.));
}
#[test]
fn baseline_can_place_child_above_parent(){
    let actual=measure(|w|Baseline::new(5.,w).into(),Size::new(20.,20.),Size::new(100.,80.));
    assert_eq!(actual,(Size::new(20.,5.),Offset::new(0.,-15.)));
}
```
