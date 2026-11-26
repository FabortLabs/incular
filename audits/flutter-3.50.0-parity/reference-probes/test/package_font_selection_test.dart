import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/painting.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'A late package font is selected as both primary and fallback',
    () async {
      const package = 'audit_pack';
      const family = 'Body';
      const primary = TextStyle(
        package: package,
        fontFamily: family,
        fontSize: 23,
      );
      const fallback = TextStyle(
        package: package,
        fontFamily: 'Unregistered',
        fontFamilyFallback: ['Missing', family],
        fontSize: 23,
      );
      Size measure(TextStyle style, [String text = "iiiiiiii"]) {
        final painter = TextPainter(
          text: TextSpan(text: text, style: style),
          textDirection: TextDirection.ltr,
        )..layout();
        final size = painter.size;
        painter.dispose();
        return size;
      }

      final before = measure(primary);
      Directory directory = Directory.current;
      File? fixture;
      while (true) {
        final candidate = File(
          '${directory.path}/harness/flutter/fonts/Inter-Regular.ttf',
        );
        if (candidate.existsSync()) {
          fixture = candidate;
          break;
        }
        if (directory.parent.path == directory.path) {
          break;
        }
        directory = directory.parent;
      }
      expect(
        fixture,
        isNotNull,
        reason: 'Use Incular’s existing OFL harness font.',
      );
      final bytes = await fixture!.readAsBytes();
      final loader = FontLoader('packages/$package/$family')
        ..addFont(Future.value(ByteData.sublistView(bytes)));
      await loader.load();

      final chosen = measure(primary);
      expect(chosen, isNot(before));
      // Flutter's test font manager returns Ahem for an unknown primary
      // family. For ASCII it satisfies the request before consulting later
      // families. Use a glyph that Inter has and Ahem lacks for fallback.
      expect(measure(fallback), before);
      expect(measure(fallback, 'ΨΨΨ'), measure(primary, 'ΨΨΨ'));
    },
  );
}
