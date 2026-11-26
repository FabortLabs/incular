import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'actual failing route supplies the router, callback and transformed event',
    () {
      final router = PointerRouter();
      final transform = Matrix4.identity()..scaleByDouble(0.5, 0.5, 1, 1);
      final log = <String>[];
      final original = TestPointer(2).down(const Offset(16, 36));
      final failing = (PointerEvent event) {
        log.add('route');
        throw StateError('route failed');
      };
      router.addRoute(2, failing, transform);
      router.addGlobalRoute((_) => log.add('later'));
      FlutterErrorDetails? report;
      final previous = FlutterError.onError;
      try {
        FlutterError.onError = (details) {
          log.add('report');
          report = details;
        };
        router.route(original);
      } finally {
        FlutterError.onError = previous;
      }
      expect(log, ['route', 'report', 'later']);
      final failure = report!;
      expect(failure.exception, isA<StateError>());
      expect(failure.library, 'gesture library');
      final properties = {
        for (final node in failure.informationCollector!())
          node.name: node.value,
      };
      expect(properties['router'], same(router));
      expect(properties['route'], same(failing));
      final local = properties['event']! as PointerEvent;
      expect(local.position, const Offset(16, 36));
      expect(local.localPosition, const Offset(8, 18));
      expect(local.transform, same(transform));
    },
  );
}
