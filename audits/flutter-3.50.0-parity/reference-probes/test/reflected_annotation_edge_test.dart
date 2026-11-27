import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void expectSharedEdge(Layer root) {
  for (final dx in <double>[-0.125, 0, 0.125]) {
    final point = Offset(dx, 5);
    expect(root.find<int>(point), dx == 0 ? 1 : null, reason: '$point');
    expect(
      root.findAllAnnotations<int>(point).annotations.toList(),
      dx == 0 ? <int>[1] : <int>[],
      reason: '$point',
    );
  }
  expect(root.find<int>(const Offset(0, 10)), isNull);
}

void main() {
  test('a clip and a reflected sized region share a leading edge', () {
    final outer = TransformLayer(transform: Matrix4.diagonal3Values(-1, 1, 1));
    final clip = ClipRectLayer(clipRect: const Rect.fromLTWH(0, 0, 10, 10));
    final mirror = TransformLayer(transform: Matrix4.diagonal3Values(-1, 1, 1));
    mirror.append(AnnotatedRegionLayer<int>(1, size: const Size(10, 10)));
    clip.append(mirror);
    outer.append(clip);
    expectSharedEdge(outer);
  });

  test('opposing clips share a leading edge around an unsized region', () {
    final outer = TransformLayer(transform: Matrix4.diagonal3Values(-1, 1, 1));
    final outerClip = ClipRectLayer(
      clipRect: const Rect.fromLTWH(0, 0, 10, 10),
    );
    final mirror = TransformLayer(transform: Matrix4.diagonal3Values(-1, 1, 1));
    final innerClip = ClipRectLayer(
      clipRect: const Rect.fromLTWH(0, 0, 10, 10),
    );
    innerClip.append(AnnotatedRegionLayer<int>(1));
    mirror.append(innerClip);
    outerClip.append(mirror);
    outer.append(outerClip);
    expectSharedEdge(outer);
  });
}
