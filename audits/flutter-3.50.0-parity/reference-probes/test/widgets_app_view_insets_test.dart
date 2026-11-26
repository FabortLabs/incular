import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'live view keyboard insets reposition an open WidgetsApp tooltip',
    (WidgetTester tester) async {
      const key = ValueKey('live view bubble');
      final tooltip = GlobalKey<RawTooltipState>();
      addTearDown(tester.view.resetViewInsets);
      late double measuredInset;
      final entry = OverlayEntry(
        builder: (_) => Stack(
          children: [
            Positioned(
              left: 400,
              top: 380,
              child: RawTooltip(
                key: tooltip,
                semanticsTooltip: 'Tooltip',
                tooltipBuilder: (_, _) =>
                    const SizedBox(key: key, width: 100, height: 40),
                child: const SizedBox.shrink(),
              ),
            ),
          ],
        ),
      );
      addTearDown(
        () => entry
          ..remove()
          ..dispose(),
      );
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0),
          builder: (context, _) {
            measuredInset = MediaQuery.viewInsetsOf(context).bottom;
            return Overlay(initialEntries: [entry]);
          },
        ),
      );
      expect(measuredInset, 0);
      expect(tooltip.currentState!.ensureTooltipVisible(), isTrue);
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(find.byKey(key)), const Offset(350, 380));

      tester.view.viewInsets = FakeViewPadding(
        bottom: 200 * tester.view.devicePixelRatio,
      );
      await tester.pump();
      expect(measuredInset, 200);
      expect(tester.getTopLeft(find.byKey(key)), const Offset(350, 340));

      tester.view.resetViewInsets();
      await tester.pump();
      expect(measuredInset, 0);
      expect(tester.getTopLeft(find.byKey(key)), const Offset(350, 380));
    },
  );
}
