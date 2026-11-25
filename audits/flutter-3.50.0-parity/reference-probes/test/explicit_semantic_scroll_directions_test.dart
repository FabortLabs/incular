import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'Explicit Semantics dispatches four independent scroll directions',
    (WidgetTester tester) async {
      final semantics = tester.ensureSemantics();
      try {
        const key = ValueKey<String>('directional');
        final seen = <String>[];
        await tester.pumpWidget(
          Directionality(
            textDirection: TextDirection.ltr,
            child: Semantics(
              key: key,
              container: true,
              label: 'directional',
              onScrollLeft: () => seen.add('left'),
              onScrollRight: () => seen.add('right'),
              onScrollUp: () => seen.add('up'),
              onScrollDown: () => seen.add('down'),
              child: const SizedBox.square(dimension: 100),
            ),
          ),
        );
        final id = tester.getSemantics(find.byKey(key)).id;
        for (final direction in [
          SemanticsAction.scrollLeft,
          SemanticsAction.scrollRight,
          SemanticsAction.scrollUp,
          SemanticsAction.scrollDown,
        ]) {
          tester.binding.pipelineOwner.semanticsOwner!.performAction(
            id,
            direction,
          );
        }
        expect(seen, ['left', 'right', 'up', 'down']);
      } finally {
        semantics.dispose();
      }
    },
  );
}
