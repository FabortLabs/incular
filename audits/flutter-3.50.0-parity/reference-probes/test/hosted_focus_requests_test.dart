import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Widget scope(Widget child) =>
    Directionality(textDirection: TextDirection.ltr, child: child);

Widget bound(FocusNode node) =>
    Focus(focusNode: node, child: const SizedBox.shrink());

void main() {
  testWidgets('a hidden request cannot publish or replay focus', (tester) async {
    final a = FocusNode();
    final b = FocusNode();
    addTearDown(a.dispose);
    addTearDown(b.dispose);
    final changes = <bool>[];
    var previous = b.hasPrimaryFocus;
    b.addListener(() {
      final now = b.hasPrimaryFocus;
      if (previous != now) {
        changes.add(now);
        previous = now;
      }
    });
    Widget stack(int index) => scope(
      IndexedStack(index: index, children: [bound(a), bound(b)]),
    );

    await tester.pumpWidget(stack(0));
    a.requestFocus();
    expect(a.hasPrimaryFocus, false);
    await tester.pump();
    expect(a.hasPrimaryFocus, true);
    b.requestFocus();
    expect(b.hasPrimaryFocus, false);
    await tester.pump();
    expect((a.hasPrimaryFocus, b.hasPrimaryFocus), (true, false));
    await tester.pumpWidget(stack(1));
    expect(b.hasPrimaryFocus, false);
    expect(changes, isEmpty);
    b.requestFocus();
    await tester.pump();
    expect(b.hasPrimaryFocus, true);
    expect(changes, [true]);
  });

  testWidgets('a detached node waits to be reattached', (tester) async {
    final node = FocusNode();
    addTearDown(node.dispose);
    Widget scene(bool visible) => scope(
      Column(children: [if (visible) bound(node) else const SizedBox.shrink()]),
    );

    await tester.pumpWidget(scene(true));
    node.requestFocus();
    await tester.pump();
    expect(node.hasPrimaryFocus, true);
    await tester.pumpWidget(scene(false));
    expect(node.hasPrimaryFocus, false);
    node.requestFocus();
    expect(node.hasPrimaryFocus, false);
    await tester.pump();
    expect(node.hasPrimaryFocus, false);
    await tester.pumpWidget(scene(true));
    await tester.pump();
    expect(node.hasPrimaryFocus, true);
  });

  testWidgets('last explicit eligible request wins despite skipTraversal', (
    tester,
  ) async {
    final a = FocusNode();
    final b = FocusNode(skipTraversal: true);
    addTearDown(a.dispose);
    addTearDown(b.dispose);
    await tester.pumpWidget(scope(Column(children: [bound(a), bound(b)])));
    a.requestFocus();
    b.requestFocus();
    await tester.pump();
    expect((a.hasPrimaryFocus, b.hasPrimaryFocus), (false, true));
    await tester.pump();
    expect((a.hasPrimaryFocus, b.hasPrimaryFocus), (false, true));
  });

  testWidgets('parent focus membership survives child switches', (tester) async {
    final parent = FocusNode();
    final first = FocusNode();
    final second = FocusNode();
    final other = FocusNode();
    for (final node in [parent, first, second, other]) {
      addTearDown(node.dispose);
    }
    final trace = <bool>[];
    var previous = false;
    parent.addListener(() {
      final focused = parent.hasFocus;
      if (focused != previous) {
        trace.add(focused);
        previous = focused;
      }
    });
    await tester.pumpWidget(scope(Column(children: [
      Focus(focusNode: parent, child: Column(children: [bound(first), bound(second)])),
      bound(other),
    ])));
    parent.requestFocus();
    await tester.pump();
    expect(parent.hasPrimaryFocus, true);
    for (final child in [first, second]) {
      child.requestFocus();
      await tester.pump();
      expect(child.hasPrimaryFocus, true);
      expect((parent.hasFocus, parent.hasPrimaryFocus), (true, false));
      expect(trace, [true]);
    }
    other.requestFocus();
    await tester.pump();
    expect(other.hasPrimaryFocus, true);
    expect(parent.hasFocus, false);
    expect(trace, [true, false]);
  });
}
