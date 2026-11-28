import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

class ProbeBundle extends CachingAssetBundle {
  ProbeBundle(this.manifest);

  final ByteData manifest;
  final keys = <String>[];
  final image = Completer<ByteData>();

  @override
  Future<ByteData> load(String key) {
    keys.add(key);
    switch (key) {
      case 'AssetManifest.bin':
        return SynchronousFuture<ByteData>(manifest);
      case '3.0x/badge.png':
        return image.future;
      default:
        return Future<ByteData>.error(StateError('Unexpected asset: $key'));
    }
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('An asset cache status query uses the selected density without loading pixels', () async {
    final manifest = const StandardMessageCodec().encodeMessage(
      <String, Object>{
        'badge.png': [
          <String, Object>{'asset': '3.0x/badge.png', 'dpr': 3.0},
        ],
      },
    )!;
    final bundle = ProbeBundle(manifest);
    final provider = AssetImage('badge.png', bundle: bundle);
    const cfg = ImageConfiguration(devicePixelRatio: 3);
    final cold = await provider.obtainCacheStatus(configuration: cfg);
    expect(cold!.untracked, isTrue);
    expect(bundle.keys, ['AssetManifest.bin']);

    final stream = provider.resolve(cfg);
    final pending = (await provider.obtainCacheStatus(configuration: cfg))!;
    expect(pending.pending, isTrue);
    expect(pending.keepAlive, isFalse);
    expect(pending.live, isTrue);
    expect(bundle.keys, ['AssetManifest.bin', '3.0x/badge.png']);

    final frame = Completer<double>();
    final listener = ImageStreamListener((info, synchronous) {
      expect(synchronous, isFalse);
      frame.complete(info.scale);
      info.dispose();
    }, onError: frame.completeError);
    stream.addListener(listener);
    addTearDown(() => stream.removeListener(listener));
    addTearDown(() => provider.evict(configuration: cfg));
    final data = base64Decode(png);
    bundle.image.complete(ByteData.sublistView(data));
    expect(await frame.future, 3);
    final warm = (await provider.obtainCacheStatus(configuration: cfg))!;
    expect(warm.pending, isFalse);
    expect(warm.keepAlive, isTrue);
    expect(warm.live, isTrue);
    final wrongScale = ExactAssetImage('3.0x/badge.png', bundle: bundle);
    expect(
      (await wrongScale.obtainCacheStatus(configuration: cfg))!.untracked,
      isTrue,
    );
    stream.removeListener(listener);
    final idle = (await provider.obtainCacheStatus(configuration: cfg))!;
    expect(idle.keepAlive, isTrue);
    expect(idle.live, isFalse);
    expect(bundle.keys, ['AssetManifest.bin', '3.0x/badge.png']);
  });

  test(
    'A malformed key manifest does not read an image or publish a frame',
    () async {
      final bundle = ProbeBundle(
        ByteData.sublistView(Uint8List.fromList([0xff, 0xff])),
      );
      final provider = AssetImage('never-load.png', bundle: bundle);
      final reports = <FlutterErrorDetails>[];
      final prior = FlutterError.onError;
      FlutterError.onError = reports.add;
      addTearDown(() => FlutterError.onError = prior);
      expect(
        await provider.obtainCacheStatus(
          configuration: ImageConfiguration.empty,
        ),
        isNull,
      );
      expect(
        reports.single.context.toString(),
        contains('checking the cache location of an image'),
      );

      final stream = provider.resolve(ImageConfiguration.empty);
      final error = Completer<Object>();
      final listener = ImageStreamListener((info, synchronous) {
        info.dispose();
        error.completeError(StateError('No asset key exists'));
      }, onError: (exception, stack) => error.complete(exception));
      stream.addListener(listener);
      addTearDown(() => stream.removeListener(listener));
      expect(await error.future, isA<FormatException>());
      expect(reports, hasLength(1));
      expect(bundle.keys, everyElement('AssetManifest.bin'));
    },
  );
}

const png =
    'iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAEklEQVR42mOQiTv4Hx9mGBkKAB3PjoF7MRzrAAAAAElFTkSuQmCC';
