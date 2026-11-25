import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('ExcludeFocusTraversal allows an explicit request after a skip', (
    WidgetTester tester,
  ) async {
    final List<FocusNode> nodes = List<FocusNode>.generate(
      4,
      (int i) => FocusNode(),
    );
    addTearDown(() {
      for (final FocusNode node in nodes) {
        node.dispose();
      }
    });

    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: FocusTraversalGroup(
          child: Column(
            children: <Widget>[
              Focus(
                autofocus: true,
                focusNode: nodes[0],
                child: const SizedBox.square(dimension: 10),
              ),
              ExcludeFocusTraversal(
                child: Focus(
                  focusNode: nodes[1],
                  child: Focus(
                    focusNode: nodes[2],
                    child: const SizedBox.square(dimension: 10),
                  ),
                ),
              ),
              Focus(
                focusNode: nodes[3],
                child: const SizedBox.square(dimension: 10),
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    List<int> primary() => <int>[
      for (var i = 0; i < nodes.length; i++)
        if (nodes[i].hasPrimaryFocus) i,
    ];
    expect(primary(), <int>[0]);

    nodes[0].nextFocus();
    await tester.pump();
    expect(primary(), <int>[3]);

    nodes[2].requestFocus();
    await tester.pump();
    expect(primary(), <int>[2]);
    expect(FocusManager.instance.primaryFocus, same(nodes[2]));
    expect(nodes[1].hasFocus, isTrue);
    expect(nodes[1].hasPrimaryFocus, isFalse);

    expect(nodes[2].nextFocus(), isTrue);
    await tester.pump();
    expect(primary(), <int>[3]);

    nodes[2].requestFocus();
    await tester.pump();
    expect(nodes[2].previousFocus(), isTrue);
    await tester.pump();
    expect(primary(), <int>[0]);
  });

  testWidgets('skipTraversal still allows a direct node request', (
    WidgetTester tester,
  ) async {
    final nodes = [FocusNode(), FocusNode(skipTraversal: true), FocusNode()];
    addTearDown(() {
      for (final node in nodes) {
        node.dispose();
      }
    });
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: FocusTraversalGroup(
          child: Column(
            children: [
              for (var i = 0; i < nodes.length; i++)
                Focus(
                  focusNode: nodes[i],
                  autofocus: i == 0,
                  child: const SizedBox.square(dimension: 10),
                ),
            ],
          ),
        ),
      ),
    );
    await tester.pump();
    expect(FocusManager.instance.primaryFocus, same(nodes[0]));
    nodes[0].nextFocus();
    await tester.pump();
    expect(FocusManager.instance.primaryFocus, same(nodes[2]));
    nodes[1].requestFocus();
    await tester.pump();
    expect(FocusManager.instance.primaryFocus, same(nodes[1]));
    nodes[1].previousFocus();
    await tester.pump();
    expect(FocusManager.instance.primaryFocus, same(nodes[0]));
  });

  testWidgets('Focus owns a stable node when none is provided', (tester) async {
    final before = FocusNode();
    final after = FocusNode();
    addTearDown(before.dispose);
    addTearDown(after.dispose);
    final identity = GlobalKey();
    Widget scene(double height) => Directionality(
      textDirection: TextDirection.ltr,
      child: FocusTraversalGroup(
        child: Column(
          children: [
            Focus(
              focusNode: before,
              autofocus: true,
              child: const SizedBox.square(dimension: 10),
            ),
            Focus(
              child: SizedBox(key: identity, width: 10, height: height),
            ),
            Focus(
              focusNode: after,
              child: const SizedBox.square(dimension: 10),
            ),
          ],
        ),
      ),
    );

    await tester.pumpWidget(scene(10));
    await tester.pump();
    expect(before.hasPrimaryFocus, isTrue);
    final internal = Focus.of(identity.currentContext!);
    expect(before.nextFocus(), isTrue);
    await tester.pump();
    expect(FocusManager.instance.primaryFocus, same(internal));
    expect(before.hasFocus || after.hasFocus, isFalse);

    await tester.pumpWidget(scene(14));
    await tester.pump();
    expect(Focus.of(identity.currentContext!), same(internal));
    expect(internal.hasPrimaryFocus, isTrue);
    expect(internal.nextFocus(), isTrue);
    await tester.pump();
    expect(after.hasPrimaryFocus, isTrue);
  });
}
