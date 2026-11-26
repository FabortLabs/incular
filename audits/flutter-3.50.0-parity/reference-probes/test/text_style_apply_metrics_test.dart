import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('apply multiplies each metric before adding its delta', () {
    const base = TextStyle(
      fontSize: 10,
      letterSpacing: 2,
      wordSpacing: -2,
      height: 2,
      decorationThickness: 4,
    );
    final style = base.apply(
      fontSizeFactor: -2,
      fontSizeDelta: 30,
      letterSpacingFactor: 3,
      letterSpacingDelta: -8,
      wordSpacingFactor: 2,
      wordSpacingDelta: 1,
      heightFactor: 2,
      heightDelta: 1,
      decorationThicknessFactor: 3,
      decorationThicknessDelta: -1,
    );
    expect(
      [
        style.fontSize,
        style.letterSpacing,
        style.wordSpacing,
        style.height,
        style.decorationThickness,
      ],
      [10, -2, -3, 5, 11],
    );
    expect(
      const TextStyle().apply(heightFactor: 99, heightDelta: 99).height,
      isNull,
    );
    expect(
      () => const TextStyle().apply(wordSpacingDelta: 1),
      throwsAssertionError,
    );
  });

  test('apply shifts the font weight index, including variable weights', () {
    const style = TextStyle(fontWeight: ui.FontWeight(580));
    expect(style.apply().fontWeight, ui.FontWeight.w500);
    expect(style.apply(fontWeightDelta: -2).fontWeight, ui.FontWeight.w300);
    expect(style.apply(fontWeightDelta: -99).fontWeight, ui.FontWeight.w100);
    expect(style.apply(fontWeightDelta: 99).fontWeight, ui.FontWeight.w900);
    expect(
      () => const TextStyle().apply(fontWeightDelta: 1),
      throwsAssertionError,
    );
  });

  test('explicit natural height overrides inherited heights and is not transformed', () {
    const parent = TextStyle(fontFamily: 'Ahem', fontSize: 20, height: 5);
    const natural = TextStyle(height: ui.kTextHeightNone);
    final unchanged = natural.apply(heightFactor: 1000, heightDelta: 1000);
    expect(unchanged.height, ui.kTextHeightNone);
    double advance(TextStyle style) {
      final painter = TextPainter(
        text: TextSpan(text: 'H\nH', style: style),
        textDirection: ui.TextDirection.ltr,
      )..layout();
      try {
        final lines = painter.computeLineMetrics();
        return lines[1].baseline - lines[0].baseline;
      } finally {
        painter.dispose();
      }
    }

    expect(advance(parent), 100);
    expect(
      advance(parent.merge(unchanged)),
      advance(const TextStyle(fontFamily: 'Ahem', fontSize: 20)),
    );
    expect(
      TextStyle.lerp(natural, const TextStyle(height: 2), 0.25)!.height,
      0.5,
    );
  });

  test('explicit empty shadow overrides inheritance rather than retaining the old shadow', () {
    const base = TextStyle(
      shadows: [ui.Shadow(offset: ui.Offset(4, 0), blurRadius: 2)],
    );
    expect(base.merge(const TextStyle().apply()).shadows, hasLength(1));
    expect(base.merge(const TextStyle().apply(shadows: [])).shadows, isEmpty);
    expect(
      base
          .merge(
            TextStyle.lerp(
              const TextStyle(shadows: []),
              const TextStyle(),
              0.5,
            ),
          )
          .shadows,
      isEmpty,
    );
  });
}
