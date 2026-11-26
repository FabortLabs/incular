import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

class _AlwaysDefer extends ScrollPhysics {
  const _AlwaysDefer({super.parent});

  @override
  _AlwaysDefer applyTo(ScrollPhysics? ancestor) =>
      _AlwaysDefer(parent: buildParent(ancestor));

  @override
  bool recommendDeferredLoading(
    double velocity,
    ScrollMetrics metrics,
    BuildContext context,
  ) => true;
}

class _Anchor extends StatefulWidget {
  const _Anchor({super.key});

  @override
  State<_Anchor> createState() => _AnchorState();
}

class _AnchorState extends State<_Anchor> {
  @override
  Widget build(BuildContext context) =>
      const SizedBox(height: 2000, width: 100);
}

class _Ready extends ImageProvider<_Ready> {
  _Ready(this.image);
  final ui.Image image;
  int loads = 0;

  @override
  Future<_Ready> obtainKey(ImageConfiguration configuration) =>
      SynchronousFuture(this);

  @override
  ImageStreamCompleter loadImage(_Ready key, ImageDecoderCallback decode) {
    loads++;
    return OneFrameImageStreamCompleter(
      SynchronousFuture(ImageInfo(image: image.clone())),
    );
  }
}

Future<void> main() async {
  TestWidgetsFlutterBinding.ensureInitialized();
  final raw = await createTestImage(width: 8, height: 8);
  tearDownAll(raw.dispose);

  testWidgets(
    'warm images bypass deferral and still touch the LRU after disposal',
    (tester) async {
      final key = GlobalKey<_AnchorState>();
      await tester.pumpWidget(
        Directionality(
          textDirection: TextDirection.ltr,
          child: SingleChildScrollView(
            physics: const _AlwaysDefer(),
            child: _Anchor(key: key),
          ),
        ),
      );
      final context = DisposableBuildContext(key.currentState!);
      expect(
        Scrollable.recommendDeferredLoadingForContext(key.currentContext!),
        isTrue,
      );

      final cache = PaintingBinding.instance.imageCache;
      final oldSize = cache.maximumSize;
      cache.maximumSize = 2;
      addTearDown(() {
        cache.clear();
        cache.clearLiveImages();
        cache.maximumSize = oldSize;
      });
      final first = _Ready(raw);
      final second = _Ready(raw);
      final third = _Ready(raw);
      final decoder = PaintingBinding.instance.instantiateImageCodecWithSize;
      final a = cache.putIfAbsent(first, () => first.loadImage(first, decoder));
      cache.putIfAbsent(second, () => second.loadImage(second, decoder));
      expect(cache.currentSize, 2);
      final warm = ScrollAwareImageProvider(
        context: context,
        imageProvider: first,
      );
      expect(
        identical(warm.resolve(ImageConfiguration.empty).completer, a),
        isTrue,
      );
      expect(first.loads, 1);
      context.dispose();

      final cold = ScrollAwareImageProvider(
        context: context,
        imageProvider: third,
      );
      expect(cold.resolve(ImageConfiguration.empty).completer, isNull);
      expect(third.loads, 0);
      expect(
        identical(warm.resolve(ImageConfiguration.empty).completer, a),
        isTrue,
      );
      cache.putIfAbsent(third, () => third.loadImage(third, decoder));
      expect(cache.containsKey(first), isTrue);
      expect(cache.containsKey(second), isFalse);
      expect(cache.containsKey(third), isTrue);
    },
  );
}
