import 'package:flutter/semantics.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'standard action hints describe but do not invoke an extra action',
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        final calls = [0, 0];
        for (final index in [0, 1]) {
          final isTap = index == 0;
          final hint = isTap ? 'test' : 'foo';
          final action = isTap
              ? SemanticsAction.tap
              : SemanticsAction.longPress;
          void callback() => calls[index]++;
          await tester.pumpWidget(
            Semantics(
              container: true,
              onTap: isTap ? callback : null,
              onTapHint: isTap ? hint : null,
              onLongPress: isTap ? null : callback,
              onLongPressHint: isTap ? null : hint,
            ),
          );
          final node = tester.getSemantics(find.byType(Semantics));
          final data = node.getSemanticsData();
          expect(data.hasAction(action), isTrue);
          expect(data.hasAction(SemanticsAction.customAction), isFalse);
          final overrideId = data.customSemanticsActionIds!.single;
          final descriptor = CustomSemanticsAction.getAction(overrideId)!;
          expect(descriptor.label, isNull);
          expect(descriptor.action, action);
          expect(descriptor.hint, hint);
          final owner = tester.binding.pipelineOwner.semanticsOwner!;
          owner.performAction(
            node.id,
            SemanticsAction.customAction,
            overrideId,
          );
          expect(calls[index], 0);
          owner.performAction(node.id, action);
          expect(calls[index], 1);
        }
        expect(calls, [1, 1]);
      } finally {
        semantics.dispose();
      }
    },
  );
}
