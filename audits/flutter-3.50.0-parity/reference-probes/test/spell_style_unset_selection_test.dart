import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/src/widgets/spell_check.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('A missing selection does not suppress the styled final word', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    const base = TextStyle(fontSize: 10);
    const tall = TextStyle(fontSize: 100);
    const results = SpellCheckResults(
      'TALL',
      [SuggestionSpan(TextRange(start: 0, end: 4), ['tall'])],
    );

    double height(TextSelection selection) {
      final span = buildTextSpanWithSpellCheckSuggestions(
        TextEditingValue(text: 'TALL', selection: selection),
        true,
        base,
        tall,
        results,
      );
      final painter = TextPainter(text: span, textDirection: TextDirection.ltr, maxLines: 1);
      try {
        painter.layout(maxWidth: 600);
        return painter.height;
      } finally {
        painter.dispose();
      }
    }

    expect(height(const TextSelection.collapsed(offset: 4)), 10);
    expect(height(const TextSelection.collapsed(offset: -1)), 100);
  });
}
