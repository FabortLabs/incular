import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Sliver text actions forward exact action data to the mounted callbacks',
    (WidgetTester tester) async {
      const ValueKey<String> key = ValueKey<String>('editable annotation');
      final List<Object> calls = <Object>[];
      Widget scene(String marker, {bool active = true}) {
        return Directionality(
          textDirection: TextDirection.ltr,
          child: CustomScrollView(
            slivers: <Widget>[
              SliverSemantics(
                key: key,
                container: true,
                label: 'editable annotation',
                onSetText: active
                    ? (String text) => calls.add((marker, text))
                    : null,
                onSetSelection: active
                    ? (TextSelection selection) => calls.add((
                        marker,
                        selection.baseOffset,
                        selection.extentOffset,
                      ))
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
      owner.performAction(id, SemanticsAction.setText, 'café\nrepeat');
      owner.performAction(id, SemanticsAction.setSelection, <String, int>{
        'base': 4,
        'extent': 1,
      });
      expect(calls, <Object>[('first', 'café\nrepeat'), ('first', 4, 1)]);

      int updates = 0;
      void count() => updates++;
      owner.addListener(count);
      addTearDown(() => owner.removeListener(count));
      await tester.pumpWidget(scene('second'));
      expect(tester.getSemantics(find.byKey(key)).id, id);
      expect(updates, 0);
      owner.performAction(id, SemanticsAction.setText, '');
      owner.performAction(id, SemanticsAction.setSelection, <String, int>{
        'base': 1,
        'extent': 5,
      });
      expect(calls, <Object>[
        ('first', 'café\nrepeat'),
        ('first', 4, 1),
        ('second', ''),
        ('second', 1, 5),
      ]);

      await tester.pumpWidget(scene('removed', active: false));
      owner.performAction(id, SemanticsAction.setText, 'ignored');
      owner.performAction(id, SemanticsAction.setSelection, <String, int>{
        'base': 0,
        'extent': 0,
      });
      expect(calls.length, 4);
    },
  );
}
