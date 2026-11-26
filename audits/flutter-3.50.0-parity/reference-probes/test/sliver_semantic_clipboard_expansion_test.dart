import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Sliver clipboard and expansion callbacks remain independent', (
    WidgetTester tester,
  ) async {
    const ValueKey<String> key = ValueKey<String>('commands');
    final List<String> calls = <String>[];
    Widget scene(String name, {bool active = true, bool blocked = false}) {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          slivers: <Widget>[
            SliverSemantics(
              key: key,
              container: true,
              label: 'commands',
              blockUserActions: blocked,
              onCopy: active ? () => calls.add('$name copy') : null,
              onCut: active ? () => calls.add('$name cut') : null,
              onPaste: active ? () => calls.add('$name paste') : null,
              onExpand: active ? () => calls.add('$name expand') : null,
              onCollapse: active ? () => calls.add('$name collapse') : null,
              sliver: const SliverToBoxAdapter(child: SizedBox(height: 20)),
            ),
          ],
        ),
      );
    }

    await tester.pumpWidget(scene('first'));
    final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
    final int id = tester.getSemantics(find.byKey(key)).id;
    const List<SemanticsAction> actions = <SemanticsAction>[
      SemanticsAction.copy,
      SemanticsAction.cut,
      SemanticsAction.paste,
      SemanticsAction.expand,
      SemanticsAction.collapse,
    ];
    void dispatch() {
      for (final SemanticsAction action in actions) {
        owner.performAction(id, action);
      }
    }

    dispatch();
    expect(calls, <String>[
      'first copy',
      'first cut',
      'first paste',
      'first expand',
      'first collapse',
    ]);

    int updates = 0;
    void count() => updates++;
    owner.addListener(count);
    addTearDown(() => owner.removeListener(count));
    await tester.pumpWidget(scene('second'));
    expect(tester.getSemantics(find.byKey(key)).id, id);
    expect(updates, 0);
    calls.clear();
    dispatch();
    expect(calls, <String>[
      'second copy',
      'second cut',
      'second paste',
      'second expand',
      'second collapse',
    ]);

    calls.clear();
    await tester.pumpWidget(scene('blocked', blocked: true));
    final SemanticsData blocked = tester
        .getSemantics(find.byKey(key))
        .getSemanticsData();
    for (final SemanticsAction action in actions) {
      expect(blocked.hasAction(action), isFalse);
    }
    // The Flutter owner trusts a direct request to this ID even when it
    // no longer advertises the action. Incular rejects such stale requests.
    dispatch();
    expect(calls, <String>[
      'blocked copy',
      'blocked cut',
      'blocked paste',
      'blocked expand',
      'blocked collapse',
    ]);
    calls.clear();
    await tester.pumpWidget(scene('removed', active: false));
    dispatch();
    expect(calls, isEmpty);
  });
}
