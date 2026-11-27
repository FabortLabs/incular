import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('moving one picture clears the old branch in the next raster', () async {
    final OffsetLayer root = OffsetLayer();
    final LayerHandle<OffsetLayer> rootOwner = LayerHandle<OffsetLayer>(root);
    final OffsetLayer old = OffsetLayer();
    final OffsetLayer destination = OffsetLayer(offset: const ui.Offset(20, 0));
    root
      ..append(old)
      ..append(destination);
    const ui.Rect marker = ui.Rect.fromLTWH(0, 0, 4, 4);
    final ui.PictureRecorder recorder = ui.PictureRecorder();
    ui.Canvas(recorder)
        .drawRect(marker, ui.Paint()..color = const ui.Color(0xFFFFFFFF));
    final PictureLayer picture = PictureLayer(marker)
      ..picture = recorder.endRecording();
    final LayerHandle<PictureLayer> owner = LayerHandle<PictureLayer>(picture);
    addTearDown(() {
      owner.layer = null;
      rootOwner.layer = null;
    });

    Future<List<int>> alphaAtBothSites() async {
      final ui.Image image = await root.toImage(
        const ui.Rect.fromLTWH(0, 0, 28, 6),
      );
      try {
        final bytes = (await image.toByteData())!;
        return <int>[
          bytes.getUint8((image.width + 1) * 4 + 3),
          bytes.getUint8((image.width + 21) * 4 + 3),
        ];
      } finally {
        image.dispose();
      }
    }

    old.append(picture);
    expect(await alphaAtBothSites(), <int>[255, 0]);
    picture.remove();
    destination.append(picture);
    expect(await alphaAtBothSites(), <int>[0, 255]);
  });
}
