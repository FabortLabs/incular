import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'TextStyle uses intermediate variable axes when shaping a paragraph',
    () async {
      final fixture = Platform.environment['INCULAR_FLUTTER_VARIABLE_FONT']!;
      const family = 'ParityRobotSlabVariable';
      final bytes = await File(fixture).readAsBytes();
      final loader = FontLoader(family)
        ..addFont(Future<ByteData>.value(ByteData.sublistView(bytes)));
      await loader.load();

      TextStyle style(double weight) => TextStyle(
        fontFamily: family,
        fontSize: 48,
        fontVariations: [ui.FontVariation.weight(weight)],
      );
      double width(TextStyle style) {
        final builder =
            ui.ParagraphBuilder(
                ui.ParagraphStyle(textDirection: ui.TextDirection.ltr),
              )
              ..pushStyle(style.getTextStyle())
              ..addText('Flutter variation WWW');
        final paragraph = builder.build()
          ..layout(const ui.ParagraphConstraints(width: 1600));
        return paragraph.computeLineMetrics().single.width;
      }

      final left = style(100);
      final right = style(900);
      final samples = <(double, double)>[
        (0, 100),
        (0.25, 300),
        (0.499, 499.2),
        (0.5, 500),
        (0.75, 700),
        (1, 900),
      ];
      final widths = <double>[];
      for (final (time, weight) in samples) {
        final tween = TextStyle.lerp(left, right, time)!;
        expect(tween.fontVariations!.single.value, closeTo(weight, 1e-10));
        final actual = width(tween);
        expect(actual, closeTo(width(style(weight)), 1e-6));
        widths.add(actual);
      }
      expect(widths[0], lessThan(widths[1]));
      expect(widths[1], lessThan(widths[3]));
      expect(widths[3], lessThan(widths[4]));
      expect(widths[4], lessThan(widths[5]));
      expect(
        (widths[3] - widths[2]).abs(),
        lessThan((widths[3] - widths[1]).abs()),
      );
    },
  );

  test('axis interpolation preserves fractions and limits extrapolation', () {
    const a = ui.FontVariation('TEST', 0.0);
    const b = ui.FontVariation('TEST', 16384.0);
    expect(ui.FontVariation.lerp(a, b, -3)!.value, -32768.0);
    expect(ui.FontVariation.lerp(a, b, 3)!.value, 32768.0 - 1.0 / 65536);
    const left = TextStyle(
      fontVariations: [ui.FontVariation.weight(100.03125)],
    );
    const right = TextStyle(
      fontVariations: [ui.FontVariation.weight(300.09375)],
    );
    expect(
      TextStyle.lerp(left, right, 0.5)!.fontVariations!.single.value,
      200.0625,
    );
    expect(
      TextStyle.lerp(left, right, 2)!.fontVariations!.single.value,
      500.15625,
    );
  });
}
