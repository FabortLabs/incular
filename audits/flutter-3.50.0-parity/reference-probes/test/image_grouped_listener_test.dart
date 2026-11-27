import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _ManualCompleter extends ImageStreamCompleter {
  void emit(int count) {
    reportImageChunkEvent(
      ImageChunkEvent(cumulativeBytesLoaded: count, expectedTotalBytes: 9),
    );
  }

  void fail(Object error) {
    reportError(exception: error);
  }
}

void main() {
  test('one listener can receive progress and handle a failed frame', () {
    final source = _ManualCompleter();
    final events = <Object>[];
    final failure = StateError('decode failed');
    final listener = ImageStreamListener(
      (_, _) => fail('no image was decoded'),
      onChunk: (event) => events.add(event.cumulativeBytesLoaded),
      onError: (error, _) => events.add(error),
    );
    source.addListener(listener);
    source.emit(4);
    source.fail(failure);
    expect(events, [4, failure]);
    source.removeListener(listener);
  });
}
