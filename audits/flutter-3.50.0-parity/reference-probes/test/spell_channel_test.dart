import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('default spell service sends BCP-47 and decodes ordered spans', (
    WidgetTester tester,
  ) async {
    final calls = <MethodCall>[];
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(SystemChannels.spellCheck, (call) async {
      calls.add(call);
      switch (calls.length) {
        case 1:
          return [
            {
              'startIndex': 3,
              'endIndex': 7,
              'suggestions': ['test', 'sets', 'set'],
            },
          ];
        case 2:
          return <Object?>[];
        default:
          throw PlatformException(code: 'busy');
      }
    });
    addTearDown(
      () => messenger.setMockMethodCallHandler(SystemChannels.spellCheck, null),
    );

    final service = DefaultSpellCheckService();
    final first = await service.fetchSpellCheckSuggestions(
      const Locale('en', 'GB'),
      '😀 tset',
    );
    expect(first, const [
      SuggestionSpan(TextRange(start: 3, end: 7), ['test', 'sets', 'set']),
    ]);
    expect(service.lastSavedResults!.spellCheckedText, '😀 tset');
    expect(calls.first.method, 'SpellCheck.initiateSpellCheck');
    expect(calls.first.arguments, ['en-GB', '😀 tset']);

    expect(
      await service.fetchSpellCheckSuggestions(
        const Locale('fr', 'CA'),
        'test',
      ),
      isEmpty,
    );
    final saved = service.lastSavedResults;
    expect(saved!.spellCheckedText, 'test');
    expect(
      await service.fetchSpellCheckSuggestions(
        const Locale('fr', 'CA'),
        'busy',
      ),
      isNull,
    );
    expect(service.lastSavedResults, same(saved));
  });
}
