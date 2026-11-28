import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('An opted-out RawScrollbar has no visible track or inner edge', (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);

    Widget build({bool? visible, Radius? trackRadius, Color? border}) {
      return Directionality(
        textDirection: TextDirection.ltr,
        child: MediaQuery(
          data: const MediaQueryData(),
          child: RawScrollbar(
            controller: controller,
            thumbVisibility: true,
            trackVisibility: visible,
            trackRadius: trackRadius,
            trackColor: const Color(0x00000000),
            trackBorderColor: border,
            child: SingleChildScrollView(
              controller: controller,
              child: const SizedBox(width: 4000, height: 4000),
            ),
          ),
        ),
      );
    }

    final scrollbar = find.byType(RawScrollbar);
    const start = Offset(794, 0);
    const end = Offset(794, 600);
    await tester.pumpWidget(build(border: const Color(0xFFFF0000)));
    await tester.pumpAndSettle();
    expect(
      scrollbar,
      paints
        ..rect(
          rect: const Rect.fromLTWH(794, 0, 6, 600),
          color: const Color(0x00000000),
        )
        ..line(
          p1: start,
          p2: end,
          strokeWidth: 1,
          color: const Color(0x00000000),
        )
        ..rect(
          rect: const Rect.fromLTWH(794, 0, 6, 90),
          color: const Color(0x66BCBCBC),
        ),
    );

    await tester.pumpWidget(
      build(visible: true, trackRadius: const Radius.circular(1)),
    );
    await tester.pumpAndSettle();
    expect(
      scrollbar,
      paints
        ..rrect(
          rrect: RRect.fromLTRBR(794, 0, 800, 600, const Radius.circular(1)),
          color: const Color(0x00000000),
        )
        ..line(
          p1: start,
          p2: end,
          strokeWidth: 1,
          color: const Color(0x1A000000),
        ),
    );
  });
}
