import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _Chunks extends ImageStreamCompleter {
  void emit(int count) {
    reportImageChunkEvent(
      ImageChunkEvent(cumulativeBytesLoaded: count, expectedTotalBytes: 9),
    );
  }
}

void main() {
  test(
    'chunk listeners do not replay old bytes and do preserve duplicate reports',
    () {
      final source = _Chunks();
      final keepAlive = source.keepAlive();
      final owner = ImageStream();
      final seen = <int>[];
      final listener = ImageStreamListener(
        (_, _) {},
        onChunk: (event) => seen.add(event.cumulativeBytesLoaded),
      );
      source.emit(1);
      owner.addListener(listener);
      owner.setCompleter(source);
      expect(seen, isEmpty);
      source.emit(2);
      source.emit(2);
      expect(seen, [2, 2]);
      owner.removeListener(listener);
      source.emit(3);
      expect(seen, [2, 2]);
      keepAlive.dispose();
    },
  );

  test(
    'a listener removed inside callback still gets the current snapshot only',
    () {
      final source = _Chunks();
      final seen = <String>[];
      final other = ImageStreamListener(
        (_, _) {},
        onChunk: (_) => seen.add('second'),
      );
      final first = ImageStreamListener(
        (_, _) {},
        onChunk: (_) {
          seen.add('first');
          source.removeListener(other);
        },
      );
      source.addListener(first);
      source.addListener(other);
      source.emit(1);
      source.emit(2);
      expect(seen, ['first', 'second', 'first']);
      source.removeListener(first);
    },
  );
}
