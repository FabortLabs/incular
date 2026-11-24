# Paired slice: path hit testing and horizontal direction

Flutter 3.50.0 `ab2912035e` vs Incular framework `c647cb85`. No native
assistive-technology or device-specific scroll sampling was included.

## C26: custom path clips hit testing

Both fixtures lay out a 100×100 input target and clip it to the triangle
(0,0)→(100,0)→(0,100). Both deliver a tap at (20,20). Flutter rejects
a tap at (80,80) (inside layout rect, outside path) and remains at one tap.
Incular delivers it (two taps). Flutter tests the custom path before
delegating hits (`rendering/proxy_box.dart:2011–2020`); Incular's ClipPath
explicitly treats clipping as raster-only
(`crates/incular-widgets/src/layout/basic/clipping.rs:264–268`).
This does not claim that all Flutter clips alter hits or that Incular's
visual path failed.

## C27: scroll axis uses ambient text direction

Neutral horizontal SingleChildScrollView at offset 0, 100 px viewport,
200 px content. Values are child x relative to viewport x:

| Direction | reverse | Flutter | Incular |
| --- | --- | ---: | ---: |
| LTR | false | 0 | 0 |
| LTR | true | -100 | -100 |
| RTL | true | 0 | -100 |
| RTL | false | -100 | 0 |

Flutter sets positive direction from Directionality then flips by reverse
(`widgets/basic.dart:4567–4585`); Incular passes the configured reverse
without direction (`crates/incular-widgets/src/scrolling/scroll_views.rs:143–173`).
This result covers the single-child viewport only; list/grid/page, scroll-to
operations and restored/controller positions remain pending.

## Runnable scratch fixtures

Pinned Flutter scratch package, only the named test file per command:
`flutter test --no-pub test/clip_hit_slice_test.dart` and
`flutter test --no-pub test/scroll_rtl_slice_test.dart`. Both passed.

```dart
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
class Triangle extends CustomClipper<Path> {
  const Triangle();
  @override Path getClip(Size s)=>Path()..moveTo(0,0)..lineTo(s.width,0)..lineTo(0,s.height)..close();
  @override bool shouldReclip(Triangle old)=>false;
}
void main(){
  testWidgets('custom clip rejects a hit in the box but outside the path',(t)async{
    const key=ValueKey('box');
    var taps=0;
    await t.pumpWidget(Center(child:ClipPath(clipper:const Triangle(),
      child:GestureDetector(behavior:HitTestBehavior.opaque,onTap:()=>taps++,
        child:const SizedBox(key:key,width:100,height:100)))));
    final p=t.getTopLeft(find.byKey(key));
    await t.tapAt(p+const Offset(20,20));
    expect(taps,1);
    await t.tapAt(p+const Offset(80,80));
    expect(taps,1);
  });
}
```

```dart
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
void main(){
  testWidgets('horizontal offset zero follows direction and reverse',(t)async{
    const viewport=ValueKey('viewport'),child=ValueKey('child');
    for(final (dir,reverse,want) in [(TextDirection.ltr,false,0.0),(TextDirection.ltr,true,-100.0),
      (TextDirection.rtl,true,0.0),(TextDirection.rtl,false,-100.0)]){
      await t.pumpWidget(Directionality(textDirection:dir,child:Center(child:SizedBox(width:100,height:60,
        child:SingleChildScrollView(key:viewport,scrollDirection:Axis.horizontal,reverse:reverse,
          child:const SizedBox(key:child,width:200,height:60))))));
      expect(t.getTopLeft(find.byKey(child)).dx-t.getTopLeft(find.byKey(viewport)).dx,want,
        reason:'$dir reverse=$reverse');
    }
  });
}
```

Temporary Rust tests in `crates/incular-widgets/tests/`:
`audit_clip_hit_tmp.rs` and `audit_scroll_rtl_tmp.rs`.
Only those targets ran; each failed as listed after positive controls.
Temporary copies were removed. Logs in `.tmp/flutter350-full-audit/`.

```rust
use incular_config::Constraints;
use incular_core::{Color,Offset,PointerPhase,Size};
use incular_gestures::PointerEvent;
use incular_rendering::Path;
use incular_widgets::{ClipPath,ClipShape,CustomClipper,GestureDetector,Widget,internal::WidgetTree};
use std::{cell::Cell,rc::Rc,sync::Arc,time::{Duration,Instant}};
fn triangle(size:Size)->Arc<Path>{
    let mut p=Path::builder();
    p.move_to(Offset::ZERO).line_to(Offset::new(size.width,0.)).line_to(Offset::new(0.,size.height)).close();
    Arc::new(p.build())
}
struct Triangle;
impl CustomClipper for Triangle {
    fn get_clip(&self,size:Size)->ClipShape {ClipShape::Path(triangle(size))}
}
#[test]
fn custom_clip_rejects_inside_box_but_outside_path(){
    let size=Size::new(100.,100.);
    let taps=Rc::new(Cell::new(0));
    let counter=taps.clone();
    let child=GestureDetector::new(Widget::box_(size,Color::WHITE)).on_tap(move||counter.set(counter.get()+1));
    let mut tree=WidgetTree::new();
    tree.mount(ClipPath::new(triangle(size),child).custom_clipper(Triangle).into()).unwrap();
    tree.layout(Constraints::tight(size)).unwrap();
    let _=tree.paint();
    let base=Instant::now();
    for (i,point) in [(0,Offset::new(20.,20.)),(1,Offset::new(80.,80.))] {
        tree.dispatch_gesture(PointerEvent{pointer:1,position:point,phase:PointerPhase::Down,time:base+Duration::from_millis(i*100)});
        tree.dispatch_gesture(PointerEvent{pointer:1,position:point,phase:PointerPhase::Up,time:base+Duration::from_millis(i*100+10)});
        assert_eq!(taps.get(),1,"tap {i}");
    }
}
```

```rust
use incular_config::{Axis,Constraints};
use incular_core::Size;
use incular_widgets::{Directionality,SingleChildScrollView,SizedBox,TextDirection,Widget,internal::{Key,WidgetTree}};
#[test]
fn horizontal_offset_zero_follows_direction_and_reverse(){
    let mut failures=Vec::new();
    for (dir,reverse,want) in [(TextDirection::Ltr,false,0.),(TextDirection::Ltr,true,-100.),
        (TextDirection::Rtl,true,0.),(TextDirection::Rtl,false,-100.)] {
        let key=Key::from(36051_u64);
        let content=Widget::from(SizedBox::new().width(200.).height(60.)).with_key(key.clone());
        let mut tree=WidgetTree::new();
        let root=Directionality::new(dir,SingleChildScrollView::new(content).scroll_direction(Axis::Horizontal).reverse(reverse));
        tree.mount(root.into()).unwrap();
        tree.layout(Constraints::tight(Size::new(100.,60.))).unwrap();
        let _=tree.paint();
        let child=tree.element_with_key(&key).unwrap();
        let actual=tree.element_bounds(child).unwrap().origin.x;
        eprintln!("{dir:?} reverse={reverse} child x={actual}");
        if actual!=want { failures.push((dir,reverse,actual,want)); }
    }
    assert!(failures.is_empty(),"{failures:?}");
}
```
