# Semantics blocker propagation

Pinned Flutter 3.50.0 vs Incular implementation `c647cb85`.
Headless semantic tree comparison, not a physical screen reader test.

**C23 (paired mismatch):** a Stack has labelled siblings before and after a
middle BlockSemantics. Direct blocking hides the earlier sibling on both.
Blocking inside an explicit semantics boundary does not hide the external
earlier sibling on either. Wrapping the blocker in ordinary Padding leaves
the earlier sibling **visible to accessibility only on Incular**; Flutter
propagates the blocker through the nonboundary render object. Flutter
`rendering/object.dart:5742–5768`; Incular
`crates/incular-widgets/src/tree/semantics.rs:304–316`.

## Exact upstream registrations

Copied the pinned `packages/flutter/test/widgets/semantics_9_test.dart`
and its local `semantics_tester.dart` into the fresh scratch Flutter
test directory and ran only that file (all three passed). Per-registration
dispositions for Incular, with named-label assertions and equivalent neutral
render-tree shapes:

| Registration | Result |
| --- | --- |
| line 13, hides semantic nodes of siblings | Paired match for hiding and removing the blocker. |
| line 43, does not hides semantic nodes of siblings outside the current semantic boundary | Paired mismatch: exterior siblings and the boundary remained visible and the earlier inner sibling was hidden on both. Incular omitted the deepest `#2.2.1` label; it exposed only `#1,#2,#2.2,#2.3,#3`. This is independent from the Padding leak above. |
| line 88, node is semantic boundary and blocking previously painted nodes | Flutter passed; open on Incular. Flutter uses a caller-defined RenderProxyBox setting both flags on a single node. Do not recast two wrapper nodes as the same test. |

Two exact upstream named assertions are now dispositioned; this is not a
complete native or semantic boundary sign-off. Other semantics ordering and
custom render hooks remain open.

## Focused custom comparator

Save as `test/block_semantics_slice_test.dart` in the pinned Flutter scratch
package and run `flutter test --no-pub test/block_semantics_slice_test.dart`.

```dart
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
void main() {
  testWidgets('BlockSemantics propagates through padding but stops at a semantic boundary',(t) async {
    final handle=t.ensureSemantics();
    Widget tag(String s)=>Semantics(container:true,label:s,child:const SizedBox(width:40,height:40));
    Future<void> check(Widget middle,{required bool early,bool boundary=false}) async {
      await t.pumpWidget(Directionality(textDirection:TextDirection.ltr,
        child:Stack(children:[tag('earlier'),middle,tag('later')])));
      final labels=<String>{};
      void walk(SemanticsNode node){
        labels.add(node.getSemanticsData().label);
        node.visitChildren((child){walk(child);return true;});
      }
      walk(t.binding.renderViews.single.owner!.semanticsOwner!.rootSemanticsNode!);
      expect(labels.contains('earlier'),early,reason:labels.toString());
      expect(labels,containsAll(['front','later']));
      expect(labels.contains('boundary'),boundary);
      expect(labels,isNot(contains('inside earlier')));
    }
    try {
      await check(BlockSemantics(child:tag('front')),early:false);
      await check(Semantics(container:true,explicitChildNodes:true,label:'boundary',
        child:Stack(children:[tag('inside earlier'),BlockSemantics(child:tag('front'))])),early:true,boundary:true);
      await check(Padding(padding:const EdgeInsets.all(3),child:BlockSemantics(child:tag('front'))),early:false);
    } finally {handle.dispose();}
  });
}
```

Append to a temporary copy of
`crates/incular-widgets/tests/semantics_flutter_parity.rs` as
`audit_block_semantics_tmp.rs`; run the named test with `--exact --nocapture`.

```rust
#[test]
fn block_semantics_propagates_through_padding_and_stops_at_boundary() {
    use incular_widgets::{BlockSemantics, Padding, Stack};
    fn tag(s: &str) -> Widget {
        Semantics::new(SizedBox::new().width(40.).height(40.)).container(true).label(s).into()
    }
    fn check(case: &str, middle: Widget, early: bool, boundary: bool) {
        let tree = mounted(Stack::new([tag("earlier"), middle, tag("later")]));
        eprintln!("{case}: earlier present={}",labelled(&tree,"earlier").is_some());
        assert_eq!(labelled(&tree,"earlier").is_some(),early,"{case}");
        assert!(labelled(&tree,"front").is_some());
        assert!(labelled(&tree,"later").is_some());
        assert_eq!(labelled(&tree,"boundary").is_some(),boundary);
        assert!(labelled(&tree,"inside earlier").is_none());
    }
    check("direct",BlockSemantics::new(tag("front")).into(), false,false);
    check("boundary",Semantics::new(Stack::new([tag("inside earlier"),BlockSemantics::new(tag("front")).into()]))
        .container(true).explicit_child_nodes(true).label("boundary").into(),true,true);
    check("padding",Padding::all(3.,BlockSemantics::new(tag("front"))).into(),false,false);
}
```

## Rust upstream fixture translation

Append to a separate temporary copy of the same harness as
`audit_block_upstream_tmp.rs` and run
`cargo test --locked -p incular-widgets --test audit_block_upstream_tmp upstream_block_ -- --nocapture`.
Sized boxes fill the pinned 240×160 Rust fixture, corresponding to the
non-empty sizing of the upstream Container leaves. Assertions compare the
exposed labels, not Flutter or Rust implementation-specific node IDs.
Fixtures and actual logs remain in `.tmp/flutter350-full-audit/`; temporary
Rust targets were removed.

```rust
#[test]
fn upstream_block_direct_sibling_and_removal() {
    use incular_widgets::{BlockSemantics, Stack};
    let first = || Widget::from(Semantics::new(SizedBox::new().width(240.).height(160.)).label("layer#1"));
    let blocked=mounted(Stack::new([first(),BlockSemantics::new(SizedBox::shrink()).into(),
        Semantics::new(SizedBox::new().width(240.).height(160.)).label("layer#2").into()]));
    assert!(labelled(&blocked,"layer#1").is_none());
    let unblocked=mounted(Stack::new([first()]));
    assert!(labelled(&unblocked,"layer#1").is_some());
}
#[test]
fn upstream_block_propagates_within_boundary_only() {
    use incular_widgets::{BlockSemantics, Stack};
    let tag=|s| Widget::from(Semantics::new(SizedBox::new().width(240.).height(160.)).label(s));
    let block=BlockSemantics::new(Semantics::new(SizedBox::new().width(240.).height(160.)).container(true).label("#2.2.1"));
    let wrapper=Semantics::new(block).label("#2.2");
    let boundary=Semantics::new(Stack::new([tag("NOT#2.1"),wrapper.into(),tag("#2.3")]))
        .container(true).explicit_child_nodes(true).label("#2");
    let tree=mounted(Stack::new([tag("#1"),boundary.into(),tag("#3")]));
    let labels:std::collections::BTreeSet<_>=tree.semantics().iter().filter_map(|(_,n)|n.label.as_deref()).collect();
    eprintln!("labels: {labels:?}");
    let expected=std::collections::BTreeSet::from(["#1","#2","#2.2","#2.2.1","#2.3","#3"]);
    assert_eq!(labels,expected);
}
```
