import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/painting.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

const logical = 'assets/foo.png';
const red = 'assets/foo.9f64a747.png';
const blue = 'assets/2.0x/foo.74f81fe1.png';

// Same PNG bytes as reference-probes/fixtures/hashed_asset_{1x,2x}.png.
const redPng =
    'iVBORw0KGgoAAAANSUhEUgAAAAMAAAACCAYAAACddGYaAAAAEUlEQVR4nGP4z8DwH4YZkDkAm34L9XKwuTwAAAAASUVORK5CYII=';
const bluePng =
    'iVBORw0KGgoAAAANSUhEUgAAAAYAAAAECAYAAACtBE5DAAAAEUlEQVR4nGNgYPj/HzummgQAJ7sv0bbTc20AAAAASUVORK5CYII=';

class _Bundle extends CachingAssetBundle {
  final reads = <String>[];

  @override
  Future<ByteData> load(String key) async {
    reads.add(key);
    if (key == 'AssetManifest.bin') {
      return const StandardMessageCodec().encodeMessage({
        logical: [
          {'asset': red},
          {'asset': blue, 'dpr': 2.0},
        ],
      })!;
    }
    final encoded = {red: redPng, blue: bluePng}[key];
    if (encoded == null) {
      throw StateError('Unexpected physical asset $key');
    }
    return ByteData.sublistView(base64Decode(encoded));
  }
}

Future<ImageInfo> resolve(AssetImage image, ImageConfiguration config) {
  final stream = image.resolve(config);
  final completion = Completer<ImageInfo>();
  late final ImageStreamListener listener;
  listener = ImageStreamListener((info, _) {
    stream.removeListener(listener);
    completion.complete(info);
  }, onError: completion.completeError);
  stream.addListener(listener);
  return completion.future;
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'content-hashed variants decode their own pixels at equal logical size',
    () async {
      final bundle = _Bundle();
      final asset = AssetImage(logical, bundle: bundle);
      for (final (ratio, name, pixels) in [
        (1.0, red, [255, 0, 0, 255]),
        (2.0, blue, [0, 0, 255, 255]),
      ]) {
        final config = ImageConfiguration(devicePixelRatio: ratio);
        final key = await asset.obtainKey(config);
        expect(key.name, name);
        expect(key.scale, ratio);
        final info = await resolve(asset, config);
        expect(info.scale, ratio);
        expect(info.image.width / info.scale, 3.0);
        expect(info.image.height / info.scale, 2.0);
        final ByteData? data = await info.image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        );
        expect(data!.buffer.asUint8List(data.offsetInBytes, 4), pixels);
        info.dispose();
      }
      expect(bundle.reads, ['AssetManifest.bin', red, blue]);
    },
  );
}
