import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Sliver focus lifecycle is not a request for input focus', (
    WidgetTester tester,
  ) async {
    const ValueKey<String> key = ValueKey<String>('focus controls');
    final List<String> calls = <String>[];
    Widget scene(String marker, {bool active = true, bool blocked = false}) {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          slivers: <Widget>[
            SliverSemantics(
              key: key,
              container: true,
              label: 'focus controls',
              blockUserActions: blocked,
              onFocus: active ? () => calls.add('$marker request') : null,
              onDidGainAccessibilityFocus: active
                  ? () => calls.add('$marker gained')
                  : null,
              onDidLoseAccessibilityFocus: active
                  ? () => calls.add('$marker lost')
                  : null,
              sliver: const SliverToBoxAdapter(child: SizedBox(height: 20)),
            ),
          ],
        ),
      );
    }

    await tester.pumpWidget(scene('initial'));
    final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
    final int id = tester.getSemantics(find.byKey(key)).id;
    final FocusNode? inputFocus = FocusManager.instance.primaryFocus;
    void notifyLifecycle() {
      owner.performAction(id, SemanticsAction.didGainAccessibilityFocus);
      owner.performAction(id, SemanticsAction.didLoseAccessibilityFocus);
    }

    notifyLifecycle();
    owner.performAction(id, SemanticsAction.focus);
    expect(calls, <String>[
      'initial gained',
      'initial lost',
      'initial request',
    ]);
    expect(FocusManager.instance.primaryFocus, inputFocus);

    int updates = 0;
    void count() => updates++;
    owner.addListener(count);
    addTearDown(() => owner.removeListener(count));
    await tester.pumpWidget(scene('current'));
    expect(tester.getSemantics(find.byKey(key)).id, id);
    expect(updates, 0);
    calls.clear();
    notifyLifecycle();
    expect(calls, <String>['current gained', 'current lost']);

    await tester.pumpWidget(scene('blocked', blocked: true));
    final SemanticsData blocked = tester
        .getSemantics(find.byKey(key))
        .getSemanticsData();
    expect(blocked.hasAction(SemanticsAction.focus), isFalse);
    expect(
      blocked.hasAction(SemanticsAction.didGainAccessibilityFocus),
      isTrue,
    );
    expect(
      blocked.hasAction(SemanticsAction.didLoseAccessibilityFocus),
      isTrue,
    );
    calls.clear();
    notifyLifecycle();
    expect(calls, <String>['blocked gained', 'blocked lost']);
    expect(FocusManager.instance.primaryFocus, inputFocus);

    await tester.pumpWidget(scene('removed', active: false));
    calls.clear();
    notifyLifecycle();
    owner.performAction(id, SemanticsAction.focus);
    expect(calls, isEmpty);
  });
}
