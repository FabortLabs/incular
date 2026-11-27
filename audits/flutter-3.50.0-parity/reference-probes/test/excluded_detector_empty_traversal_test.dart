import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('a sole focused node excluded by a detector cannot advance', (
    tester,
  ) async {
    final first = FocusNode(debugLabel: 'first');
    final second = FocusNode(debugLabel: 'second');
    final detector = FocusNode(debugLabel: 'detector', skipTraversal: true);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    addTearDown(detector.dispose);
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: FocusableActionDetector(
          focusNode: detector,
          descendantsAreTraversable: false,
          child: Column(
            children: [
              Focus(
                focusNode: first,
                child: const SizedBox(width: 30, height: 30),
              ),
              Focus(
                focusNode: second,
                child: const SizedBox(width: 30, height: 30),
              ),
            ],
          ),
        ),
      ),
    );
    first.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, true);
    final moved = first.nextFocus();
    await tester.pump();
    // Use an explicit expectation here to differentiate the empty root
    // from the upstream TestWidgetsApp fixture which owns a route scope.
    expect(moved, false);
    expect(first.hasPrimaryFocus, true);
    expect(second.hasFocus, false);
  });
  testWidgets('an excluded detector yields to another target in a real route', (
    tester,
  ) async {
    final first = FocusNode(debugLabel: 'first');
    final second = FocusNode(debugLabel: 'second');
    final detector = FocusNode(debugLabel: 'detector', skipTraversal: true);
    addTearDown(first.dispose);
    addTearDown(second.dispose);
    addTearDown(detector.dispose);
    await tester.pumpWidget(
      TestWidgetsApp(
        home: FocusableActionDetector(
          focusNode: detector,
          descendantsAreTraversable: false,
          child: Column(
            children: [
              Focus(
                focusNode: first,
                child: const SizedBox(width: 30, height: 30),
              ),
              Focus(
                focusNode: second,
                child: const SizedBox(width: 30, height: 30),
              ),
            ],
          ),
        ),
      ),
    );
    first.requestFocus();
    await tester.pump();
    expect(first.hasPrimaryFocus, true);
    final scope = first.enclosingScope!;
    final moved = first.nextFocus();
    await tester.pump();
    expect(scope.traversalEdgeBehavior, TraversalEdgeBehavior.parentScope);
    expect(moved, true);
    expect(FocusManager.instance.primaryFocus, same(scope));
    expect(first.hasFocus, false);
    expect(second.hasFocus, false);
  });
}
