import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' show AppExitResponse;

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

Future<Object?> queryExit() async {
  const JSONMethodCodec codec = JSONMethodCodec();
  final ByteData message = codec.encodeMethodCall(const MethodCall('System.requestAppExit'));
  Object? result;
  await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger.handlePlatformMessage(
    'flutter/platform',
    message,
    (ByteData? reply) => result = codec.decodeEnvelope(reply!),
  );
  return result;
}

void main() {
  testWidgets('Exit checks continue after cancellation and after a reported observer error', (
    WidgetTester tester,
  ) async {
    final calls = <String>[];
    final FlutterExceptionHandler? previous = FlutterError.onError;
    FlutterError.onError = (FlutterErrorDetails _) => calls.add('reported');
    try {
      addTearDown(AppLifecycleListener(onExitRequested: () async {
        calls.add('cancel');
        return AppExitResponse.cancel;
      }).dispose);
      addTearDown(AppLifecycleListener(onExitRequested: () async {
        calls.add('error');
        throw StateError('exit observer failed');
      }).dispose);
      addTearDown(AppLifecycleListener(onExitRequested: () async {
        calls.add('last');
        return AppExitResponse.exit;
      }).dispose);

      expect(await queryExit(), <String, Object>{'response': 'cancel'});
      expect(calls, <String>['cancel', 'error', 'reported', 'last']);
    } finally {
      FlutterError.onError = previous;
    }
  });

  testWidgets('A newly registered listener only sees a later request', (WidgetTester tester) async {
    final entered = Completer<void>();
    final leave = Completer<void>();
    var calls = 0;
    final first = AppLifecycleListener(onExitRequested: () async {
      if (!entered.isCompleted) {
        entered.complete();
      }
      await leave.future;
      return AppExitResponse.exit;
    });
    addTearDown(first.dispose);

    final Future<Object?> pending = queryExit();
    await entered.future;
    final lateListener = AppLifecycleListener(onExitRequested: () async {
      calls++;
      return AppExitResponse.exit;
    });
    addTearDown(lateListener.dispose);
    leave.complete();
    expect(await pending, <String, Object>{'response': 'exit'});
    expect(calls, 0);
    expect(await queryExit(), <String, Object>{'response': 'exit'});
    expect(calls, 1);
  });
}
