import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'a single specified metric stays constant at both ends and between them',
    () {
      const empty = TextStyle();
      const explicit = TextStyle(
        fontSize: 24,
        letterSpacing: 3,
        wordSpacing: -2,
        height: 1.6,
        decorationThickness: 2,
      );
      for (final t in [-1.0, 0.0, 0.01, 0.499, 0.5, 0.99, 1.0, 2.0]) {
        for (final style in [
          TextStyle.lerp(empty, explicit, t)!,
          TextStyle.lerp(explicit, empty, t)!,
        ]) {
          expect(style.fontSize, 24, reason: 'time $t');
          expect(style.letterSpacing, 3, reason: 'time $t');
          expect(style.wordSpacing, -2, reason: 'time $t');
          expect(style.height, 1.6, reason: 'time $t');
          expect(style.decorationThickness, 2, reason: 'time $t');
        }
      }
      expect(TextStyle.lerp(empty, empty, 0.5)!.fontSize, isNull);
      expect(TextStyle.lerp(empty, empty, 0.5)!.wordSpacing, isNull);
    },
  );

  test(
    'discrete fields inherit only on the side where they are unspecified',
    () {
      const inherited = TextStyle(
        fontFamily: 'Ambient',
        fontStyle: ui.FontStyle.italic,
      );
      const empty = TextStyle();
      const named = TextStyle(
        fontFamily: 'Named',
        fontStyle: ui.FontStyle.normal,
      );
      expect(
        inherited.merge(TextStyle.lerp(empty, named, 0.499)).fontFamily,
        'Ambient',
      );
      expect(
        inherited.merge(TextStyle.lerp(empty, named, 0.499)).fontStyle,
        ui.FontStyle.italic,
      );
      expect(
        inherited.merge(TextStyle.lerp(empty, named, 0.5)).fontFamily,
        'Named',
      );
      expect(
        inherited.merge(TextStyle.lerp(empty, named, 0.5)).fontStyle,
        ui.FontStyle.normal,
      );
      expect(
        inherited.merge(TextStyle.lerp(named, empty, 0.499)).fontFamily,
        'Named',
      );
      expect(
        inherited.merge(TextStyle.lerp(named, empty, 0.5)).fontFamily,
        'Ambient',
      );
      expect(
        inherited.merge(TextStyle.lerp(named, empty, 0.5)).fontStyle,
        ui.FontStyle.italic,
      );
    },
  );

  test(
    'a single plain text or decoration color fades rather than changing RGB',
    () {
      const empty = TextStyle();
      const painted = TextStyle(
        color: ui.Color(0xFF113355),
        decorationColor: ui.Color(0xFF224466),
      );
      for (final t in [0.0, 0.25, 0.5, 0.75, 1.0]) {
        final style = TextStyle.lerp(empty, painted, t)!;
        expect(style.color!.a, t);
        expect(style.color!.withValues(alpha: 1), painted.color);
        expect(style.decorationColor!.a, t);
        expect(
          style.decorationColor!.withValues(alpha: 1),
          painted.decorationColor,
        );
      }
    },
  );

  test(
    'two specified metrics can extrapolate before and after the transition',
    () {
      const a = TextStyle(
        fontSize: 20,
        letterSpacing: 1,
        wordSpacing: 2,
        height: 1,
        decorationThickness: 3,
      );
      const b = TextStyle(
        fontSize: 30,
        letterSpacing: 3,
        wordSpacing: 6,
        height: 2,
        decorationThickness: 5,
      );
      final left = TextStyle.lerp(a, b, -0.5)!;
      final right = TextStyle.lerp(a, b, 1.5)!;
      expect(
        [
          left.fontSize,
          left.letterSpacing,
          left.wordSpacing,
          left.height,
          left.decorationThickness,
        ],
        [15, 0, 0, 0.5, 2],
      );
      expect(
        [
          right.fontSize,
          right.letterSpacing,
          right.wordSpacing,
          right.height,
          right.decorationThickness,
        ],
        [35, 4, 8, 2.5, 6],
      );
    },
  );

  testWidgets(
    'AnimatedDefaultTextStyle uses the single font size during the tween',
    (tester) async {
      Widget scene(TextStyle style) => Directionality(
        textDirection: ui.TextDirection.ltr,
        child: AnimatedDefaultTextStyle(
          duration: const Duration(seconds: 1),
          style: style,
          child: const Text('Color A'),
        ),
      );
      TextStyle resolved() {
        final rich = tester.widget<RichText>(find.byType(RichText));
        return rich.text.style!;
      }

      const color = ui.Color(0xFF113355);
      await tester.pumpWidget(scene(const TextStyle()));
      expect(resolved().fontSize, isNull);
      await tester.pumpWidget(
        scene(const TextStyle(fontSize: 26, color: color)),
      );
      for (final alpha in [0.25, 0.5, 0.75, 1.0]) {
        await tester.pump(const Duration(milliseconds: 250));
        final style = resolved();
        expect(style.fontSize, 26);
        expect(style.color!.a, alpha);
        expect(style.color!.withValues(alpha: 1), color);
      }
    },
  );
}
