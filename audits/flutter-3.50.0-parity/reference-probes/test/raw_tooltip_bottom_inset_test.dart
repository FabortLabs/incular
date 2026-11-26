import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'an open tooltip changes sides when the keyboard occludes its lower area',
    (WidgetTester tester) async {
      final tooltip = GlobalKey<RawTooltipState>();
      const popup = ValueKey('measured tooltip');
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
                    const SizedBox(key: popup, width: 100, height: 40),
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
      final bottomInset = ValueNotifier(0.0);
      addTearDown(bottomInset.dispose);
      await tester.pumpWidget(
        WidgetsApp(
          color: const Color(0),
          builder: (context, _) => ValueListenableBuilder<double>(
            valueListenable: bottomInset,
            child: Overlay(initialEntries: [entry]),
            builder: (context, bottom, child) => MediaQuery(
              data: MediaQuery.of(context)
                  .copyWith(viewInsets: EdgeInsets.only(bottom: bottom)),
              child: child!,
            ),
          ),
        ),
      );
      expect(tooltip.currentState!.ensureTooltipVisible(), isTrue);
      await tester.pump(const Duration(milliseconds: 200));
      expect(tester.getTopLeft(find.byKey(popup)), const Offset(350, 380));

      bottomInset.value = 200;
      await tester.pump();
      expect(tester.getTopLeft(find.byKey(popup)), const Offset(350, 340));

      bottomInset.value = 0;
      await tester.pump();
      expect(tester.getTopLeft(find.byKey(popup)), const Offset(350, 380));
    },
  );

  testWidgets(
    'a singular anchor transform contributes no tooltip overlay content',
    (WidgetTester tester) async {
      final tooltip = GlobalKey<RawTooltipState>();
      const popup = ValueKey('singular tooltip');
      final scale = ValueNotifier(0.0);
      addTearDown(scale.dispose);
      final entry = OverlayEntry(
        builder: (_) => Center(
          child: ValueListenableBuilder<double>(
            valueListenable: scale,
            builder: (_, value, child) =>
                Transform.scale(scale: value, child: child),
            child: RawTooltip(
              key: tooltip,
              semanticsTooltip: 'Tooltip',
              tooltipBuilder: (_, _) =>
                  const SizedBox(key: popup, width: 100, height: 40),
              child: const SizedBox(width: 50, height: 50),
            ),
          ),
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
          builder: (_, _) => Overlay(initialEntries: [entry]),
        ),
      );
      expect(tooltip.currentState!.ensureTooltipVisible(), isTrue);
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.byKey(popup), findsNothing);

      scale.value = 1;
      await tester.pump();
      expect(find.byKey(popup), findsOneWidget);
    },
  );
}
