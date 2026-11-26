import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('Sliver cursor actions preserve direction and extension', (
    WidgetTester tester,
  ) async {
    const ValueKey<String> key = ValueKey<String>('cursor controls');
    final List<(String, bool)> calls = <(String, bool)>[];
    Widget scene(String marker, {bool active = true}) {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: CustomScrollView(
          slivers: <Widget>[
            SliverSemantics(
              key: key,
              container: true,
              label: 'cursor controls',
              onMoveCursorForwardByCharacter: active
                  ? (bool extend) => calls.add(('$marker forward', extend))
                  : null,
              onMoveCursorBackwardByCharacter: active
                  ? (bool extend) => calls.add(('$marker backward', extend))
                  : null,
              sliver: const SliverToBoxAdapter(child: SizedBox(height: 20)),
            ),
          ],
        ),
      );
    }

    await tester.pumpWidget(scene('first'));
    final SemanticsOwner owner = tester.binding.pipelineOwner.semanticsOwner!;
    final int id = tester.getSemantics(find.byKey(key)).id;
    void checkDirections(String marker) {
      calls.clear();
      for (final bool extend in <bool>[false, true]) {
        owner.performAction(
          id,
          SemanticsAction.moveCursorForwardByCharacter,
          extend,
        );
        owner.performAction(
          id,
          SemanticsAction.moveCursorBackwardByCharacter,
          extend,
        );
      }
      expect(calls, <(String, bool)>[
        ('$marker forward', false),
        ('$marker backward', false),
        ('$marker forward', true),
        ('$marker backward', true),
      ]);
    }

    final SemanticsData data = tester
        .getSemantics(find.byKey(key))
        .getSemanticsData();
    expect(
      data.hasAction(SemanticsAction.moveCursorForwardByCharacter),
      isTrue,
    );
    expect(
      data.hasAction(SemanticsAction.moveCursorBackwardByCharacter),
      isTrue,
    );
    expect(data.hasAction(SemanticsAction.moveCursorForwardByWord), isFalse);
    expect(data.hasAction(SemanticsAction.moveCursorBackwardByWord), isFalse);
    checkDirections('first');

    int updates = 0;
    void count() => updates++;
    owner.addListener(count);
    addTearDown(() => owner.removeListener(count));
    await tester.pumpWidget(scene('second'));
    expect(tester.getSemantics(find.byKey(key)).id, id);
    expect(updates, 0);
    checkDirections('second');

    calls.clear();
    await tester.pumpWidget(scene('removed', active: false));
    owner.performAction(id, SemanticsAction.moveCursorForwardByCharacter, true);
    owner.performAction(
      id,
      SemanticsAction.moveCursorBackwardByCharacter,
      false,
    );
    expect(calls, isEmpty);
  });
}
