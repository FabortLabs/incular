import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
    'clipboard presence refreshes on resume only while observed and ignores late results',
    (tester) async {
      var calls = 0;
      var notifications = 0;
      final delayed = Completer<Map<String, bool>>();
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(SystemChannels.platform, (call) async {
        if (call.method != 'Clipboard.hasStrings') {
          return null;
        }
        expect(call.arguments, 'text/plain');
        calls++;
        return calls == 1 ? <String, bool>{'value': false} : delayed.future;
      });
      addTearDown(
        () => messenger.setMockMethodCallHandler(SystemChannels.platform, null),
      );
      final notifier = ClipboardStatusNotifier();
      void listener() {
        notifications++;
      }

      expect(notifier.value, ClipboardStatus.unknown);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pump();
      expect(calls, 0);

      notifier.addListener(listener);
      await tester.pump();
      expect(calls, 1);
      expect(notifier.value, ClipboardStatus.notPasteable);
      expect(notifications, 1);

      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      await tester.pump();
      expect(calls, 1);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pump();
      expect(calls, 2);
      notifier.removeListener(listener);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pump();
      expect(calls, 2);
      notifier.dispose();
      delayed.complete(<String, bool>{'value': true});
      await tester.pump();
      expect(notifications, 1);
      expect(tester.takeException(), isNull);
    },
  );
}
