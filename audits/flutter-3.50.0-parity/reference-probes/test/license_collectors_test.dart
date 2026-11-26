import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'a paused collector prevents subsequent collectors from starting early',
    () async {
      LicenseRegistry.reset();
      addTearDown(LicenseRegistry.reset);
      final gate = Completer<void>();
      final launches = <String>[];
      LicenseRegistry.addLicense(() async* {
        launches.add('first');
        await gate.future;
        yield const LicenseEntryWithLineBreaks(['demo'], 'first');
      });
      LicenseRegistry.addLicense(() async* {
        launches.add('second');
        yield const LicenseEntryWithLineBreaks(['demo'], 'second');
      });
      final stream = LicenseRegistry.licenses;
      expect(launches, isEmpty);
      final collection = stream.toList();
      await Future<void>.delayed(Duration.zero);
      expect(launches, ['first']);
      gate.complete();
      expect((await collection).map((entry) => entry.paragraphs.single.text), [
        'first',
        'second',
      ]);
      expect(launches, ['first', 'second']);
    },
  );

  test('tab indentation keeps international notices and continuation text', () {
    const notice = LicenseEntryWithLineBreaks([
      'demo',
    ], '© demo\n日本語\n\tlegal\n  continuation');
    expect(notice.packages, ['demo']);
    expect(
      notice.paragraphs.map((paragraph) => (paragraph.text, paragraph.indent)),
      [('© demo 日本語', 0), ('legal continuation', 2)],
    );
  });
}
