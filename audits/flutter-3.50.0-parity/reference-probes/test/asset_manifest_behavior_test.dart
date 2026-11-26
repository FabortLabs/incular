import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

Map<String, Object?> variant(String asset, [Object? dpr]) => {
  'asset': asset,
  if (dpr != null) 'dpr': dpr,
};

class _Bundle extends CachingAssetBundle {
  _Bundle(this.data);
  final Map<String, Object?> data;
  int reads = 0;
  Completer<ByteData>? pending;

  @override
  Future<ByteData> load(String key) async {
    expect(key, 'AssetManifest.bin');
    reads++;
    return pending == null
        ? const StandardMessageCodec().encodeMessage(data)!
        : pending!.future;
  }
}

const _fixture = <String, Object?>{
  'assets/foo.png': [
    {'asset': 'assets/foo.png'},
    {'asset': 'assets/2x/foo.png', 'dpr': 2.0},
  ],
  'assets/bar.png': [
    {'asset': 'assets/bar.png'},
  ],
};
const _hashed = <String, Object?>{
  'assets/foo.png': [
    {'asset': 'assets/foo.9f64a747.png'},
    {'asset': 'assets/2.0x/foo.74f81fe1.png', 'dpr': 2.0},
  ],
  'assets/sub dir/space image.png': [
    {'asset': 'assets/sub dir/space image.1a2b3c4d.png'},
  ],
};
final _extra = <String, Object?>{
  'web/worker.mjs.map': [variant('web/worker.0123abcd.mjs.map')],
  'web/worker.js.map': [variant('web/worker.0123abcd.js.map')],
  'web/worker.wasm.map': [variant('web/worker.0123abcd.wasm.map')],
  'web/raw': [variant('web/raw.0123abcd')],
  'web/both.bbbbbbbb': [variant('web/both.aaaaaaaa.bbbbbbbb')],
  'web/dir/file.png': [variant('web/dir.0123abcd/file.png')],
  'web/upper.png': [variant('web/upper.0123ABCD.png')],
  'web/long.png': [variant('web/long.0123abcde.png')],
  'web/worker.ts.map': [variant('web/worker.0123abcd.ts.map')],
  'empty': <Object>[],
  'null': null,
  'bad': [variant('bad', 2), variant('bad')],
  'emoji😃': [variant('emoji😃')],
};

String encoded(Map<String, Object?> manifest) {
  final ByteData bytes = const StandardMessageCodec().encodeMessage(manifest)!;
  return base64Encode(
    bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes),
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('writes exact manifest fixture bytes from the pinned Flutter codec', () {
    print('NORMAL=${encoded(_fixture)}');
    print('HASHED=${encoded(_hashed)}');
    print('EXTRA=${encoded(_extra)}');
    expect(base64.decode('8J-Ygw%3d%3D'), utf8.encode('😃'));
    expect(() => base64.decode('8J+Ygw'), throwsFormatException);
    expect(() => base64.decode('%38J+Ygw=='), throwsFormatException);
  });

  test(
    'only the permitted content hash suffixes mark variants as main',
    () async {
      final manifest = await AssetManifest.loadFromAssetBundle(_Bundle(_extra));
      for (final key in _extra.keys.take(5)) {
        expect(
          manifest.getAssetVariants(key)!.single.main,
          isTrue,
          reason: key,
        );
      }
      for (final key in _extra.keys.skip(5).take(4)) {
        expect(
          manifest.getAssetVariants(key)!.single.main,
          isFalse,
          reason: key,
        );
      }
      expect(manifest.getAssetVariants('emoji😃')!.single.main, isTrue);
    },
  );

  test('materialization is lazy and follows successful query order', () async {
    final manifest = await AssetManifest.loadFromAssetBundle(_Bundle(_extra));
    final keys = _extra.keys.toList();
    expect(manifest.listAssets(), keys);
    expect(manifest.getAssetVariants('null'), isNull);
    expect(manifest.getAssetVariants('absent'), isNull);
    expect(() => manifest.getAssetVariants('bad'), throwsA(isA<TypeError>()));
    expect(manifest.listAssets(), keys);
    final empty = manifest.getAssetVariants('empty');
    expect(empty, isEmpty);
    expect(identical(manifest.getAssetVariants('empty'), empty), isTrue);
    expect(manifest.getAssetVariants('web/raw'), hasLength(1));
    expect(manifest.listAssets(), [
      ...keys.where((key) => key != 'empty' && key != 'web/raw'),
      'empty',
      'web/raw',
    ]);
  });

  test('overlapping requests share a binary load and its manifest', () async {
    final bundle = _Bundle(_fixture)..pending = Completer<ByteData>();
    final first = AssetManifest.loadFromAssetBundle(bundle);
    final second = AssetManifest.loadFromAssetBundle(bundle);
    expect(bundle.reads, 1);
    bundle.pending!.complete(
      const StandardMessageCodec().encodeMessage(_fixture)!,
    );
    final loaded = await first;
    expect(identical(await second, loaded), isTrue);
    expect(
      identical(await AssetManifest.loadFromAssetBundle(bundle), loaded),
      isTrue,
    );
    expect(bundle.reads, 1);
  });

  test('a failed manifest is not cached', () async {
    final bundle = _Bundle(_fixture)..pending = Completer<ByteData>();
    final first = AssetManifest.loadFromAssetBundle(bundle);
    final error = expectLater(first, throwsFormatException);
    bundle.pending!.complete(ByteData.sublistView(Uint8List.fromList([255])));
    await error;
    bundle.pending = null;
    expect(
      (await AssetManifest.loadFromAssetBundle(bundle)).listAssets(),
      _fixture.keys,
    );
    expect(bundle.reads, 2);
  });
}
