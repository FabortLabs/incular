# Paired slice: declarative navigation reconciliation

Flutter 3.50.0 `ab2912035e` and Incular implementation `c647cb85`.
Neutral, transition-free route and widget cases.

| ID | Case | Flutter | Incular |
| --- | --- | --- | --- |
| C24 | Replace an unkeyed Page with a fresh compatible Page of the same type | Same route and mounted child state; counter stays 1 | Route ID changed 1→2, child element identity changed, focus went from the editor to None |
| Control | Stable page key in the same sequence | Route and child state preserved | Route, child element and editor focus all preserved |
| C25 | Push a pageless route over a stable keyed Page, then reconcile an equivalent page list | Pushed route stays current and visible | Active routes shrink from [1,2] to [1]; pushed entry is retired |

C24 has complementary child checks: Flutter's retained counter and Incular's
retained editable widget ID/focus, in addition to exactly the same route
identity contract. The editor controller is deliberately external so the Rust
test checks route-owned retention, not survival of external state. Incular's
defined policy of always remounting unkeyed pages is explicitly documented
and has a pre-existing opposed test; treat as a deliberate divergence from
Flutter until the desired API is decided, not as a random failure. C25 is
independent of unkeyed behavior: the declarative page retains its key, and
an imperative route over it is dropped.

Flutter `widgets/navigator.dart:763,4165–4253,4413–4418`; Incular
`crates/incular-navigation/src/navigator.rs:718–740,751–783,809–839`,
`tests/navigation.rs:466–495`, and outlet keys
`crates/incular-runtime/src/route_outlet.rs:1020–1047`.
Reordering, added/removed pages, explicit pop results, transition and focus
restoration interactions remain open.

## Reproduction

Use the pinned Flutter scratch package from [paired comparisons](focused-comparisons.md)
and `flutter test --no-pub test/navigation_slice_test.dart`.
All Flutter checks passed. Rust checked two narrow targets; no normal CI
files or production source were changed.

```dart
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
class ProbePage extends Page<void> {
  ProbePage(this.child,{super.key}):super(name:'editor');
  final Widget child;
  @override Route<void> createRoute(BuildContext ctx)=>PageRouteBuilder<void>(settings:this,
    transitionDuration:Duration.zero,reverseTransitionDuration:Duration.zero,pageBuilder:(_,__,___)=>child);
}
class Counter extends StatefulWidget {
  const Counter({super.key});
  @override State<Counter> createState()=>CounterState();
}
class CounterState extends State<Counter> {
  int count=0;
  void bump()=>setState(()=>count++);
  @override Widget build(BuildContext context)=>Text('count $count');
}
void main(){
  for(final keyed in [true,false]){
    testWidgets('compatible page preserves route and child state: keyed=$keyed',(t) async{
      final key=keyed?const ValueKey('editor'):null;
      final first=ProbePage(Counter(),key:key);
      final second=ProbePage(Counter(),key:key);
      Widget shell(ProbePage page)=>MediaQuery(data:MediaQueryData.fromView(t.view),
        child:Directionality(textDirection:TextDirection.ltr,child:Navigator(pages:[page],onDidRemovePage:(_){})));
      final child=find.byType(Counter);
      await t.pumpWidget(shell(first));
      final beforeState=t.state<CounterState>(child);
      final beforeRoute=ModalRoute.of<void>(t.element(child))!;
      beforeState.bump();await t.pump();
      expect(find.text('count 1'),findsOneWidget);
      expect(identical(first,second),isFalse);
      await t.pumpWidget(shell(second));
      final afterRoute=ModalRoute.of<void>(t.element(child))!;
      expect(afterRoute,same(beforeRoute));
      expect(afterRoute.settings,same(second));
      expect(t.state<CounterState>(child),same(beforeState));
      expect(find.text('count 1'),findsOneWidget);
    });
  }
  testWidgets('pageless route stays attached to a retained page',(t) async {
    final nav=GlobalKey<NavigatorState>();
    Widget shell()=>MediaQuery(data:MediaQueryData.fromView(t.view),
      child:Directionality(textDirection:TextDirection.ltr,child:Navigator(key:nav,
        pages:[ProbePage(Counter(),key:const ValueKey('editor'))],onDidRemovePage:(_){})));
    await t.pumpWidget(shell());
    final pushed=PageRouteBuilder<void>(transitionDuration:Duration.zero,
      reverseTransitionDuration:Duration.zero,pageBuilder:(_,__,___)=>const Text('imperative'));
    nav.currentState!.push<void>(pushed);
    await t.pumpAndSettle();
    expect(find.text('imperative'),findsOneWidget);
    await t.pumpWidget(shell());
    await t.pumpAndSettle();
    expect(find.text('imperative'),findsOneWidget);
    expect(pushed.isCurrent,isTrue);
  });
}
```

