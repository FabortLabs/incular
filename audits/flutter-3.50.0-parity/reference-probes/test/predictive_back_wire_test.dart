import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _Recorder extends WidgetsBindingObserver {
  _Recorder(this.name, this.accept, this.log);

  final String name;
  final bool accept;
  final List<String> log;

  @override
  bool handleStartBackGesture(PredictiveBackEvent event) {
    expect(event.swipeEdge, SwipeEdge.right);
    expect(event.touchOffset, const Offset(0, 100));
    log.add('$name start ${event.progress}');
    return accept;
  }

  @override
  void handleUpdateBackGestureProgress(PredictiveBackEvent event) {
    log.add('$name update ${event.progress}');
  }

  @override
  void handleCancelBackGesture() {
    log.add('$name cancel');
  }
}

void main() {
  testWidgets('Standard method bytes reach only the back gesture follower', (
    tester,
  ) async {
    final log = <String>[];
    final a = _Recorder('a', true, log);
    final b = _Recorder('b', false, log);
    final binding = tester.binding;
    binding.addObserver(a);
    binding.addObserver(b);
    try {
      Map<String, Object?> map(double p) => {
        'touchOffset': [0.0, 100.0],
        'progress': p,
        'swipeEdge': 1,
      };
      Future<void> send(String name, [Object? args]) =>
          binding.defaultBinaryMessenger.handlePlatformMessage(
            'flutter/backgesture',
            const StandardMethodCodec().encodeMethodCall(
              MethodCall(name, args),
            ),
            (_) {},
          );
      await send('startBackGesture', map(0.0));
      await send('updateBackGestureProgress', map(0.5));
      await send('cancelBackGesture');
      expect(log, ['a start 0.0', 'b start 0.0', 'a update 0.5', 'a cancel']);
    } finally {
      binding.removeObserver(a);
      binding.removeObserver(b);
    }
  });

  test('Predictive event diagnostics include all three fields', () {
    final event = PredictiveBackEvent.fromMap(const {
      'touchOffset': [0.0, 100.0],
      'progress': 0.0,
      'swipeEdge': 0,
    });
    expect(
      event.toString(),
      'PredictiveBackEvent{touchOffset: Offset(0.0, 100.0), progress: 0.0, swipeEdge: SwipeEdge.left}',
    );
  });
}
