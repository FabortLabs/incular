import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'barrier insets only accessibility and rebinds to the new notifier',
    (WidgetTester tester) async {
      final handle = tester.ensureSemantics();
      final original = ValueNotifier(const EdgeInsets.fromLTRB(10, 20, 30, 40));
      final replacement = ValueNotifier(const EdgeInsets.fromLTRB(8, 9, 10, 11));
      addTearDown(original.dispose);
      addTearDown(replacement.dispose);
      var taps = 0;
      Widget barrier(ValueNotifier<EdgeInsets> notifier) => Directionality(
        textDirection: TextDirection.ltr,
        child: ModalBarrier(
          semanticsLabel: 'Dismiss',
          clipDetailsNotifier: notifier,
          onDismiss: () => taps++,
        ),
      );
      Rect semanticRect() =>
          tester.getSemantics(find.bySemanticsLabel('Dismiss')).rect;
      await tester.pumpWidget(barrier(original));
      expect(semanticRect(), const Rect.fromLTRB(10, 20, 770, 560));
      expect(tester.getSize(find.byType(ModalBarrier)), const Size(800, 600));

      original.value = const EdgeInsets.fromLTRB(40, 30, 20, 10);
      await tester.pump();
      expect(semanticRect(), const Rect.fromLTRB(40, 30, 780, 590));
      expect(tester.getSize(find.byType(ModalBarrier)), const Size(800, 600));
      await tester.tapAt(const Offset(12, 24));
      expect(taps, 1, reason: 'the visual and physical veil stays full-size');

      await tester.pumpWidget(barrier(replacement));
      expect(semanticRect(), const Rect.fromLTRB(8, 9, 790, 589));
      original.value = const EdgeInsets.all(60);
      await tester.pump();
      expect(semanticRect(), const Rect.fromLTRB(8, 9, 790, 589));
      replacement.value = EdgeInsets.zero;
      await tester.pump();
      expect(semanticRect(), const Rect.fromLTRB(0, 0, 800, 600));
      handle.dispose();
    },
    variant: const TargetPlatformVariant(<TargetPlatform>{TargetPlatform.android}),
  );
}
