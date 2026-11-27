import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('removing a live branch notifies the child and branch before the next scene', () {
    final ContainerLayer root = ContainerLayer();
    final ContainerLayer branch = ContainerLayer();
    final ContainerLayer sibling = ContainerLayer();
    final ContainerLayer child = ContainerLayer();
    final LayerHandle<ContainerLayer> keepBranch = LayerHandle<ContainerLayer>(
      branch,
    );
    final LayerHandle<ContainerLayer> keepRoot = LayerHandle<ContainerLayer>(
      root,
    );
    addTearDown(() {
      if (root.attached) {
        root.detach();
      }
      keepRoot.layer = null;
      keepBranch.layer = null;
    });
    root
      ..append(branch)
      ..append(sibling);
    branch.append(child);
    final List<String> seen = <String>[];
    for (final (ContainerLayer, String) entry in <(ContainerLayer, String)>[
      (root, 'root'),
      (branch, 'branch'),
      (sibling, 'sibling'),
      (child, 'child'),
    ]) {
      entry.$1.addCompositionCallback((Layer layer) {
        expect(layer, same(entry.$1));
        seen.add(entry.$2);
      });
    }
    root.attach(Object());
    root.buildScene(ui.SceneBuilder()).dispose();
    expect(seen, <String>['root', 'branch', 'child', 'sibling']);

    seen.clear();
    branch.remove();
    expect(seen, <String>['child', 'branch']);
    expect(branch.subtreeHasCompositionCallbacks, isTrue);
    seen.clear();
    root.buildScene(ui.SceneBuilder()).dispose();
    expect(seen, <String>['root', 'sibling']);

    seen.clear();
    root.append(branch);
    expect(seen, isEmpty);
    root.buildScene(ui.SceneBuilder()).dispose();
    expect(seen, <String>['root', 'sibling', 'branch', 'child']);
  });
}
