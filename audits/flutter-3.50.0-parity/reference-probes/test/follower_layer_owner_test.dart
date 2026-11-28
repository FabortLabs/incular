import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final showWhenUnlinked in [false, true]) {
    test('follower cannot import a foreign owner (showWhenUnlinked: $showWhenUnlinked)', () {
      final link = LayerLink();
      final leader = LeaderLayer(link: link, offset: const ui.Offset(31, 17));
      final follower = FollowerLayer(
        link: link,
        showWhenUnlinked: showWhenUnlinked,
        linkedOffset: const ui.Offset(10, 0),
        unlinkedOffset: const ui.Offset(80, 70),
      );
      final keepFollower = LayerHandle<FollowerLayer>(follower);
      final leaderRoot = ContainerLayer()..append(leader);
      final followerRoot = ContainerLayer()..append(follower);
      leaderRoot.attach(Object());
      followerRoot.attach(Object());
      expect(link.leader, same(leader));

      expect(
        () => follower.addToScene(ui.SceneBuilder()),
        throwsA(isA<AssertionError>().having(
          (error) => error.toString(),
          'message',
          contains('not in the same layer tree'),
        )),
      );
      expect(follower.getLastTransform(), isNull);

      follower.remove();
      leaderRoot.append(follower);
      final scene = leaderRoot.buildScene(ui.SceneBuilder());
      expect(follower.getLastTransform()?.getTranslation().x, -39);
      expect(follower.getLastTransform()?.getTranslation().y, -53);
      scene.dispose();
      leaderRoot.detach();
      followerRoot.detach();
      leaderRoot.removeAllChildren();
      keepFollower.layer = null;
    });
  }
}