For the outlet probe, append to a temporary copy of
`crates/incular-runtime/tests/runtime/route_outlet.rs` named
`audit_route_outlet_tmp.rs` in the same directory. Copy
`crates/incular-runtime/tests/runtime.rs` to `audit_nav_runtime_tmp.rs`;
in its inline `mod runtime` block, add
`#[path = "audit_route_outlet_tmp.rs"]` immediately above `mod route_outlet;`.
Run `cargo test --locked -p incular-runtime --test audit_nav_runtime_tmp runtime::route_outlet::audit_compatible_page_preserves_route_and_child_focus -- --exact --nocapture`.
The keyed control passed; the unkeyed assert failed at the observed IDs.

```rust
#[test]
fn audit_compatible_page_preserves_route_and_child_focus() {
    use incular_widgets::{EditableText,TextDirection,internal::TextEditingController};
    for keyed in [true,false] {
        let nav=Navigator::new();
        let controller=TextEditingController::with_text("draft");
        let key=Key::from(7301_u64);
        let pages=|| {
            let page=Page::new("editor", Widget::from(EditableText::new(controller.clone())
                .text_direction(TextDirection::Ltr).size(Size::new(120.,32.))).with_key(key.clone()));
            [if keyed {page.key(PageKey::new("editor").unwrap())} else {page}]
        };
        nav.set_pages(pages()).unwrap();
        let mut h=harness(&nav);
        let before_route=nav.current().unwrap().id;
        let before_child=h.runtime.tree().element_with_key(&key).unwrap();
        let bounds=h.runtime.tree().element_bounds(before_child).unwrap();
        tap(&mut h,bounds.origin.x+3.,bounds.origin.y+3.);
        assert_eq!(h.runtime.focused_element(),Some(before_child),"preflight keyed={keyed}");
        nav.set_pages(pages()).unwrap();
        present(&mut h);
        let after_child=h.runtime.tree().element_with_key(&key).unwrap();
        eprintln!("keyed={keyed}, route preserved={}, child preserved={}, focus preserved={}",
          nav.current().unwrap().id==before_route,after_child==before_child,h.runtime.focused_element()==Some(before_child));
        assert_eq!((nav.current().unwrap().id,after_child,h.runtime.focused_element()),
          (before_route,before_child,Some(before_child)),"keyed={keyed}");
    }
}
```

For pageless routes use a temporary
`crates/incular-navigation/tests/audit_pageless_tmp.rs` and
`cargo test --locked -p incular-navigation --test audit_pageless_tmp`.

```rust
use incular_navigation::{Navigator,Page,PageKey,Route};
use incular_widgets::{SizedBox,Widget};
#[test]
fn pageless_route_stays_attached_to_retained_page(){
    let nav=Navigator::new();
    let leaf=||Widget::from(SizedBox::shrink());
    let pages=||[Page::new("editor",leaf()).key(PageKey::new("editor").unwrap())];
    nav.set_pages(pages()).unwrap();
    let original=nav.current().unwrap().id;
    let pushed=nav.push(Route::new("imperative",leaf()));
    assert_eq!(nav.current().unwrap().id,pushed);
    nav.set_pages(pages()).unwrap();
    let ids=nav.routes().iter().map(|route|route.id).collect::<Vec<_>>();
    assert_eq!(ids,[original,pushed]);
}
```

Temporary test targets were removed; logs are in `.tmp/flutter350-full-audit/`.
