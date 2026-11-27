import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';

class _ManualCompleter extends ImageStreamCompleter {
  void emit(int count) {
    reportImageChunkEvent(
      ImageChunkEvent(cumulativeBytesLoaded: count, expectedTotalBytes: 9),
    );
  }

  void ready(ui.Image image) {
    setImage(ImageInfo(image: image));
  }

  void fail(Object error) {
    reportError(exception: error);
  }
}

void main() {
  test(
    'successful frame dispatch snapshots listeners before callbacks',
    () async {
      final recorder = ui.PictureRecorder();
      ui.Canvas(recorder)
          .drawColor(const ui.Color(0xFF112233), ui.BlendMode.src);
      final picture = recorder.endRecording();
      final image = picture.toImageSync(1, 1);
      picture.dispose();
      addTearDown(image.dispose);

      for (final removeBefore in [false, true]) {
        final source = _ManualCompleter();
        final seen = <(String, bool)>[];
        final second = ImageStreamListener((info, synchronous) {
          seen.add(('B', synchronous));
          info.dispose();
        });
        final first = ImageStreamListener((info, synchronous) {
          seen.add(('A', synchronous));
          source.removeListener(second);
          info.dispose();
        });
        source.addListener(first);
        source.addListener(second);
        if (removeBefore) {
          source.removeListener(second);
        }
        source.ready(image.clone());
        expect(seen, [('A', false), if (!removeBefore) ('B', false)]);
        source.removeListener(first);
      }
    },
  );

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
