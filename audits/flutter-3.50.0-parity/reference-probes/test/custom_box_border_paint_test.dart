import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

const ink = Color(0xFFD10D77);
const background = Color(0xFFFFFFFF);

class LeadingBorder extends BoxBorder {
  const LeadingBorder(this.width);

  final double width;

  @override
  BorderSide get top => BorderSide.none;

  @override
  BorderSide get bottom => BorderSide.none;

  @override
  EdgeInsetsGeometry get dimensions => EdgeInsetsDirectional.only(start: width);

  @override
  bool get isUniform => false;

  @override
  void paint(
    Canvas canvas,
    Rect rect, {
    TextDirection? textDirection,
    BoxShape shape = BoxShape.rectangle,
    BorderRadius? borderRadius,
  }) {
    expect(shape, BoxShape.rectangle);
    expect(borderRadius, isNull);
    expect(textDirection, isNotNull);
    final x = textDirection == TextDirection.rtl
        ? rect.right - width
        : rect.left;
    canvas.drawRect(
      Rect.fromLTWH(x, rect.top, width, rect.height),
      Paint()..color = ink,
    );
  }

  @override
  ShapeBorder scale(double t) => LeadingBorder(width * t);
}

void main() {
  testWidgets(
    'custom leading border controls mounted padding and emitted paint in both directions',
    (tester) async {
      const child = ValueKey('child');
      for (final direction in [
        TextDirection.ltr,
        TextDirection.rtl,
        TextDirection.ltr,
      ]) {
        await tester.pumpWidget(
          Directionality(
            textDirection: direction,
            child: Center(
              child: Container(
                decoration: const BoxDecoration(
                  color: background,
                  border: LeadingBorder(3),
                ),
                child: const SizedBox(key: child, width: 10, height: 8),
              ),
            ),
          ),
        );

        final box = find.byType(DecoratedBox);
        expect(tester.getSize(box), const Size(13, 8));
        final local =
            tester.getTopLeft(find.byKey(child)) - tester.getTopLeft(box);
        expect(
          local,
          direction == TextDirection.ltr ? const Offset(3, 0) : Offset.zero,
        );
        final x = direction == TextDirection.ltr ? 0.0 : 10.0;
        expect(
          box,
          paints
            ..rect(rect: const Rect.fromLTWH(0, 0, 13, 8), color: background)
            ..rect(rect: Rect.fromLTWH(x, 0, 3, 8), color: ink),
        );
      }
    },
  );
}
